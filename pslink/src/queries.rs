use std::str::FromStr;

use actix_identity::Identity;
use enum_map::EnumMap;
use pslink_shared::{
    apirequests::{
        general::{EditMode, Filter, Operation, Ordering},
        links::{LinkDelta, LinkOverviewColumns, LinkRequestForm},
        users::{Role, UserDelta, UserOverviewColumns, UserRequestForm},
    },
    datatypes::{Clicks, Count, FullLink, Lang, Link, Secret, Statistics, User},
};
use serde::Serialize;
use sqlx::Row;
use tracing::{info, instrument, warn};

use super::models::NewUser;
use crate::{
    models::{LinkDbOperations, NewClick, NewLink, UserDbOperations},
    ServerConfig, ServerError,
};

/// This type is used to guard the Roles. The typesystem enforces that the user can only be extracted if permissions are considered.
#[derive(Debug, Clone)]
pub enum RoleGuard {
    NotAuthenticated,
    Disabled,
    Regular { user: User },
    Admin { user: User },
}

impl RoleGuard {
    fn create(user: &User) -> Self {
        match user.role {
            Role::NotAuthenticated => Self::NotAuthenticated,
            Role::Disabled => Self::Disabled,
            Role::Regular => Self::Regular { user: user.clone() },
            Role::Admin => Self::Admin { user: user.clone() },
        }
    }
    /// Determin if the user is admin or the given user id is his own. This is used for things where users can edit or view their own entries, whereas admins can do so for all entries.
    const fn admin_or_self(&self, id: i64) -> bool {
        match self {
            Self::Admin { .. } => true,
            Self::Regular { user } => user.id == id,
            Self::NotAuthenticated | Self::Disabled => false,
        }
    }
}

/// queries the user matching the given [`actix_identity::Identity`] and determins its authentication and permission level. Returns a [`RoleGuard`] containing the user if it is authenticated.
///
/// # Errors
/// Fails only if there are issues using the database.
#[instrument(skip(id))]
pub async fn authenticate(
    id: &Identity,
    server_config: &ServerConfig,
) -> Result<RoleGuard, ServerError> {
    if let Ok(username) = id.id() {
        info!("Looking for user {}", username);
        let user = User::get_user_by_name(&username, server_config).await?;
        info!("Found user {:?}", user);

        return Ok(RoleGuard::create(&user));
    }
    Ok(RoleGuard::NotAuthenticated)
}

/// A generic list returntype containing the User and a Vec containing e.g. Links or Users
#[derive(Serialize)]
pub struct ListWithOwner<T> {
    pub user: User,
    pub list: Vec<T>,
}

/// Returns a List of `FullLink` meaning `Links` enriched by their author and statistics. This returns all links if the user is either Admin or Regular user.
///
/// Todo: this function only naively protects agains SQL-injections use better variants.
///
/// # Errors
/// Fails with [`ServerError`] if access to the database fails.
#[instrument(skip(id))]
pub async fn list_all_allowed(
    id: &Identity,
    server_config: &ServerConfig,
    parameters: LinkRequestForm,
) -> Result<ListWithOwner<FullLink>, ServerError> {
    use sqlx::Row;
    match authenticate(id, server_config).await? {
        RoleGuard::Admin { user } | RoleGuard::Regular { user } => {
            let mut querystring = "select
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
                        users.language as ulang,
                        count(clicks.id) as counter
                    from
                        links
                        join users on links.author = users.id
                        left join clicks on links.id = clicks.link"
                .to_string();
            querystring.push_str(&generate_filter_sql(&parameters.filter));
            querystring.push_str("\n GROUP BY links.id");
            if let Some(order) = parameters.order {
                querystring.push_str(&generate_order_sql(&order));
            }
            querystring.push_str(&format!("\n LIMIT {}", parameters.amount));
            querystring.push_str(&format!("\n OFFSET {}", parameters.offset));
            info!("{}", querystring);

            let links = sqlx::query(&querystring)
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
                        password: Secret::new("invalid".to_string()),
                        role: Role::convert(v.get("urole")),
                        language: Lang::from_str(v.get("ulang")).expect("Should parse"),
                    },
                    clicks: Clicks::Count(Count {
                        number: v.get("counter"), /* count is never None */
                    }),
                });
            // show all links
            let all_links: Vec<FullLink> = links.collect();
            Ok(ListWithOwner {
                user,
                list: all_links,
            })
        }
        RoleGuard::Disabled | RoleGuard::NotAuthenticated => {
            Err(ServerError::User("Not allowed".to_owned()))
        }
    }
}

/// Generate a filter statement for the SQL-Query according to the parameters...
///
/// Todo: this function only naively protects agains SQL-injections use better variants.
fn generate_filter_sql(filters: &EnumMap<LinkOverviewColumns, Filter>) -> String {
    let mut result = String::new();
    let filterstring = filters
        .iter()
        .filter_map(|(column, sieve)| {
            // avoid sql injections
            let sieve: String = sieve.chars().filter(|x| x.is_alphanumeric()).collect();
            if sieve.is_empty() {
                None
            } else {
                Some(match column {
                    LinkOverviewColumns::Code => {
                        format!("\n lcode LIKE '%{}%'", sieve)
                    }
                    LinkOverviewColumns::Description => {
                        format!("\n ltitle LIKE '%{}%'", sieve)
                    }
                    LinkOverviewColumns::Target => {
                        format!("\n ltarget LIKE '%{}%'", sieve)
                    }
                    LinkOverviewColumns::Author => {
                        format!("\n usern LIKE '%{}%'", sieve)
                    }
                    LinkOverviewColumns::Statistics => {
                        format!("\n counter LIKE '%{}%'", sieve)
                    }
                })
            }
        })
        .collect::<Vec<String>>()
        .join(" AND ");
    if filterstring.len() > 1 {
        result.push_str("\n WHERE ");
        result.push_str(&filterstring);
    }
    result
}

/// A macro to translate the Ordering Type into a sql ordering string.
macro_rules! ts {
    ($ordering:expr) => {
        match $ordering {
            Ordering::Ascending => "ASC",
            Ordering::Descending => "DESC",
        }
    };
}

/// Generate a order statement for the SQL-Query according to the parameters...
fn generate_order_sql(order: &Operation<LinkOverviewColumns, Ordering>) -> String {
    match order.column {
        LinkOverviewColumns::Code => {
            format!("\n ORDER BY lcode {}", ts!(order.value))
        }
        LinkOverviewColumns::Description => {
            format!("\n ORDER BY ltitle {}", ts!(order.value))
        }
        LinkOverviewColumns::Target => {
            format!("\n ORDER BY ltarget {}", ts!(order.value))
        }
        LinkOverviewColumns::Author => {
            format!("\n ORDER BY usern {}", ts!(order.value))
        }
        LinkOverviewColumns::Statistics => {
            format!("\n ORDER BY counter {}", ts!(order.value))
        }
    }
}

/// Only admins can list all users other users will only see themselves.
///
/// # Errors
/// Fails with [`ServerError`] if access to the database fails.
#[instrument(skip(id))]
pub async fn list_users(
    id: &Identity,
    server_config: &ServerConfig,
    parameters: UserRequestForm,
) -> Result<ListWithOwner<User>, ServerError> {
    let auth = authenticate(id, server_config).await?;
    info!("Auth: {:?}", auth);
    match auth {
        RoleGuard::Admin { user } => {
            info!("Admin User {:?}", user.username);
            let mut querystring = "Select * from users".to_string();
            querystring.push_str(&generate_filter_users_sql(&parameters.filter));
            if let Some(order) = parameters.order {
                querystring.push_str(&generate_order_users_sql(&order));
            }
            querystring.push_str(&format!("\n LIMIT {}", parameters.amount));
            info!("{}", querystring);

            let query_result = sqlx::query(&querystring)
                .fetch_all(&server_config.db_pool)
                .await;
            if let Err(e) = &query_result {
                info!("Query: {:?}", e);
            }
            let users: Vec<User> = query_result?
                .into_iter()
                .map(|v| User {
                    id: v.get("id"),
                    username: v.get("username"),
                    email: v.get("email"),
                    password: Secret::new("".to_string()),
                    role: Role::convert(v.get("role")),
                    language: Lang::from_str(v.get("language")).expect("Should parse"),
                })
                .collect();

            info!("Found {} users", users.len());
            info!("Found {:?} users", users);

            Ok(ListWithOwner { user, list: users })
        }
        RoleGuard::Regular { user } => {
            info!("Regular User {:?} users", user);
            Ok(ListWithOwner {
                user: user.clone(),
                list: vec![user],
            })
        }
        _ => Err(ServerError::User(
            "Administrator permissions required".to_owned(),
        )),
    }
}

/// Generate a filter statement for the SQL-Query according to the parameters...
///
/// Todo: this function only naively protects agains SQL-injections use better variants.
fn generate_filter_users_sql(filters: &EnumMap<UserOverviewColumns, Filter>) -> String {
    let mut result = String::new();
    let filterstring = filters
        .iter()
        .filter_map(|(column, sieve)| {
            // avoid sql injections
            let sieve: String = sieve.chars().filter(|x| x.is_alphanumeric()).collect();
            if sieve.is_empty() {
                None
            } else {
                Some(match column {
                    UserOverviewColumns::Id => {
                        format!("\n id LIKE '%{}%'", sieve)
                    }
                    UserOverviewColumns::Username => {
                        format!("\n username LIKE '%{}%'", sieve)
                    }
                    UserOverviewColumns::Email => {
                        format!("\n email LIKE '%{}%'", sieve)
                    }
                })
            }
        })
        .collect::<Vec<String>>()
        .join(" AND ");
    if filterstring.len() > 1 {
        result.push_str("\n WHERE ");
        result.push_str(&filterstring);
    }
    result
}

/// Generate a order statement for the SQL-Query according to the parameters...
fn generate_order_users_sql(order: &Operation<UserOverviewColumns, Ordering>) -> String {
    match order.column {
        UserOverviewColumns::Id => {
            format!("\n ORDER BY id {}", ts!(order.value))
        }
        UserOverviewColumns::Username => {
            format!("\n ORDER BY username {}", ts!(order.value))
        }
        UserOverviewColumns::Email => {
            format!("\n ORDER BY email {}", ts!(order.value))
        }
    }
}

/// A generic returntype containing the User and a single item
pub struct Item<T> {
    pub user: User,
    pub item: T,
}

/// Get a user if permissions are accordingly
///
/// # Errors
/// Fails with [`ServerError`] if access to the database fails or this user does not have permissions.
#[allow(clippy::missing_panics_doc)]
#[instrument(skip(id))]
pub async fn get_user(
    id: &Identity,
    user_id: &str,
    server_config: &ServerConfig,
) -> Result<Item<User>, ServerError> {
    if let Ok(uid) = user_id.parse::<i64>() {
        info!("Getting user {}", uid);
        let auth = authenticate(id, server_config).await?;
        if auth.admin_or_self(uid) {
            match auth {
                RoleGuard::Admin { user } | RoleGuard::Regular { user } => {
                    let viewed_user = User::get_user(uid as i64, server_config).await?;
                    Ok(Item {
                        user,
                        item: viewed_user,
                    })
                }
                RoleGuard::Disabled | RoleGuard::NotAuthenticated => {
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
///
/// # Errors
/// Fails with [`ServerError`] if access to the database fails.
#[instrument()]
pub async fn get_user_by_name(
    username: &str,
    server_config: &ServerConfig,
) -> Result<User, ServerError> {
    let user = User::get_user_by_name(username, server_config).await?;
    Ok(user)
}

/// Create a new user and save it to the database
///
/// # Errors
/// Fails with [`ServerError`] if access to the database fails, this user does not have permissions or the user already exists.
#[instrument(skip(id))]
pub async fn create_user(
    id: &Identity,
    data: UserDelta,
    server_config: &ServerConfig,
) -> Result<Item<User>, ServerError> {
    info!("Creating a User: {:?}", &data);
    if data.edit != EditMode::Create {
        return Err(ServerError::User("Wrong Request".to_string()));
    }
    let auth = authenticate(id, server_config).await?;

    // Require a password on user creation!
    let password = match &data.password {
        Some(pass) => pass,
        None => {
            return Err(ServerError::User(
                "A new users does require a password".to_string(),
            ))
        }
    };
    match auth {
        RoleGuard::Admin { user } => {
            let new_user = NewUser::new(
                data.username.clone(),
                data.email.clone(),
                password,
                &server_config.secret,
            )?;

            new_user.insert_user(server_config).await?;

            // querry the new user
            let new_user = get_user_by_name(&data.username, server_config).await?;
            Ok(Item {
                user,
                item: new_user,
            })
        }
        RoleGuard::Regular { .. } | RoleGuard::Disabled | RoleGuard::NotAuthenticated => {
            Err(ServerError::User("Permission denied!".to_owned()))
        }
    }
}
/// Create a new user and save it to the database
///
/// # Errors
/// Fails with [`ServerError`] if access to the database fails, this user does not have permissions or the user already exists.
#[instrument(skip(id))]
pub async fn create_user_json(
    id: &Identity,
    data: &actix_web::web::Json<UserDelta>,
    server_config: &ServerConfig,
) -> Result<Item<User>, ServerError> {
    info!("Creating a User: {:?}", &data);
    if data.edit != EditMode::Create {
        return Err(ServerError::User("Wrong Request".to_string()));
    }
    let auth = authenticate(id, server_config).await?;

    // Require a password on user creation!
    let password = match &data.password {
        Some(pass) => pass,
        None => {
            return Err(ServerError::User(
                "A new users does require a password".to_string(),
            ))
        }
    };
    match auth {
        RoleGuard::Admin { user } => {
            let new_user = NewUser::new(
                data.username.clone(),
                data.email.clone(),
                password,
                &server_config.secret,
            )?;

            new_user.insert_user(server_config).await?;

            // querry the new user
            let new_user = get_user_by_name(&data.username, server_config).await?;
            Ok(Item {
                user,
                item: new_user,
            })
        }
        RoleGuard::Regular { .. } | RoleGuard::Disabled | RoleGuard::NotAuthenticated => {
            Err(ServerError::User("Permission denied!".to_owned()))
        }
    }
}

/// Take a [`actix_web::web::Form<NewUser>`] and update the corresponding entry in the database.
/// The password is only updated if a new password of at least 4 characters is provided.
/// The `user_id` is never changed.
///
/// # Errors
/// Fails with [`ServerError`] if access to the database fails, this user does not have permissions, or the given data is malformed.

#[instrument(skip(id))]
pub async fn update_user(
    id: &Identity,
    data: &actix_web::web::Json<UserDelta>,
    server_config: &ServerConfig,
) -> Result<Item<User>, ServerError> {
    let auth = authenticate(id, server_config).await?;
    if let Some(uid) = data.id {
        let unmodified_user = User::get_user(uid, server_config).await?;
        if auth.admin_or_self(uid) {
            match auth {
                RoleGuard::Admin { .. } | RoleGuard::Regular { .. } => {
                    info!("Updating userinfo: ");
                    let password = match &data.password {
                        Some(password) if password.len() > 4 => {
                            Secret::new(NewUser::hash_password(password, &server_config.secret)?)
                        }
                        _ => unmodified_user.password,
                    };
                    let new_user = User {
                        id: uid,
                        username: data.username.clone(),
                        email: data.email.clone(),
                        password,
                        role: unmodified_user.role,
                        language: unmodified_user.language,
                    };
                    new_user.update_user(server_config).await?;
                    let changed_user = User::get_user(uid, server_config).await?;
                    Ok(Item {
                        user: changed_user.clone(),
                        item: changed_user,
                    })
                }
                RoleGuard::NotAuthenticated | RoleGuard::Disabled => {
                    unreachable!("Should be unreachable because of the `admin_or_self`")
                }
            }
        } else {
            Err(ServerError::User("Not a valid UID".to_owned()))
        }
    } else {
        Err(ServerError::User("Not a valid UID".to_owned()))
    }
}

/// Demote an admin user to a normal user or promote a normal user to admin privileges.
///
/// # Errors
/// Fails with [`ServerError`] if access to the database fails, this user does not have permissions or the user does not exist.
#[instrument(skip(id))]
pub async fn toggle_admin(
    id: &Identity,
    user_id: Option<i64>,
    server_config: &ServerConfig,
) -> Result<Item<User>, ServerError> {
    if let Some(uid) = user_id {
        let auth = authenticate(id, server_config).await?;
        match auth {
            RoleGuard::Admin { .. } => {
                info!("Changing administrator privileges: ");

                let unchanged_user = User::get_user(uid, server_config).await?;

                let old = unchanged_user.role;
                unchanged_user.toggle_admin(server_config).await?;

                info!("Toggling role: old was {:?}", old);

                let changed_user = User::get_user(uid, server_config).await?;
                info!("Toggled role: new is {:?}", changed_user.role);
                Ok(Item {
                    user: changed_user.clone(),
                    item: changed_user,
                })
            }
            RoleGuard::Regular { .. } | RoleGuard::NotAuthenticated | RoleGuard::Disabled => {
                Err(ServerError::User("Permission denied".to_owned()))
            }
        }
    } else {
        Err(ServerError::User("Permission denied".to_owned()))
    }
}

/// Set the language of a given user
///
/// # Errors
/// Fails with [`ServerError`] if access to the database fails, this user does not have permissions or the language given is invalid.
#[instrument(skip(id))]
pub async fn set_language(
    id: &Identity,
    lang_code: Lang,
    server_config: &ServerConfig,
) -> Result<(), ServerError> {
    match authenticate(id, server_config).await? {
        RoleGuard::Admin { user } | RoleGuard::Regular { user } => {
            user.set_language(server_config, lang_code).await
        }
        RoleGuard::Disabled | RoleGuard::NotAuthenticated => {
            Err(ServerError::User("Not Allowed".to_owned()))
        }
    }
}

/// Get one link if permissions are accordingly.
///
/// # Errors
/// Fails with [`ServerError`] if access to the database fails or this user does not have permissions.
#[instrument(skip(id))]
pub async fn get_link(
    id: &Identity,
    link_code: &str,
    server_config: &ServerConfig,
) -> Result<Item<Link>, ServerError> {
    match authenticate(id, server_config).await? {
        RoleGuard::Admin { user } | RoleGuard::Regular { user } => {
            let link = Link::get_link_by_code(link_code, server_config).await?;
            Ok(Item { user, item: link })
        }
        RoleGuard::Disabled | RoleGuard::NotAuthenticated => {
            warn!("User could not be authenticated!");
            Err(ServerError::User("Not Allowed".to_owned()))
        }
    }
}

/// Get monthly statistics for one link if permissions are accordingly.
///
/// # Errors
/// Fails with [`ServerError`] if access to the database fails or this user does not have permissions.
#[instrument(skip(id))]
pub async fn get_statistics(
    id: &Identity,
    link_id: i64,
    server_config: &ServerConfig,
) -> Result<Item<Statistics>, ServerError> {
    match authenticate(id, server_config).await? {
        RoleGuard::Admin { user } | RoleGuard::Regular { user } => {
            let stats = Link::get_statistics(link_id, server_config).await?;
            Ok(Item { user, item: stats })
        }
        RoleGuard::Disabled | RoleGuard::NotAuthenticated => {
            warn!("User could not be authenticated!");
            Err(ServerError::User("Not Allowed".to_owned()))
        }
    }
}

/// Get one link if permissions are accordingly.
///
/// # Errors
/// Fails with [`ServerError`] if access to the database fails or this user does not have permissions.
#[instrument(skip(id))]
pub async fn get_link_by_id(
    id: &Identity,
    lid: i64,
    server_config: &ServerConfig,
) -> Result<Item<Link>, ServerError> {
    match authenticate(id, server_config).await? {
        RoleGuard::Admin { user } | RoleGuard::Regular { user } => {
            let link = Link::get_link_by_id(lid, server_config).await?;
            Ok(Item { user, item: link })
        }
        RoleGuard::Disabled | RoleGuard::NotAuthenticated => {
            warn!("User could not be authenticated!");
            Err(ServerError::User("Not Allowed".to_owned()))
        }
    }
}

/// Get link **without authentication**
///
/// # Errors
/// Fails with [`ServerError`] if access to the database fails.
#[instrument()]
pub async fn get_link_simple(
    link_code: &str,
    server_config: &ServerConfig,
) -> Result<Link, ServerError> {
    info!("Getting link for {:?}", link_code);

    let link = Link::get_link_by_code(link_code, server_config).await?;
    info!("Foun d link for {:?}", link);
    Ok(link)
}

/// Click on a link
///
/// # Errors
/// Fails with [`ServerError`] if access to the database fails.
#[instrument()]
pub async fn click_link(link_id: i64, server_config: &ServerConfig) -> Result<(), ServerError> {
    info!("Clicking on {:?}", link_id);
    let new_click = NewClick::new(link_id);
    new_click.insert_click(server_config).await?;
    Ok(())
}

/// Delete a link
///
/// # Errors
/// Fails with [`ServerError`] if access to the database fails or this user does not have permissions.
#[instrument(skip(id))]
pub async fn delete_link(
    id: &Identity,
    link_code: &str,
    server_config: &ServerConfig,
) -> Result<(), ServerError> {
    let auth = authenticate(id, server_config).await?;
    let link: Link = get_link_simple(link_code, server_config).await?;
    if auth.admin_or_self(link.author) {
        Link::delete_link_by_code(link_code, server_config).await?;
        Ok(())
    } else {
        Err(ServerError::User("Permission denied!".to_owned()))
    }
}

/// Create a new link
///
/// # Errors
/// Fails with [`ServerError`] if access to the database fails or this user does not have permissions.
#[instrument(skip(id))]
pub async fn create_link(
    id: &Identity,
    data: LinkDelta,
    server_config: &ServerConfig,
) -> Result<Item<Link>, ServerError> {
    let auth = authenticate(id, server_config).await?;
    match auth {
        RoleGuard::Admin { user } | RoleGuard::Regular { user } => {
            let code = data.code.clone();
            info!("Creating link for: {}", &code);
            let new_link = NewLink::from_link_delta(data, user.id);
            info!("Creating link for: {:?}", &new_link);

            new_link.insert(server_config).await?;
            let new_link: Link = get_link_simple(&code, server_config).await?;
            Ok(Item {
                user,
                item: new_link,
            })
        }
        RoleGuard::Disabled | RoleGuard::NotAuthenticated => {
            Err(ServerError::User("Permission denied!".to_owned()))
        }
    }
}

/// Update a link if the user is admin or it is its own link.
///
/// # Errors
/// Fails with [`ServerError`] if access to the database fails or this user does not have permissions.
#[instrument(skip(ident))]
pub async fn update_link(
    ident: &Identity,
    data: LinkDelta,
    server_config: &ServerConfig,
) -> Result<Item<Link>, ServerError> {
    let auth = authenticate(ident, server_config).await?;
    match auth {
        RoleGuard::Admin { .. } | RoleGuard::Regular { .. } => {
            if let Some(id) = data.id {
                let query: Item<Link> = get_link_by_id(ident, id, server_config).await?;
                if auth.admin_or_self(query.item.author) {
                    let mut link = query.item;
                    let LinkDelta {
                        title,
                        target,
                        code,
                        ..
                    } = data;
                    link.code = code.clone();
                    link.target = target;
                    link.title = title;
                    link.update_link(server_config).await?;
                    get_link(ident, &code, server_config).await
                } else {
                    Err(ServerError::User("Invalid Request".to_owned()))
                }
            } else {
                Err(ServerError::User("Not Allowed".to_owned()))
            }
        }
        RoleGuard::Disabled | RoleGuard::NotAuthenticated => {
            Err(ServerError::User("Not Allowed".to_owned()))
        }
    }
}
/// Create a new link
///
/// # Errors
/// Fails with [`ServerError`] if access to the database fails or this user does not have permissions.
#[instrument(skip(id))]
pub async fn create_link_json(
    id: &Identity,
    data: actix_web::web::Json<LinkDelta>,
    server_config: &ServerConfig,
) -> Result<Item<Link>, ServerError> {
    let auth = authenticate(id, server_config).await?;
    match auth {
        RoleGuard::Admin { user } | RoleGuard::Regular { user } => {
            let code = data.code.clone();
            info!("Creating link for: {}", &code);
            let new_link = NewLink::from_link_delta(data.into_inner(), user.id);
            info!("Creating link for: {:?}", &new_link);

            new_link.insert(server_config).await?;
            let new_link: Link = get_link_simple(&code, server_config).await?;
            Ok(Item {
                user,
                item: new_link,
            })
        }
        RoleGuard::Disabled | RoleGuard::NotAuthenticated => {
            Err(ServerError::User("Permission denied!".to_owned()))
        }
    }
}
