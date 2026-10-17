//! Authentication and session server functions.

#[cfg(feature = "server")]
use dioxus::logger::tracing::info;
use dioxus::prelude::{ServerFnError, server};

use crate::domain::{models::User, requests::SessionInfo};

#[server(endpoint = "login")]
pub async fn login(username: String, password: String) -> Result<User, ServerFnError> {
    use crate::{
        domain::models::Role,
        server::{
            auth::get_session,
            db::{NewUser, get_user_with_password_by_name},
            state::get_state,
        },
    };

    let auth = get_session().await?;
    let state = get_state().await?;

    let username = username.trim();
    if username.is_empty() || password.is_empty() {
        return Err(ServerFnError::new("Username or password cannot be empty"));
    }
    info!("Login attempt for user: {}", username);
    if username.len() > 64 || password.chars().count() > 256 {
        return Err(ServerFnError::new("Username or password too long"));
    }

    let (user, stored_hash) = get_user_with_password_by_name(&state.db, username)
        .await
        .map_err(|_| ServerFnError::new("Unauthorized"))?;

    if user.role == Role::Disabled || user.role == Role::NotAuthenticated {
        info!("Login rejected for disabled user: {}", user.username);
        return Err(ServerFnError::new("Account is disabled"));
    }

    if let Err(e) = NewUser::verify_password(&password, &stored_hash, &state.config.secret) {
        info!("Failed to verify password for {}: {}", &user.username, e);
        return Err(ServerFnError::new("Unauthorized"));
    }

    auth.login_user(user.id);
    Ok(user)
}

#[server(endpoint = "logout")]
pub async fn logout() -> Result<(), ServerFnError> {
    let auth = crate::server::auth::get_session().await?;
    auth.logout_user();
    Ok(())
}

#[server(endpoint = "session")]
pub async fn get_session_info() -> Result<SessionInfo, ServerFnError> {
    use crate::{
        domain::models::Role,
        server::{
            auth::{get_hostname, get_session},
            state::get_state,
        },
    };

    let auth = get_session().await?;
    let state = get_state().await?;
    let hostname = get_hostname()
        .await
        .unwrap_or_else(|_| state.config.public_url.clone());

    let user = auth
        .current_user
        .and_then(|aa| aa.get_user())
        .filter(|u| matches!(u.role, Role::Regular | Role::Admin));

    let base_url = state.config.base_url(Some(&hostname));

    Ok(SessionInfo {
        user,
        hostname,
        base_url,
        brand_name: state.config.brand_name.clone(),
        is_demo: state.config.secret.is_random,
    })
}
