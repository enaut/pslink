extern crate sqlx;

pub mod models;
pub mod queries;
mod views;

use actix_files::Files;
use actix_identity::IdentityMiddleware;
use actix_session::SessionMiddleware;
use actix_web::cookie::Key;
use actix_web::web::Data;
use actix_web::{web, App, HttpServer};
use fluent_templates::static_loader;
use qrcode::types::QrError;
use shared::datatypes::Secret;
use sqlx::{Pool, Sqlite};
use std::{fmt::Display, path::PathBuf, str::FromStr};
use thiserror::Error;
use tracing::{error, info};
use tracing_actix_web::TracingLogger;

/// The Error type that is returned by most function calls if anything failed.
#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Failed to encrypt the password {0} - aborting!")]
    Password(argon2::password_hash::Error),
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

impl From<argon2::password_hash::Error> for ServerError {
    fn from(e: argon2::password_hash::Error) -> Self {
        Self::Password(e)
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
    /*     fn error_response2(&self) -> HttpResponse {
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
    } */

    fn status_code(&self) -> actix_web::http::StatusCode {
        actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
    }

    /*     fn error_response(&self) -> actix_web::BaseHttpResponse<actix_web::body::Body> {
        let mut resp = actix_web::BaseHttpResponse::new(self.status_code());
        let mut buf = web::BytesMut::new();
        let _ = write!(Writer(&mut buf), "{}", self);
        resp.headers_mut().insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("text/plain; charset=utf-8"),
        );
        resp.set_body(actix_web::body::Body::from(buf))
    } */
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

// include the static files into the binary
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

static_loader! {
    static LOCALES = {
        locales: "./locales",
        fallback_language: "en",
    };
}

/// Launch the pslink-webservice
///
/// # Errors
/// This produces a [`ServerError`] if:
///   * The server failed to bind to the designated port.
#[allow(
    clippy::future_not_send,
    clippy::too_many_lines,
    unknown_lints,
    clippy::unused_async
)]
pub async fn webservice(
    server_config: ServerConfig,
) -> Result<actix_web::dev::Server, std::io::Error> {
    let host_port = format!("{}:{}", &server_config.internal_ip, &server_config.port);
    info!(
        "Running on: {}://{}/app/",
        &server_config.protocol, host_port
    );
    info!(
        "If the public url is set up correctly it should be accessible via: {}://{}/app/",
        &server_config.protocol, &server_config.public_url
    );
    let cookie_secret = Key::generate();

    let server = HttpServer::new(move || {
        let generated = generate();
        let store = actix_session::storage::CookieSessionStore::default();
        let session_mw = SessionMiddleware::builder(store, cookie_secret.clone())
            // disable secure cookie for local testing
            .cookie_secure(false)
            .cookie_http_only(false)
            .build();
        App::new()
            .app_data(Data::new(server_config.clone()))
            .wrap(TracingLogger::default())
            .wrap(session_mw)
            .wrap(IdentityMiddleware::default())
            .service(actix_web_static_files::ResourceFiles::new(
                "/static", generated,
            ))
            // directly go to the main page set the target with the environment variable.
            .route("/", web::get().to(views::redirect_empty))
            // admin block
            .service(
                web::scope("/admin")
                    .route("/logout/", web::to(views::logout))
                    .service(
                        web::scope("/download")
                            .route("/png/{redirect_id}", web::get().to(views::download_png)),
                    )
                    .service(
                        web::scope("/json")
                            .route("/list_links/", web::post().to(views::index_json))
                            .route("/get_language/", web::get().to(views::get_language))
                            .route("/change_language/", web::post().to(views::set_language))
                            .route(
                                "/create_link/",
                                web::post().to(views::process_create_link_json),
                            )
                            .route(
                                "/edit_link/",
                                web::post().to(views::process_update_link_json),
                            )
                            .route(
                                "/delete_link/",
                                web::post().to(views::process_delete_link_json),
                            )
                            .route("/list_users/", web::post().to(views::index_users_json))
                            .route(
                                "/create_user/",
                                web::post().to(views::process_create_user_json),
                            )
                            .route(
                                "/update_user/",
                                web::post().to(views::process_update_user_json),
                            )
                            .route("/update_privileges/", web::post().to(views::toggle_admin))
                            .route(
                                "/get_logged_user/",
                                web::post().to(views::get_logged_user_json),
                            )
                            .route(
                                "/get_link_statistics/",
                                web::post().to(views::get_statistics),
                            )
                            .route("/login_user/", web::post().to(views::process_login_json)),
                    ),
            )
            // Serve the Wasm App for the admin interface.
            .service(
                web::scope("/app")
                    .service(Files::new("/pkg", "./app/pkg"))
                    .default_service(web::get().to(views::wasm_app)),
            )
            // redirect to the url hidden behind the code
            .route("/{redirect_id}", web::get().to(views::redirect))
    })
    .bind(host_port)
    .inspect_err(|_| {
        error!("Failed to bind to port!");
    })?
    .run();
    Ok(server)
}
