//! Database and link export functionality for admin users.

#[cfg(feature = "server")]
use dioxus::logger::tracing::info;
use dioxus::prelude::*;

use crate::domain::models::{Clicks, FullLink};

/// Export all links as CSV for admin users.
///
/// Encodes CSV in UTF-8 with a BOM (`\u{FEFF}`) for automatic compatibility with
/// LibreOffice Calc and Microsoft Excel. Escapes fields conforming to RFC 4180.
///
/// # Errors
/// Fails with [`ServerFnError`] if:
/// - User is not authenticated as admin
/// - Database query fails
#[server(endpoint = "export_links_csv")]
pub async fn export_links_csv(delimiter: String) -> Result<String, ServerFnError> {
    use crate::server::{auth::require_admin, db, state::get_state};

    let user = require_admin().await?;
    let state = get_state().await?;

    info!(
        "Links CSV export requested by admin user: {}",
        user.username
    );

    let links = db::get_all_links_for_export(&state.db).await?;
    let base_url = state.config.base_url();

    let delim_char = match delimiter.as_str() {
        ";" => ';',
        "|" => '|',
        _ => ',',
    };

    let csv_content = format_links_as_csv(&links, &base_url, delim_char);
    Ok(csv_content)
}

/// Escape a CSV field according to RFC 4180 rules using the specified delimiter.
pub fn escape_csv_field(field: &str, delimiter: char) -> String {
    if field.contains(delimiter)
        || field.contains('"')
        || field.contains('\n')
        || field.contains('\r')
    {
        let escaped = field.replace('"', "\"\"");
        format!("\"{escaped}\"")
    } else {
        field.to_string()
    }
}

/// Format a list of [`FullLink`] items into a CSV string with a UTF-8 BOM.
pub fn format_links_as_csv(links: &[FullLink], base_url: &str, delimiter: char) -> String {
    let delim_str = delimiter.to_string();
    let headers = [
        "ID",
        "Code",
        "Short URL",
        "Target URL",
        "Title",
        "Comment",
        "Author",
        "Created At",
        "Total Clicks",
        "Check Status",
    ];

    let header_line = headers
        .iter()
        .map(|h| escape_csv_field(h, delimiter))
        .collect::<Vec<_>>()
        .join(&delim_str);

    let mut out = String::from("\u{FEFF}");
    out.push_str(&header_line);
    out.push_str("\r\n");

    for item in links {
        let total_clicks = match &item.clicks {
            Clicks::Count(c) => c.number,
            Clicks::Extended(s) => s.total.number,
        };
        let check_status = match &item.last_check {
            Some(check) => check.status.as_str(),
            None => "unchecked",
        };
        let short_url = crate::ui::links::generate_url_for_code(&item.link.code, base_url);

        let row = [
            item.link.id.to_string(),
            item.link.code.clone(),
            short_url,
            item.link.target.clone(),
            item.link.title.clone(),
            item.link.comment.clone(),
            item.user.username.clone(),
            item.link.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            total_clicks.to_string(),
            check_status.to_string(),
        ];

        let row_line = row
            .iter()
            .map(|f| escape_csv_field(f, delimiter))
            .collect::<Vec<_>>()
            .join(&delim_str);

        out.push_str(&row_line);
        out.push_str("\r\n");
    }

    out
}

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

    export_database_snapshot(&state.db, &state.config.db)
        .await
        .inspect(|_| {
            info!(
                "Database export successful for admin user: {}",
                user.username
            );
        })
}

#[cfg(feature = "server")]
pub async fn export_database_snapshot(
    pool: &sqlx::SqlitePool,
    db_path: &std::path::Path,
) -> Result<Vec<u8>, ServerFnError> {
    use crate::server::ServerResultExt;
    use tokio::fs;

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

    if provided_secret.len() != expected_secret_str.len()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::{Count, Lang, Link, Role, User};
    use chrono::NaiveDateTime;

    #[test]
    fn test_escape_csv_field() {
        assert_eq!(escape_csv_field("simple", ','), "simple");
        assert_eq!(escape_csv_field("with,comma", ','), "\"with,comma\"");
        assert_eq!(
            escape_csv_field("with;semicolon", ';'),
            "\"with;semicolon\""
        );
        assert_eq!(escape_csv_field("with;semicolon", ','), "with;semicolon");
        assert_eq!(escape_csv_field("with|pipe", '|'), "\"with|pipe\"");
        assert_eq!(
            escape_csv_field("with \"quotes\"", ','),
            "\"with \"\"quotes\"\"\""
        );
        assert_eq!(escape_csv_field("with\nnewline", ','), "\"with\nnewline\"");
    }

    #[test]
    fn test_format_links_as_csv() {
        let link = FullLink {
            link: Link {
                id: 1,
                title: "My \"Special\" Title".to_string(),
                comment: "Internal, note; with pipe |".to_string(),
                target: "https://example.com/path?a=1&b=2".to_string(),
                code: "ex1".to_string(),
                author: 42,
                created_at: NaiveDateTime::parse_from_str(
                    "2026-11-04 12:30:00",
                    "%Y-%m-%d %H:%M:%S",
                )
                .unwrap(),
            },
            user: User {
                id: 42,
                username: "admin_user".to_string(),
                email: "admin@example.com".to_string(),
                role: Role::Admin,
                language: Lang::EnUS,
            },
            clicks: Clicks::Count(Count { number: 15 }),
            last_check: None,
        };

        // Comma delimiter
        let csv_comma = format_links_as_csv(&[link.clone()], "https://ps.li", ',');
        assert!(csv_comma.starts_with('\u{FEFF}'));
        assert!(csv_comma.contains("ID,Code,Short URL,Target URL,Title,Comment,Author,Created At,Total Clicks,Check Status\r\n"));
        assert!(csv_comma.contains("1,ex1,https://ps.li/ex1,https://example.com/path?a=1&b=2,\"My \"\"Special\"\" Title\",\"Internal, note; with pipe |\",admin_user,2026-11-04 12:30:00,15,unchecked\r\n"));

        // Semicolon delimiter
        let csv_semi = format_links_as_csv(&[link.clone()], "https://ps.li", ';');
        assert!(csv_semi.contains("ID;Code;Short URL;Target URL;Title;Comment;Author;Created At;Total Clicks;Check Status\r\n"));
        assert!(csv_semi.contains("1;ex1;https://ps.li/ex1;https://example.com/path?a=1&b=2;\"My \"\"Special\"\" Title\";\"Internal, note; with pipe |\";admin_user;2026-11-04 12:30:00;15;unchecked\r\n"));

        // Pipe delimiter
        let csv_pipe = format_links_as_csv(&[link], "https://ps.li", '|');
        assert!(csv_pipe.contains("ID|Code|Short URL|Target URL|Title|Comment|Author|Created At|Total Clicks|Check Status\r\n"));
        assert!(csv_pipe.contains("1|ex1|https://ps.li/ex1|https://example.com/path?a=1&b=2|\"My \"\"Special\"\" Title\"|\"Internal, note; with pipe |\"|admin_user|2026-11-04 12:30:00|15|unchecked\r\n"));
    }
}
