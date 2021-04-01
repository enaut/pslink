extern crate sqlx;
#[allow(unused_imports)]
#[macro_use(
    slog_o,
    slog_info,
    slog_warn,
    slog_error,
    slog_log,
    slog_record,
    slog_record_static,
    slog_b,
    slog_kv
)]
extern crate slog;
extern crate slog_async;

pub mod forms;
pub mod models;
pub mod queries;

use std::{fmt::Display, path::PathBuf, str::FromStr};

use actix_web::HttpResponse;

use qrcode::types::QrError;
use sqlx::{Pool, Sqlite};

#[derive(Debug)]
pub enum ServerError {
    Argonautic,
    Database(sqlx::Error),
    DatabaseMigration(sqlx::migrate::MigrateError),
    Environment,
    Template(tera::Error),
    Qr(QrError),
    Io(std::io::Error),
    User(String),
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Argonautic => write!(f, "Argonautica Error"),
            Self::Database(e) => write!(f, "Database Error: {}", e),
            Self::DatabaseMigration(e) => {
                write!(f, "Migration Error: {}", e)
            }
            Self::Environment => write!(f, "Environment Error"),
            Self::Template(e) => write!(f, "Template Error: {:?}", e),
            Self::Qr(e) => write!(f, "Qr Code Error: {:?}", e),
            Self::Io(e) => write!(f, "IO Error: {:?}", e),
            Self::User(data) => write!(f, "{}", data),
        }
    }
}

impl actix_web::error::ResponseError for ServerError {
    fn error_response(&self) -> HttpResponse {
        match self {
            Self::Argonautic => HttpResponse::InternalServerError().json("Argonautica Error"),
            Self::Database(e) => {
                HttpResponse::InternalServerError().json(format!("Database Error: {:?}", e))
            }
            Self::DatabaseMigration(_) => {
                unimplemented!("A migration error should never be rendered")
            }
            Self::Environment => HttpResponse::InternalServerError().json("Environment Error"),
            Self::Template(e) => {
                HttpResponse::InternalServerError().json(format!("Template Error: {:?}", e))
            }
            Self::Qr(e) => {
                HttpResponse::InternalServerError().json(format!("Qr Code Error: {:?}", e))
            }
            Self::Io(e) => HttpResponse::InternalServerError().json(format!("IO Error: {:?}", e)),
            Self::User(data) => HttpResponse::InternalServerError().json(data),
        }
    }
}

impl From<std::env::VarError> for ServerError {
    fn from(e: std::env::VarError) -> Self {
        eprintln!("Environment error {:?}", e);
        Self::Environment
    }
}

impl From<sqlx::Error> for ServerError {
    fn from(err: sqlx::Error) -> Self {
        eprintln!("Database error {:?}", err);
        Self::Database(err)
    }
}
impl From<sqlx::migrate::MigrateError> for ServerError {
    fn from(err: sqlx::migrate::MigrateError) -> Self {
        eprintln!("Database error {:?}", err);
        Self::DatabaseMigration(err)
    }
}

impl From<argonautica::Error> for ServerError {
    fn from(e: argonautica::Error) -> Self {
        eprintln!("Authentication error {:?}", e);
        Self::Argonautic
    }
}
impl From<tera::Error> for ServerError {
    fn from(e: tera::Error) -> Self {
        eprintln!("Template error {:?}", e);
        Self::Template(e)
    }
}
impl From<QrError> for ServerError {
    fn from(e: QrError) -> Self {
        eprintln!("Template error {:?}", e);
        Self::Qr(e)
    }
}
impl From<std::io::Error> for ServerError {
    fn from(e: std::io::Error) -> Self {
        eprintln!("IO error {:?}", e);
        Self::Io(e)
    }
}

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

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub secret: String,
    pub db: PathBuf,
    pub db_pool: Pool<Sqlite>,
    pub public_url: String,
    pub internal_ip: String,
    pub port: u32,
    pub protocol: Protocol,
    pub log: slog::Logger,
    pub empty_forward_url: String,
    pub brand_name: String,
}

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
            format!("PSLINK_SECRET=\"{}\"\n", self.secret),
        ]
    }
}
