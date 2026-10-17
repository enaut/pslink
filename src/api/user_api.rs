//! User management server functions.

#[cfg(feature = "server")]
use dioxus::logger::tracing::info;
use dioxus::prelude::*;

use crate::domain::{
    models::{Lang, User},
    requests::{UserDelta, UserRequestForm},
};

/// Only admins can list all users; regular users only see themselves.
///
/// # Errors
/// Fails with [`ServerFnError`] if access to the database fails or the user is not authenticated.
#[server(endpoint = "list_all_users")]
pub async fn list_users(parameters: UserRequestForm) -> Result<Vec<User>, ServerFnError> {
    use crate::{
        domain::models::Role,
        server::{auth::require_user, db, state::get_state},
    };

    let user = require_user().await?;
    let state = get_state().await?;

    match user.role {
        Role::Admin => {
            let users = db::list_users_filtered(&state.db, &parameters).await?;
            info!("Found {} users", users.len());
            Ok(users)
        }
        Role::Regular => {
            if parameters.offset == 0 {
                Ok(vec![user])
            } else {
                Ok(vec![])
            }
        }
        Role::Disabled | Role::NotAuthenticated => {
            Err(ServerFnError::new("Administrator permissions required"))
        }
    }
}

/// Create a new user and save it to the database.
///
/// # Errors
/// Fails with [`ServerFnError`] if validation fails, database access fails, or the caller is not an admin.
#[server(endpoint = "create_user")]
pub async fn create_user(data: UserDelta) -> Result<User, ServerFnError> {
    use crate::{
        domain::{
            requests::EditMode,
            validation::{validate_password, validate_username},
        },
        server::{
            auth::require_admin,
            db::{self, NewUser},
            state::get_state,
        },
    };

    if data.edit != EditMode::Create {
        return Err(ServerFnError::new("Wrong Request".to_string()));
    }
    let _admin = require_admin().await?;
    let state = get_state().await?;

    let username = data.username.trim().to_string();
    let email = data.email.trim().to_string();
    validate_username(&username).map_err(ServerFnError::new)?;
    info!(
        "Creating user '{}' ({}) with role {:?}",
        &username, &email, data.role
    );

    let password = data
        .password
        .as_deref()
        .ok_or_else(|| ServerFnError::new("A new user requires a password"))?;
    validate_password(password).map_err(ServerFnError::new)?;

    let new_user = NewUser::new(
        username.clone(),
        email,
        password,
        &state.config.secret,
    )?;

    new_user.insert_user(&state.db, data.role).await?;
    let created = db::get_user_by_name(&state.db, &username).await?;

    Ok(created)
}

/// Update an existing user's profile, password, or role.
///
/// # Errors
/// Fails with [`ServerFnError`] if validation fails, the caller lacks permission, or database access fails.
#[server(endpoint = "update_user")]
pub async fn update_user(data: UserDelta) -> Result<User, ServerFnError> {
    use crate::{
        domain::{
            models::Role,
            validation::{validate_password, validate_username},
        },
        server::{
            auth::{get_session, require_user},
            db::{self, NewUser},
            state::get_state,
        },
    };

    let current_user = require_user().await?;
    let state = get_state().await?;

    let uid = data
        .id
        .ok_or_else(|| ServerFnError::new("Not a valid UID".to_owned()))?;

    if current_user.role != Role::Admin && current_user.id != uid {
        return Err(ServerFnError::new("Permission denied!".to_owned()));
    }

    let username = data.username.trim().to_string();
    let email = data.email.trim().to_string();
    validate_username(&username).map_err(ServerFnError::new)?;

    let (unmodified_user, existing_password_hash) =
        db::get_user_with_password(&state.db, uid).await?;

    let password_hash = match data.password.as_deref() {
        Some(password) if !password.is_empty() => {
            validate_password(password).map_err(ServerFnError::new)?;
            NewUser::hash_password(password, &state.config.secret)?
        }
        _ => existing_password_hash,
    };

    let role = if current_user.role == Role::Admin {
        info!(
            "Admin user {}, setting role of user {} to: {:?}",
            current_user.id, uid, data.role
        );
        data.role
    } else {
        unmodified_user.role
    };

    let updated_user = User {
        id: uid,
        username,
        email,
        role,
        language: unmodified_user.language,
    };

    db::update_user(&state.db, &updated_user, &password_hash).await?;

    if let Ok(auth) = get_session().await {
        auth.cache_clear_user(uid);
    }

    db::get_user(&state.db, uid).await
}

/// Delete a user from the database (admin only; cannot delete oneself).
#[server(endpoint = "delete_user")]
pub async fn delete_user(user_id: i64) -> Result<(), ServerFnError> {
    use crate::server::{
        auth::{get_session, require_admin},
        db,
        state::get_state,
    };

    let admin = require_admin().await?;
    if admin.id == user_id {
        return Err(ServerFnError::new("Can not delete yourself".to_owned()));
    }
    let state = get_state().await?;
    db::delete_user(&state.db, user_id, admin.id).await?;

    if let Ok(auth) = get_session().await {
        auth.cache_clear_user(user_id);
    }
    Ok(())
}

/// Set the current user's preferred language.
#[server(endpoint = "set_user_language")]
pub async fn set_user_language(language: Lang) -> Result<(), ServerFnError> {
    use crate::server::{
        auth::{get_session, require_user},
        db,
        state::get_state,
    };

    let user = require_user().await?;
    let state = get_state().await?;

    db::set_language(&state.db, user.id, language).await?;
    if let Ok(auth) = get_session().await {
        auth.cache_clear_user(user.id);
    }
    Ok(())
}
