pub mod models;
pub mod queries;

use actix_web::HttpResponse;
use qrcode::types::QrError;
use shared::datatypes::Secret;
use sqlx::{Pool, Sqlite};
use std::{fmt::Display, path::PathBuf, str::FromStr};
use thiserror::Error;
use tracing::error;

/// The Error type that is returned by most function calls if anything failed.
#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Failed to encrypt the password {0} - aborting!")]
    Argonautica(argonautica::Error),
    #[error("The database could not be used: {0}")]
    Database(#[from] sqlx::Error),
    #[error("The database could not be migrated: {0}")]
    DatabaseMigration(#[from] sqlx::migrate::MigrateError),
    #[error("The environment file could not be read")]
    Environment(#[from] std::env::VarError),
    #[error("The qr-code could not be generated: {0}")]
    Qr(#[from] QrError),
    #[error("Some error happened during input and output: {0}")]
    Io(#[from] std::io::Error),
    #[error("Error: {0}")]
    User(String),
}

impl From<argonautica::Error> for ServerError {
    fn from(e: argonautica::Error) -> Self {
        Self::Argonautica(e)
    }
}

/// Any error can be rendered to a html string.
impl ServerError {
    fn render_error(title: &str, content: &str) -> String {
        format!(
            "<!DOCTYPE html>
        <html lang=\"en\">
        <head>
            <meta charset=\"utf-8\">
            <title>{0}</title>
            <meta name=\"author\" content=\"Franz Dietrich\">
            <meta http-equiv=\"robots\" content=\"[noindex|nofollow]\">
            <link rel=\"stylesheet\" href=\"/static/style.css\">
        </head>
        <body>
        <section class=\"centered\">
        <h1>{0}</h1>
        <div class=\"container\">
          {1}
        </div>
      </section>
      </body>
      </html>",
            title, content
        )
    }
}

/// Make the error type work nicely with the actix server.
impl actix_web::error::ResponseError for ServerError {
    fn error_response(&self) -> HttpResponse {
        match self {
            Self::Argonautica(e) => {
                eprintln!("Argonautica Error happened: {:?}", e);
                HttpResponse::InternalServerError()
                    .body("Failed to encrypt the password - Aborting!")
            }
            Self::Database(e) => {
                eprintln!("Database Error happened: {:?}", e);
                HttpResponse::InternalServerError().body(&Self::render_error(
                    "Server Error",
                    "Database could not be accessed! - It could be that this value already was in the database! If you are the admin look into the logs for a more detailed error.",
                ))
            }
            Self::DatabaseMigration(e) => {
                eprintln!("Migration Error happened: {:?}", e);
                unimplemented!("A migration error should never be rendered")
            }
            Self::Environment(e) => {
                eprintln!("Environment Error happened: {:?}", e);
                HttpResponse::InternalServerError().body(&Self::render_error(
                  "Server Error",
                  "This Server is not properly configured, if you are the admin look into the installation- or update instructions!",
              ))
            }
            Self::Qr(e) => {
                eprintln!("QR Error happened: {:?}", e);
                HttpResponse::InternalServerError().body(&Self::render_error(
                    "Server Error",
                    "Could not generate the QR-code!",
                ))
            }
            Self::Io(e) => {
                eprintln!("Io Error happened: {:?}", e);
                HttpResponse::InternalServerError().body(&Self::render_error(
                    "Server Error",
                    "Some Files could not be read or written. If you are the admin look into the logfiles for more details.",
                ))
            }
            Self::User(data) => {
                eprintln!("User Error happened: {:?}", data);
                HttpResponse::InternalServerError().body(&Self::render_error(
                    "Server Error",
                    &format!("An error happened: {}", data),
                ))
            }
        }
    }
}

/// The qr-code can contain two different protocolls
#[derive(Debug, Clone)]
pub enum Protocol {
    Http,
    Https,
}

impl Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http => f.write_str("http"),
            Self::Https => f.write_str("https"),
        }
    }
}

impl FromStr for Protocol {
    type Err = ServerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "http" => Ok(Self::Http),
            "https" => Ok(Self::Https),
            _ => Err(ServerError::User("Failed to parse Protocol".to_owned())),
        }
    }
}

/// The configuration of the server. It is accessible by the views and other parts of the program. Globally valid settings should be stored here.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub secret: Secret,
    pub db: PathBuf,
    pub db_pool: Pool<Sqlite>,
    pub public_url: String,
    pub internal_ip: String,
    pub port: u32,
    pub protocol: Protocol,
    pub empty_forward_url: String,
    pub brand_name: String,
}

/// The configuration can be serialized into an environment-file.
impl ServerConfig {
    #[must_use]
    pub fn to_env_strings(&self) -> Vec<String> {
        vec![
            format!("PSLINK_DATABASE=\"{}\"\n", self.db.display()),
            format!("PSLINK_PORT={}\n", self.port),
            format!("PSLINK_PUBLIC_URL=\"{}\"\n", self.public_url),
            format!("PSLINK_EMPTY_FORWARD_URL=\"{}\"\n", self.empty_forward_url),
            format!("PSLINK_BRAND_NAME=\"{}\"\n", self.brand_name),
            format!("PSLINK_IP=\"{}\"\n", self.internal_ip),
            format!("PSLINK_PROTOCOL=\"{}\"\n", self.protocol),
            concat!(
                "# The SECRET_KEY variable is used for password encryption.\n",
                "# If it is changed all existing passwords are invalid.\n"
            )
            .to_owned(),
            format!(
                "PSLINK_SECRET=\"{}\"\n",
                self.secret
                    .secret
                    .as_ref()
                    .expect("A Secret was not specified!")
            ),
        ]
    }
}
