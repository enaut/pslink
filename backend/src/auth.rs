#[cfg(feature = "server")]
use crate::models::UserDbOperations;

use axum::extract::Host;
use dioxus::prelude::{ServerFnError, extract};
use pslink_shared::datatypes::User;

#[derive(Debug, Clone)]
pub(crate) struct AuthAccount {
    user: Option<User>,
}

impl AuthAccount {
    pub fn get_user(&self) -> Option<User> {
        self.user.clone()
    }
}

#[cfg(feature = "server")]
#[async_trait::async_trait]
impl axum_session_auth::Authentication<AuthAccount, i64, sqlx::Pool<sqlx::Sqlite>> for AuthAccount {
    async fn load_user(
        userid: i64,
        _pool: Option<&sqlx::SqlitePool>,
    ) -> Result<AuthAccount, anyhow::Error> {
        Ok(Self {
            user: User::get_user(userid).await.ok(),
        })
    }

    fn is_authenticated(&self) -> bool {
        self.user.is_some()
    }

    fn is_active(&self) -> bool {
        self.user.is_some()
    }

    fn is_anonymous(&self) -> bool {
        self.user.is_none()
    }
}

pub type Session = axum_session_auth::AuthSession<
    AuthAccount,
    i64,
    axum_session_auth::SessionSqlitePool,
    sqlx::SqlitePool,
>;

pub async fn get_session() -> Result<Session, ServerFnError> {
    extract::<_, _>()
        .await
        .map_err(|_| ServerFnError::new("AuthSessionLayer was not found"))
}

pub async fn get_hostname() -> Result<Host, ServerFnError> {
    extract::<_, _>()
        .await
        .map_err(|_| ServerFnError::new("Hostname was not found"))
}
