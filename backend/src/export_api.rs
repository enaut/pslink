//! Database export functionality for admin users

#[cfg(feature = "server")]
use crate::get_data_download_secret;
#[cfg(feature = "server")]
use dioxus::logger::tracing::info;
use dioxus::prelude::*;
#[cfg(feature = "server")]
use pslink_shared::apirequests::users::Role;

/// Export the database with secret validation and admin authentication
///
/// # Errors
/// Fails with [`ServerFnError`] if:
/// - User is not authenticated as admin
/// - Invalid secret provided
/// - Database file cannot be read
#[server(ExportDatabase, endpoint = "export_database")]
pub async fn export_database(secret: String) -> Result<Vec<u8>, ServerFnError> {
    use std::path::Path;
    use tokio::fs;

    // Check if user is authenticated and is admin
    let auth = crate::auth::get_session().await?;
    let user = auth
        .current_user
        .ok_or_else(|| ServerFnError::new("Authentication required"))?
        .get_user()
        .ok_or_else(|| ServerFnError::new("User information not available"))?;

    // Only admin users can access export functionality
    if user.role != Role::Admin {
        return Err(ServerFnError::new("Administrator permissions required"));
    }

    // Validate the secret
    if let Err(e) = validate_export_secret(&secret) {
        info!("Invalid export secret provided by user {}: {}", user.username, e);
        return Err(ServerFnError::new("Invalid secret"));
    }

    // Get database path and read file
    let db_path = crate::get_db_path().await;
    
    if !Path::new(&db_path).exists() {
        return Err(ServerFnError::new("Database file not found"));
    }

    match fs::read(&db_path).await {
        Ok(data) => {
            info!("Database export successful for admin user: {}", user.username);
            Ok(data)
        }
        Err(e) => {
            info!("Failed to read database file: {}", e);
            Err(ServerFnError::new("Failed to read database file"))
        }
    }
}

/// Validate the secret token for database export
///
/// # Errors  
/// Fails with [`ServerFnError`] if the secret token is invalid.
#[cfg(feature = "server")]
pub fn validate_export_secret(provided_secret: &str) -> Result<(), ServerFnError> {
    let expected_secret = get_data_download_secret();
    let expected_secret_str = expected_secret.secret.as_ref().expect("Data download secret not set");
    
    if provided_secret != expected_secret_str {
        return Err(ServerFnError::new("Invalid secret token"));
    }
    
    Ok(())
}