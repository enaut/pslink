//! Database export functionality for admin users using consistent SQLite snapshots.

#[cfg(feature = "server")]
use dioxus::logger::tracing::info;
use dioxus::prelude::*;

/// Export the database with secret validation and admin authentication.
///
/// Uses `VACUUM INTO` to generate a consistent, standalone SQLite snapshot before reading.
///
/// # Errors
/// Fails with [`ServerFnError`] if:
/// - User is not authenticated as admin
/// - Invalid secret provided
/// - Database snapshot cannot be created or read
#[server(endpoint = "export_database")]
pub async fn export_database(secret: String) -> Result<Vec<u8>, ServerFnError> {
    use crate::server::{auth::require_admin, state::get_state};

    let user = require_admin().await?;
    let state = get_state().await?;

    if let Err(e) = validate_export_secret(&state, &secret) {
        info!(
            "Invalid export secret provided by user {}: {}",
            user.username, e
        );
        return Err(ServerFnError::new("Invalid secret"));
    }

    export_database_snapshot(&state.db, &state.config.db).await.map(|data| {
        info!(
            "Database export successful for admin user: {}",
            user.username
        );
        data
    })
}

#[cfg(feature = "server")]
pub async fn export_database_snapshot(
    pool: &sqlx::SqlitePool,
    db_path: &std::path::Path,
) -> Result<Vec<u8>, ServerFnError> {
    use tokio::fs;
    use crate::server::ServerResultExt;

    let rand_suffix: u64 = rand::Rng::r#gen(&mut rand::thread_rng());
    let snapshot_path = db_path.with_extension(format!("export-{}.sqlite", rand_suffix));
    if snapshot_path.exists() {
        let _ = fs::remove_file(&snapshot_path).await;
    }

    let snapshot_str = snapshot_path.to_string_lossy().to_string();
    let vacuum_res = sqlx::query("VACUUM INTO ?")
        .bind(&snapshot_str)
        .execute(pool)
        .await;

    if let Err(e) = vacuum_res {
        info!("Failed to create database snapshot via VACUUM INTO: {}", e);
        let _ = fs::remove_file(&snapshot_path).await;
        return Err(ServerFnError::new("Failed to create database snapshot"));
    }

    let data_res = fs::read(&snapshot_path).await.sfe();
    let _ = fs::remove_file(&snapshot_path).await;

    data_res.map_err(|e| {
        info!("Failed to read exported database snapshot: {}", e);
        ServerFnError::new("Failed to read database file")
    })
}

/// Validate the secret token for database export.
#[cfg(feature = "server")]
pub fn validate_export_secret(
    state: &crate::server::state::AppState,
    provided_secret: &str,
) -> Result<(), ServerFnError> {
    let expected_secret_str = state
        .config
        .data_download_secret
        .secret
        .as_ref()
        .ok_or_else(|| ServerFnError::new("Data download secret not set"))?;

    if provided_secret.as_bytes().len() != expected_secret_str.as_bytes().len()
        || provided_secret
            .as_bytes()
            .iter()
            .zip(expected_secret_str.as_bytes().iter())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            != 0
    {
        return Err(ServerFnError::new("Invalid secret token"));
    }

    Ok(())
}
