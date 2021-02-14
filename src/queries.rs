use actix_identity::Identity;
use actix_web::web;
use diesel::{prelude::*, sqlite::SqliteConnection};
use dotenv::dotenv;
use serde::Serialize;

use super::models::{Count, Link, NewUser, User};
use crate::{
    forms::LinkForm,
    models::{NewClick, NewLink},
    ServerError,
};

/// Create a connection to the database
pub(super) fn establish_connection() -> Result<SqliteConnection, ServerError> {
    dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")?;

    match SqliteConnection::establish(&database_url) {
        Ok(c) => Ok(c),
        Err(e) => {
            info!("Error connecting to database: {}, {}", database_url, e);
            Err(ServerError::User(
                "Error connecting to Database".to_string(),
            ))
        }
    }
}

/// The possible roles a user could have.
#[derive(Debug, Clone)]
pub enum Role {
    NotAuthenticated,
    Disabled,
    Regular { user: User },
    Admin { user: User },
}

impl Role {
    /// Determin if the user is admin or the given user id is his own. This is used for things where users can edit or view their own entries, whereas admins can do so for all entries.
    fn admin_or_self(&self, id: i32) -> bool {
        match self {
            Self::Admin { .. } => true,
            Self::Regular { user } => user.id == id,
            Self::NotAuthenticated | Self::Disabled => false,
        }
    }
}

/// queries the user matching the given [`actix_identity::Identity`] and determins its authentication and permission level. Returns a [`Role`] containing the user if it is authenticated.
pub fn authenticate(id: Identity) -> Result<Role, ServerError> {
    if let Some(username) = id.identity() {
        use super::schema::users::dsl;
        let connection = establish_connection()?;

        let user = dsl::users
            .filter(dsl::username.eq(&username))
            .first::<User>(&connection)?;

        return Ok(match user.role {
            0 => Role::Disabled,
            1 => Role::Regular { user },
            2 => Role::Admin { user },
            _ => Role::NotAuthenticated,
        });
    }
    Ok(Role::NotAuthenticated)
}

/// A generic list returntype containing the User and a Vec containing e.g. Links or Users
pub struct List<T> {
    pub user: User,
    pub list: Vec<T>,
}

/// A link together with its author and its click-count.
#[derive(Serialize)]
pub struct FullLink {
    link: Link,
    user: User,
    clicks: Count,
}

/// Returns a List of `FullLink` meaning `Links` enriched by their author and statistics. This returns all links if the user is either Admin or Regular user.
pub(crate) fn list_all_allowed(id: Identity) -> Result<List<FullLink>, ServerError> {
    use super::schema::clicks;
    use super::schema::links;
    use super::schema::users;

    // query to select all users could be const but typespecification is too complex. A filter can be added in the match below.
    let query = links::dsl::links
        .inner_join(users::dsl::users)
        .left_join(clicks::dsl::clicks)
        .group_by(links::id)
        .select((
            (
                links::id,
                links::title,
                links::target,
                links::code,
                links::author,
                links::created_at,
            ),
            (
                users::id,
                users::username,
                users::email,
                users::password,
                users::role,
            ),
            (diesel::dsl::sql::<diesel::sql_types::Integer>(
                "COUNT(clicks.id)",
            ),),
        ));
    match authenticate(id)? {
        Role::Admin { user } | Role::Regular { user } => {
            // show all links
            let connection = establish_connection()?;
            let all_links: Vec<FullLink> = query
                .load(&connection)?
                .into_iter()
                .map(|l: (Link, User, Count)| FullLink {
                    link: l.0,
                    user: l.1,
                    clicks: l.2,
                })
                .collect();
            Ok(List {
                user,
                list: all_links,
            })
        }
        Role::Disabled | Role::NotAuthenticated => Err(ServerError::User("Not allowed".to_owned())),
    }
}

/// Only admins can list all users
pub(crate) fn list_users(id: Identity) -> Result<List<User>, ServerError> {
    use super::schema::users::dsl::users;
    match authenticate(id)? {
        Role::Admin { user } => {
            let connection = establish_connection()?;
            let all_users: Vec<User> = users.load(&connection)?;
            Ok(List {
                user,
                list: all_users,
            })
        }
        _ => Err(ServerError::User(
            "Administrator permissions required".to_owned(),
        )),
    }
}

/// A generic returntype containing the User and a single item
pub struct Item<T> {
    pub user: User,
    pub item: T,
}

/// Get a user if permissions are accordingly
pub(crate) fn get_user(id: Identity, user_id: &str) -> Result<Item<User>, ServerError> {
    use super::schema::users;
    if let Ok(uid) = user_id.parse::<i32>() {
        info!("Getting user {}", uid);
        let auth = authenticate(id)?;
        info!("{:?}", &auth);
        if auth.admin_or_self(uid) {
            match auth {
                Role::Admin { user } | Role::Regular { user } => {
                    let connection = establish_connection()?;
                    let viewed_user = users::dsl::users
                        .filter(users::dsl::id.eq(&uid))
                        .first::<User>(&connection)?;
                    Ok(Item {
                        user,
                        item: viewed_user,
                    })
                }
                Role::Disabled | Role::NotAuthenticated => {
                    unreachable!("should already be unreachable because of `admin_or_self`")
                }
            }
        } else {
            Err(ServerError::User("Permission Denied".to_owned()))
        }
    } else {
        Err(ServerError::User("Permission Denied".to_owned()))
    }
}

/// Get a user **without permission checks** (needed for login)
pub(crate) fn get_user_by_name(username: &str) -> Result<User, ServerError> {
    use super::schema::users;

    let connection = establish_connection()?;
    let user = users::dsl::users
        .filter(users::dsl::username.eq(username))
        .first::<User>(&connection)?;
    Ok(user)
}

pub(crate) fn create_user(
    id: Identity,
    data: web::Form<NewUser>,
) -> Result<Item<User>, ServerError> {
    info!("Creating a User: {:?}", &data);
    let auth = authenticate(id)?;
    match auth {
        Role::Admin { user } => {
            use super::schema::users;

            let connection = establish_connection()?;
            let new_user = NewUser::new(
                data.username.clone(),
                data.email.clone(),
                data.password.clone(),
            )?;

            diesel::insert_into(users::table)
                .values(&new_user)
                .execute(&connection)?;

            let new_user = get_user_by_name(&data.username)?;
            Ok(Item {
                user,
                item: new_user,
            })
        }
        Role::Regular { .. } | Role::Disabled | Role::NotAuthenticated => {
            Err(ServerError::User("Permission denied!".to_owned()))
        }
    }
}
/// Take a [`actix_web::web::Form<NewUser>`] and update the corresponding entry in the database.
/// The password is only updated if a new password of at least 4 characters is provided.
/// The user_id is never changed.
pub(crate) fn update_user(
    id: Identity,
    user_id: &str,
    data: web::Form<NewUser>,
) -> Result<Item<User>, ServerError> {
    if let Ok(uid) = user_id.parse::<i32>() {
        let auth = authenticate(id)?;
        if auth.admin_or_self(uid) {
            match auth {
                Role::Admin { .. } | Role::Regular { .. } => {
                    use super::schema::users::dsl::{email, id, password, username, users};

                    info!("Updating userinfo: ");
                    let connection = establish_connection()?;

                    // Update username and email - if they have not been changed their values will be replaced by the old ones.
                    diesel::update(users.filter(id.eq(&uid)))
                        .set((
                            username.eq(data.username.clone()),
                            email.eq(data.email.clone()),
                        ))
                        .execute(&connection)?;
                    // Update the password only if the user entered something.
                    if data.password.len() > 3 {
                        let hash = NewUser::hash_password(data.password.clone())?;
                        diesel::update(users.filter(id.eq(&uid)))
                            .set((password.eq(hash),))
                            .execute(&connection)?;
                    }
                    let changed_user = users.filter(id.eq(&uid)).first::<User>(&connection)?;
                    Ok(Item {
                        user: changed_user.clone(),
                        item: changed_user,
                    })
                }
                Role::NotAuthenticated | Role::Disabled => {
                    unreachable!("Should be unreachable because of the `admin_or_self`")
                }
            }
        } else {
            Err(ServerError::User("Not a valid UID".to_owned()))
        }
    } else {
        Err(ServerError::User("Permission denied".to_owned()))
    }
}

pub(crate) fn toggle_admin(id: Identity, user_id: &str) -> Result<Item<User>, ServerError> {
    if let Ok(uid) = user_id.parse::<i32>() {
        let auth = authenticate(id)?;
        match auth {
            Role::Admin { .. } => {
                use super::schema::users::dsl::{id, role, users};

                info!("Changing administrator priviledges: ");
                let connection = establish_connection()?;

                let unchanged_user = users.filter(id.eq(&uid)).first::<User>(&connection)?;

                let new_role = 2 - (unchanged_user.role + 1) % 2;
                info!(
                    "Assigning new role: {} - old was {}",
                    new_role, unchanged_user.role
                );

                // Update username and email - if they have not been changed their values will be replaced by the old ones.
                diesel::update(users.filter(id.eq(&uid)))
                    .set((role.eq(new_role),))
                    .execute(&connection)?;

                let changed_user = users.filter(id.eq(&uid)).first::<User>(&connection)?;
                Ok(Item {
                    user: changed_user.clone(),
                    item: changed_user,
                })
            }
            Role::Regular { .. } | Role::NotAuthenticated | Role::Disabled => {
                Err(ServerError::User("Permission denied".to_owned()))
            }
        }
    } else {
        Err(ServerError::User("Permission denied".to_owned()))
    }
}

/// Get one link if permissions are accordingly.
pub(crate) fn get_link(id: Identity, link_code: &str) -> Result<Item<Link>, ServerError> {
    use super::schema::links::dsl::{code, links};
    match authenticate(id)? {
        Role::Admin { user } | Role::Regular { user } => {
            let connection = establish_connection()?;
            let link: Link = links
                .filter(code.eq(&link_code))
                .first::<Link>(&connection)?;
            Ok(Item { user, item: link })
        }
        Role::Disabled | Role::NotAuthenticated => Err(ServerError::User("Not Allowed".to_owned())),
    }
}

/// Get link **without authentication**
pub(crate) fn get_link_simple(link_code: &str) -> Result<Link, ServerError> {
    use super::schema::links::dsl::{code, links};
    info!("Getting link for {:?}", link_code);
    let connection = establish_connection()?;
    let link: Link = links
        .filter(code.eq(&link_code))
        .first::<Link>(&connection)?;
    Ok(link)
}
/// Click on a link
pub(crate) fn click_link(link_id: i32) -> Result<(), ServerError> {
    use super::schema::clicks;
    let new_click = NewClick::new(link_id);
    let connection = establish_connection()?;

    diesel::insert_into(clicks::table)
        .values(&new_click)
        .execute(&connection)?;
    Ok(())
}

/// Click on a link
pub(crate) fn delete_link(id: Identity, link_code: String) -> Result<(), ServerError> {
    use super::schema::links::dsl::{code, links};
    let connection = establish_connection()?;
    let auth = authenticate(id)?;
    let link = get_link_simple(&link_code)?;
    if auth.admin_or_self(link.author) {
        diesel::delete(links.filter(code.eq(&link_code))).execute(&connection)?;
        Ok(())
    } else {
        Err(ServerError::User("Permission denied!".to_owned()))
    }
}
/// Update a link if the user is admin or it is its own link.
pub(crate) fn update_link(
    id: Identity,
    link_code: &str,
    data: web::Form<LinkForm>,
) -> Result<Item<Link>, ServerError> {
    use super::schema::links::dsl::{code, links, target, title};
    info!("Changing link to: {:?} {:?}", &data, &link_code);
    let auth = authenticate(id.clone())?;
    match auth.clone() {
        Role::Admin { .. } | Role::Regular { .. } => {
            let query = get_link(id.clone(), &link_code)?;
            if auth.admin_or_self(query.item.author) {
                let connection = establish_connection()?;
                diesel::update(links.filter(code.eq(&query.item.code)))
                    .set((
                        code.eq(&data.code),
                        target.eq(&data.target),
                        title.eq(&data.title),
                    ))
                    .execute(&connection)?;
                get_link(id, &data.code)
            } else {
                Err(ServerError::User("Not Allowed".to_owned()))
            }
        }
        Role::Disabled | Role::NotAuthenticated => Err(ServerError::User("Not Allowed".to_owned())),
    }
}

pub(crate) fn create_link(
    id: Identity,
    data: web::Form<LinkForm>,
) -> Result<Item<Link>, ServerError> {
    let auth = authenticate(id)?;
    match auth {
        Role::Admin { user } | Role::Regular { user } => {
            use super::schema::links;

            let connection = establish_connection()?;
            let new_link = NewLink::from_link_form(data.into_inner(), user.id);

            diesel::insert_into(links::table)
                .values(&new_link)
                .execute(&connection)?;
            let new_link = get_link_simple(&new_link.code)?;
            Ok(Item {
                user,
                item: new_link,
            })
        }
        Role::Disabled | Role::NotAuthenticated => {
            Err(ServerError::User("Permission denied!".to_owned()))
        }
    }
}
