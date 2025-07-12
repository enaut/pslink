//! Database export functionality for admin users

#[cfg(feature = "server")]
use crate::get_data_download_secret;
#[cfg(feature = "server")]
use dioxus::logger::tracing::info;
use dioxus::prelude::*;
#[cfg(feature = "server")]
use pslink_shared::apirequests::users::Role;

/// Generate a database export URL with the secret token for admin users
///
/// # Errors
/// Fails with [`ServerFnError`] if the user is not authenticated as admin.
#[server(GetExportUrl, endpoint = "get_export_url")]
pub async fn get_export_url() -> Result<String, ServerFnError> {
    let auth = crate::auth::get_session().await?;
    let user = auth
        .current_user
        .expect("not authenticated")
        .get_user()
        .expect("User is authenticated");

    // Only admin users can access export functionality
    if user.role != Role::Admin {
        return Err(ServerFnError::new("Administrator permissions required"));
    }

    let hostname = crate::auth::get_hostname().await?;
    let secret = get_data_download_secret();
    let secret_str = secret.secret.as_ref().expect("Data download secret not set");
    
    // Generate the export URL with the secret token
    let export_url = format!("http://{}/app/export?secret={}", hostname.0, secret_str);
    
    info!("Generated export URL for admin user: {}", user.username);
    Ok(export_url)
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