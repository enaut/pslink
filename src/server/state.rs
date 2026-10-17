//! Server application state injected into Axum and Dioxus Fullstack context.

use std::sync::Arc;

use dioxus::prelude::ServerFnError;
use dioxus_fullstack::FullstackContext;
use sqlx::SqlitePool;

use crate::server::cli::ServerConfig;

/// Shared server state holding the SQLite connection pool and immutable server configuration.
#[derive(Clone, Debug)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: Arc<ServerConfig>,
}

impl AppState {
    #[must_use]
    pub fn new(db: SqlitePool, config: ServerConfig) -> Self {
        Self {
            db,
            config: Arc::new(config),
        }
    }
}

/// Extract the [`AppState`] from the current Dioxus [`FullstackContext`].
///
/// # Errors
/// Fails with [`ServerFnError`] if the `Extension<AppState>` layer is missing.
pub async fn get_state() -> Result<AppState, ServerFnError> {
    FullstackContext::extract::<axum::Extension<AppState>, _>()
        .await
        .map(|ext| ext.0)
        .map_err(|_| ServerFnError::new("Server state was not found"))
}
