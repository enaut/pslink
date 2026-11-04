//! Authentication and session extractors for Axum and Dioxus server functions.

use dioxus::prelude::ServerFnError;
use dioxus_fullstack::FullstackContext;

use crate::domain::models::{Role, User};

#[derive(Debug, Clone)]
pub struct AuthAccount {
    user: Option<User>,
}

impl AuthAccount {
    #[must_use]
    pub fn get_user(&self) -> Option<User> {
        self.user.clone()
    }
}

#[async_trait::async_trait]
impl axum_session_auth::Authentication<AuthAccount, i64, sqlx::Pool<sqlx::Sqlite>> for AuthAccount {
    async fn load_user(
        userid: i64,
        pool: Option<&sqlx::SqlitePool>,
    ) -> Result<AuthAccount, anyhow::Error> {
        let user = if let Some(pool) = pool {
            crate::server::db::get_user(pool, userid).await.ok()
        } else {
            None
        };
        Ok(Self { user })
    }

    fn is_authenticated(&self) -> bool {
        matches!(
            self.user.as_ref().map(|u| u.role),
            Some(Role::Regular | Role::Admin)
        )
    }

    fn is_active(&self) -> bool {
        self.is_authenticated()
    }

    fn is_anonymous(&self) -> bool {
        !self.is_authenticated()
    }
}

pub type Session = axum_session_auth::AuthSession<
    AuthAccount,
    i64,
    axum_session_sqlx::SessionSqlitePool,
    sqlx::SqlitePool,
>;

pub async fn get_session() -> Result<Session, ServerFnError> {
    FullstackContext::extract::<_, _>()
        .await
        .map_err(|_| ServerFnError::new("AuthSessionLayer was not found"))
}

/// Extract and verify an authenticated, active (non-disabled) user from the session.
///
/// # Errors
/// Returns [`ServerFnError`] if the user is not logged in or if their account is disabled.
pub async fn require_user() -> Result<User, ServerFnError> {
    let auth = get_session().await?;
    let user = auth
        .current_user
        .and_then(|aa| aa.get_user())
        .ok_or_else(|| ServerFnError::new("Authentication required"))?;

    match user.role {
        Role::Regular | Role::Admin => Ok(user),
        Role::Disabled => Err(ServerFnError::new("Account is disabled")),
        Role::NotAuthenticated => Err(ServerFnError::new("Authentication required")),
    }
}

/// Extract and verify an authenticated administrator user from the session.
///
/// # Errors
/// Returns [`ServerFnError`] if the user is not logged in or does not have [`Role::Admin`].
pub async fn require_admin() -> Result<User, ServerFnError> {
    let user = require_user().await?;
    if user.role != Role::Admin {
        return Err(ServerFnError::new("Administrator permissions required"));
    }
    Ok(user)
}
