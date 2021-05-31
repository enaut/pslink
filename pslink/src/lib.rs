extern crate sqlx;

pub mod forms;
pub mod models;
pub mod queries;
mod views;

use actix_files::Files;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::HttpResponse;
use actix_web::{web, App, HttpServer};
use fluent_templates::{static_loader, FluentLoader};
use qrcode::types::QrError;
use shared::datatypes::Secret;
use sqlx::{Pool, Sqlite};
use std::{fmt::Display, path::PathBuf, str::FromStr};
use tera::Tera;
use thiserror::Error;
use tracing::instrument;
use tracing::{error, info, trace};
use tracing_actix_web::TracingLogger;

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
    #[error("The templates could not be rendered correctly: {0}")]
    Template(#[from] tera::Error),
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
            Self::Template(e) => {
                eprintln!("Template Error happened: {:?}", e);
                HttpResponse::InternalServerError().body(&Self::render_error(
                    "Server Error",
                    "The templates could not be rendered.",
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

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

static_loader! {
    static LOCALES = {
        locales: "./locales",
        fallback_language: "en",
    };
}

#[instrument]
fn build_tera() -> Result<Tera, ServerError> {
    let mut tera = Tera::default();

    // Add translation support
    tera.register_function("fluent", FluentLoader::new(&*LOCALES));

    tera.add_raw_templates(vec![
        ("admin.html", include_str!("../templates/admin.html")),
        ("base.html", include_str!("../templates/base.html")),
        (
            "edit_link.html",
            include_str!("../templates/edit_link.html"),
        ),
        (
            "edit_profile.html",
            include_str!("../templates/edit_profile.html"),
        ),
        (
            "index_users.html",
            include_str!("../templates/index_users.html"),
        ),
        ("index.html", include_str!("../templates/index.html")),
        ("login.html", include_str!("../templates/login.html")),
        (
            "not_found.html",
            include_str!("../templates/not_found.html"),
        ),
        ("signup.html", include_str!("../templates/signup.html")),
        (
            "submission.html",
            include_str!("../templates/submission.html"),
        ),
        (
            "view_link.html",
            include_str!("../templates/view_link.html"),
        ),
        (
            "view_profile.html",
            include_str!("../templates/view_profile.html"),
        ),
    ])?;
    Ok(tera)
}

/// Launch the pslink-webservice
///
/// # Errors
/// This produces a [`ServerError`] if:
///   * Tera failed to build its templates
///   * The server failed to bind to the designated port.
#[allow(clippy::future_not_send, clippy::too_many_lines)]
pub async fn webservice(
    server_config: ServerConfig,
) -> Result<actix_web::dev::Server, std::io::Error> {
    let host_port = format!("{}:{}", &server_config.internal_ip, &server_config.port);
    info!(
        "Running on: {}://{}/admin/login/",
        &server_config.protocol, host_port
    );
    info!(
        "If the public url is set up correctly it should be accessible via: {}://{}/admin/login/",
        &server_config.protocol, &server_config.public_url
    );
    let tera = build_tera().expect("Failed to build Templates");
    trace!("The tera templates are ready");

    let server = HttpServer::new(move || {
        let generated = generate();
        App::new()
            .data(server_config.clone())
            .wrap(TracingLogger)
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(&[0; 32])
                    .name("auth-cookie")
                    .secure(true),
            ))
            .data(tera.clone())
            .service(actix_web_static_files::ResourceFiles::new(
                "/static", generated,
            ))
            // directly go to the main page set the target with the environment variable.
            .route("/", web::get().to(views::redirect_empty))
            // admin block
            .service(
                web::scope("/admin")
                    // list all links
                    .route("/index/", web::get().to(views::index))
                    // invite users
                    .route("/signup/", web::get().to(views::signup))
                    .route("/signup/", web::post().to(views::process_signup))
                    // logout
                    .route("/logout/", web::to(views::logout))
                    // submit a new url for shortening
                    .route("/submit/", web::get().to(views::create_link))
                    .route("/submit/", web::post().to(views::process_link_creation))
                    // view an existing url
                    .service(
                        web::scope("/view")
                            .service(
                                web::scope("/link")
                                    .route("/{redirect_id}", web::get().to(views::view_link))
                                    .route("/", web::get().to(views::view_link_empty)),
                            )
                            .service(
                                web::scope("/profile")
                                    .route("/{user_id}", web::get().to(views::view_profile)),
                            )
                            .route("/users/", web::get().to(views::index_users)),
                    )
                    .service(
                        web::scope("/edit")
                            .service(
                                web::scope("/link")
                                    .route("/{redirect_id}", web::get().to(views::edit_link))
                                    .route(
                                        "/{redirect_id}",
                                        web::post().to(views::process_link_edit),
                                    ),
                            )
                            .service(
                                web::scope("/profile")
                                    .route("/{user_id}", web::get().to(views::edit_profile))
                                    .route(
                                        "/{user_id}",
                                        web::post().to(views::process_edit_profile),
                                    ),
                            )
                            .route("/set_admin/{user_id}", web::get().to(views::toggle_admin))
                            .route(
                                "/set_language/{language}",
                                web::get().to(views::set_language),
                            ),
                    )
                    .service(
                        web::scope("/delete").service(
                            web::scope("/link")
                                .route("/{redirect_id}", web::get().to(views::process_link_delete)),
                        ),
                    )
                    .service(
                        web::scope("/download")
                            .route("/png/{redirect_id}", web::get().to(views::download_png)),
                    )
                    .service(
                        web::scope("/json")
                            .route("/list_links/", web::post().to(views::index_json))
                            .route(
                                "/create_link/",
                                web::post().to(views::process_create_link_json),
                            )
                            .route("/get_qr_code/", web::post().to(views::get_qr_code_json))
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
                            .route(
                                "/get_logged_user/",
                                web::post().to(views::get_logged_user_json),
                            )
                            .route("/login_user/", web::post().to(views::process_login_json)),
                    )
                    // login to the admin area
                    .route("/login/", web::get().to(views::login))
                    .route("/login/", web::post().to(views::process_login)),
            )
            .service(
                web::scope("/app")
                    .service(Files::new("/pkg", "./app/pkg"))
                    .default_service(web::get().to(views::wasm_app)),
            )
            // redirect to the url hidden behind the code
            .route("/{redirect_id}", web::get().to(views::redirect))
    })
    .bind(host_port)
    .map_err(|e| {
        error!("Failed to bind to port!");
        e
    })?
    .run();
    Ok(server)
}
