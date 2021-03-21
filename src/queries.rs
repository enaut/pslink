use actix_identity::Identity;
use actix_web::web;
use serde::Serialize;

use super::models::{Count, Link, NewUser, User};
use crate::{
    forms::LinkForm,
    models::{NewClick, NewLink},
    ServerConfig, ServerError,
};

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
    const fn admin_or_self(&self, id: i64) -> bool {
        match self {
            Self::Admin { .. } => true,
            Self::Regular { user } => user.id == id,
            Self::NotAuthenticated | Self::Disabled => false,
        }
    }
}

/// queries the user matching the given [`actix_identity::Identity`] and determins its authentication and permission level. Returns a [`Role`] containing the user if it is authenticated.
pub(crate) async fn authenticate(
    id: &Identity,
    server_config: &ServerConfig,
) -> Result<Role, ServerError> {
    if let Some(username) = id.identity() {
        let user = User::get_user_by_name(&username, server_config).await?;

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
pub(crate) async fn list_all_allowed(
    id: &Identity,
    server_config: &ServerConfig,
) -> Result<List<FullLink>, ServerError> {
    use crate::sqlx::Row;
    match authenticate(id, server_config).await? {
        Role::Admin { user } | Role::Regular { user } => {
            let links = sqlx::query(
                "select
                        links.id as lid,
                        links.title as ltitle,
                        links.target as ltarget,
                        links.code as lcode,
                        links.author as lauthor,
                        links.created_at as ldate,
                        users.id as usid,
                        users.username as usern,
                        users.email as uemail,
                        users.role as urole,
                        count(clicks.id) as counter
                    from
                        links
                        join users on links.author = users.id
                        left join clicks on links.id = clicks.link
                    group by
                        links.id",
            )
            .fetch_all(&server_config.db_pool)
            .await?
            .into_iter()
            .map(|v| FullLink {
                link: Link {
                    id: v.get("lid"),
                    title: v.get("ltitle"),
                    target: v.get("ltarget"),
                    code: v.get("lcode"),
                    author: v.get("lauthor"),
                    created_at: v.get("ldate"),
                },
                user: User {
                    id: v.get("usid"),
                    username: v.get("usern"),
                    email: v.get("uemail"),
                    password: "invalid".to_owned(),
                    role: v.get("urole"),
                },
                clicks: Count {
                    number: v.get("counter"), /* count is never None */
                },
            });
            // show all links
            let all_links: Vec<FullLink> = links.collect();
            Ok(List {
                user,
                list: all_links,
            })
        }
        Role::Disabled | Role::NotAuthenticated => Err(ServerError::User("Not allowed".to_owned())),
    }
}

/// Only admins can list all users
pub(crate) async fn list_users(
    id: &Identity,
    server_config: &ServerConfig,
) -> Result<List<User>, ServerError> {
    match authenticate(id, server_config).await? {
        Role::Admin { user } => {
            let all_users: Vec<User> = User::get_all_users(server_config).await?;
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
pub(crate) async fn get_user(
    id: &Identity,
    user_id: &str,
    server_config: &ServerConfig,
) -> Result<Item<User>, ServerError> {
    if let Ok(uid) = user_id.parse::<i64>() {
        slog_info!(server_config.log, "Getting user {}", uid);
        let auth = authenticate(id, server_config).await?;
        slog_info!(server_config.log, "{:?}", &auth);
        if auth.admin_or_self(uid) {
            match auth {
                Role::Admin { user } | Role::Regular { user } => {
                    let viewed_user = User::get_user(uid as i64, server_config).await?;
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
pub(crate) async fn get_user_by_name(
    username: &str,
    server_config: &ServerConfig,
) -> Result<User, ServerError> {
    let user = User::get_user_by_name(username, server_config).await?;
    Ok(user)
}

pub(crate) async fn create_user(
    id: &Identity,
    data: &web::Form<NewUser>,
    server_config: &ServerConfig,
) -> Result<Item<User>, ServerError> {
    slog_info!(server_config.log, "Creating a User: {:?}", &data);
    let auth = authenticate(id, server_config).await?;
    match auth {
        Role::Admin { user } => {
            let new_user = NewUser::new(
                data.username.clone(),
                data.email.clone(),
                &data.password,
                server_config,
            )?;

            new_user.insert_user(server_config).await?;

            // querry the new user
            let new_user = get_user_by_name(&data.username, server_config).await?;
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
/// The `user_id` is never changed.
pub(crate) async fn update_user(
    id: &Identity,
    user_id: &str,
    server_config: &ServerConfig,
    data: &web::Form<NewUser>,
) -> Result<Item<User>, ServerError> {
    if let Ok(uid) = user_id.parse::<i64>() {
        let auth = authenticate(id, server_config).await?;
        let unmodified_user = User::get_user(uid, server_config).await?;
        if auth.admin_or_self(uid) {
            match auth {
                Role::Admin { .. } | Role::Regular { .. } => {
                    slog_info!(server_config.log, "Updating userinfo: ");
                    let password = if data.password.len() > 3 {
                        NewUser::hash_password(&data.password, server_config)?
                    } else {
                        unmodified_user.password
                    };
                    let new_user = User {
                        id: uid,
                        username: data.username.clone(),
                        email: data.email.clone(),
                        password,
                        role: unmodified_user.role,
                    };
                    new_user.update_user(server_config).await?;
                    let changed_user = User::get_user(uid, server_config).await?;
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

pub(crate) async fn toggle_admin(
    id: &Identity,
    user_id: &str,
    server_config: &ServerConfig,
) -> Result<Item<User>, ServerError> {
    if let Ok(uid) = user_id.parse::<i64>() {
        let auth = authenticate(id, server_config).await?;
        match auth {
            Role::Admin { .. } => {
                slog_info!(server_config.log, "Changing administrator priviledges: ");

                let unchanged_user = User::get_user(uid, server_config).await?;

                let old = unchanged_user.role;
                unchanged_user.toggle_admin(server_config).await?;

                slog_info!(server_config.log, "Toggling role: old was {}", old);

                let changed_user = User::get_user(uid, server_config).await?;
                slog_info!(
                    server_config.log,
                    "Toggled role: new is {}",
                    changed_user.role
                );
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
pub(crate) async fn get_link(
    id: &Identity,
    link_code: &str,
    server_config: &ServerConfig,
) -> Result<Item<Link>, ServerError> {
    match authenticate(id, server_config).await? {
        Role::Admin { user } | Role::Regular { user } => {
            let link = Link::get_link_by_code(link_code, server_config).await?;
            Ok(Item { user, item: link })
        }
        Role::Disabled | Role::NotAuthenticated => Err(ServerError::User("Not Allowed".to_owned())),
    }
}

/// Get link **without authentication**
pub(crate) async fn get_link_simple(
    link_code: &str,
    server_config: &ServerConfig,
) -> Result<Link, ServerError> {
    slog_info!(server_config.log, "Getting link for {:?}", link_code);

    let link = Link::get_link_by_code(link_code, server_config).await?;
    slog_info!(server_config.log, "Foun d link for {:?}", link);
    Ok(link)
}
/// Click on a link
pub(crate) async fn click_link(
    link_id: i64,
    server_config: &ServerConfig,
) -> Result<(), ServerError> {
    slog_info!(server_config.log, "Clicking on {:?}", link_id);
    let new_click = NewClick::new(link_id);
    new_click.insert_click(server_config).await?;
    Ok(())
}

/// Click on a link
pub(crate) async fn delete_link(
    id: &Identity,
    link_code: &str,
    server_config: &ServerConfig,
) -> Result<(), ServerError> {
    let auth = authenticate(id, server_config).await?;
    let link = get_link_simple(link_code, server_config).await?;
    if auth.admin_or_self(link.author) {
        Link::delete_link_by_code(link_code, server_config).await?;
        Ok(())
    } else {
        Err(ServerError::User("Permission denied!".to_owned()))
    }
}

/// Update a link if the user is admin or it is its own link.
pub(crate) async fn update_link(
    id: &Identity,
    link_code: &str,
    data: web::Form<LinkForm>,
    server_config: &ServerConfig,
) -> Result<Item<Link>, ServerError> {
    slog_info!(
        server_config.log,
        "Changing link to: {:?} {:?}",
        &data,
        &link_code
    );
    let auth = authenticate(id, server_config).await?;
    match auth {
        Role::Admin { .. } | Role::Regular { .. } => {
            let query = get_link(id, link_code, server_config).await?;
            if auth.admin_or_self(query.item.author) {
                let mut link = query.item;
                let LinkForm {
                    title,
                    target,
                    code,
                } = data.into_inner();
                link.code = code.clone();
                link.target = target;
                link.title = title;
                link.update_link(server_config).await?;
                get_link(id, &code, server_config).await
            } else {
                Err(ServerError::User("Not Allowed".to_owned()))
            }
        }
        Role::Disabled | Role::NotAuthenticated => Err(ServerError::User("Not Allowed".to_owned())),
    }
}

pub(crate) async fn create_link(
    id: &Identity,
    data: web::Form<LinkForm>,
    server_config: &ServerConfig,
) -> Result<Item<Link>, ServerError> {
    let auth = authenticate(id, server_config).await?;
    match auth {
        Role::Admin { user } | Role::Regular { user } => {
            let code = data.code.clone();
            slog_info!(server_config.log, "Creating link for: {}", &code);
            let new_link = NewLink::from_link_form(data.into_inner(), user.id);
            slog_info!(server_config.log, "Creating link for: {:?}", &new_link);

            new_link.insert(server_config).await?;
            let new_link = get_link_simple(&code, server_config).await?;
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
