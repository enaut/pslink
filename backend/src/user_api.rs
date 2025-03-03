use std::str::FromStr as _;

#[cfg(feature = "server")]
use crate::{
    get_secret,
    models::{NewUser, UserDbOperations as _},
};
use dioxus::{logger::tracing::info, prelude::*};
use enum_map::EnumMap;
use pslink_shared::{
    apirequests::{
        general::{EditMode, Filter, Operation, Ordering},
        users::{Role, UserDelta, UserOverviewColumns, UserRequestForm},
    },
    datatypes::{Item, Lang, ListWithOwner, Secret, User},
};
#[cfg(feature = "server")]
use sqlx::Row;

/// Only admins can list all users other users will only see themselves.
///
/// # Errors
/// Fails with [`ServerError`] if access to the database fails.
#[server(ListAllUsersFiltered)]
pub async fn list_users(parameters: UserRequestForm) -> Result<ListWithOwner<User>, ServerFnError> {
    let auth = crate::auth::get_session().await?;
    info!("Auth: {:?}", auth);
    let user = auth
        .current_user
        .expect("not authenticated")
        .get_user()
        .expect("User is authenticated");
    let db = crate::get_db().await;
    match user.role {
        Role::Admin => {
            info!("Admin User {:?}", user.username);
            let mut querystring = "Select * from users".to_string();
            querystring.push_str(&generate_filter_users_sql(&parameters.filter));
            if let Some(order) = parameters.order {
                querystring.push_str(&generate_order_users_sql(&order));
            }
            querystring.push_str(&format!("\n LIMIT {}", parameters.amount));

            let query_result = sqlx::query(&querystring).fetch_all(&db).await;
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
        Role::Regular => {
            info!("Regular User {:?} users", user);
            Ok(ListWithOwner {
                user: user.clone(),
                list: vec![user],
            })
        }
        _ => Err(ServerFnError::new("Administrator permissions required")),
    }
}

/// Generate a filter statement for the SQL-Query according to the parameters...
///
/// Todo: this function only naively protects agains SQL-injections use better variants.
#[cfg(feature = "server")]
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
#[cfg(feature = "server")]
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

/// Create a new user and save it to the database
///
/// # Errors
/// Fails with [`ServerError`] if access to the database fails, this user does not have permissions or the user already exists.
#[server(CreateUser)]
pub async fn create_user(data: UserDelta) -> Result<Item<User>, ServerFnError> {
    info!("Creating a User: {:?}", &data);
    if data.edit != EditMode::Create {
        return Err(ServerFnError::new("Wrong Request".to_string()));
    }
    let auth = crate::auth::get_session().await?;
    info!("Auth: {:?}", auth);
    let user = auth
        .current_user
        .expect("not authenticated")
        .get_user()
        .expect("User is authenticated");

    // Require a password on user creation!
    let password = match &data.password {
        Some(pass) => pass,
        None => {
            return Err(ServerFnError::new(
                "A new users does require a password".to_string(),
            ))
        }
    };

    if user.role == Role::Admin {
        let new_user = NewUser::new(
            data.username.clone(),
            data.email.clone(),
            password,
            &get_secret(),
        )?;

        new_user.insert_user().await?;

        // querry the new user
        let new_user = User::get_user_by_name(&data.username).await?;
        Ok(Item {
            user,
            item: new_user,
        })
    } else {
        Err(ServerFnError::new("Permission denied!".to_owned()))
    }
}

/// Take a [`actix_web::web::Form<NewUser>`] and update the corresponding entry in the database.
/// The password is only updated if a new password of at least 4 characters is provided.
/// The `user_id` is never changed.
///
/// # Errors
/// Fails with [`ServerFnError`] if access to the database fails, this user does not have permissions, or the given data is malformed.
#[server(UpdateUser)]
pub async fn update_user(data: UserDelta) -> Result<Item<User>, ServerFnError> {
    let auth = crate::auth::get_session().await?;
    info!("Auth: {:?}", auth);
    let user = auth
        .current_user
        .expect("not authenticated")
        .get_user()
        .expect("User is authenticated");
    if let Some(uid) = data.id {
        let unmodified_user = User::get_user(uid).await?;
        if admin_or_self(&user, unmodified_user.id) {
            if user.role == Role::Admin || user.role == Role::Regular {
                info!("Updating userinfo: ");
                let password = match &data.password {
                    Some(password) if password.len() > 4 => {
                        Secret::new(NewUser::hash_password(password, &get_secret())?)
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
                new_user.update_user().await?;
                let changed_user = User::get_user(uid).await?;
                Ok(Item {
                    user: changed_user.clone(),
                    item: changed_user,
                })
            } else {
                unreachable!("Should be unreachable because of the `admin_or_self`")
            }
        } else {
            Err(ServerFnError::new("Not a valid UID".to_owned()))
        }
    } else {
        Err(ServerFnError::new("Not a valid UID".to_owned()))
    }
}

#[cfg(feature = "server")]
fn admin_or_self(user: &User, uid: i64) -> bool {
    return user.role == Role::Admin || user.id == uid;
}

/// Delete a user from the database.
/// Only admins can delete users.
/// The user can not delete itself.
#[server(DeleteUser)]
pub async fn delete_user(user_id: i64) -> Result<(), ServerFnError> {
    let auth = crate::auth::get_session().await?;
    info!("Auth: {:?}", auth);
    let user = auth
        .current_user
        .expect("not authenticated")
        .get_user()
        .expect("User is authenticated");
    if user.role == Role::Admin {
        if user.id != user_id {
            User::delete_user(user_id, user.id).await?;
            Ok(())
        } else {
            Err(ServerFnError::new("Can not delete yourself".to_owned()))
        }
    } else {
        Err(ServerFnError::new("Permission denied!".to_owned()))
    }
}
