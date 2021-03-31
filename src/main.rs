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

mod cli;
mod forms;
pub mod models;
mod queries;
mod views;

use std::{fmt::Display, path::PathBuf, str::FromStr};

use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{web, App, HttpResponse, HttpServer};

use fluent_templates::{static_loader, FluentLoader};
use qrcode::types::QrError;
use sqlx::{Pool, Sqlite};
use tera::Tera;

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
                HttpResponse::InternalServerError().json(format!("Diesel Error: {:?}", e))
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
enum Protocol {
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
pub(crate) struct ServerConfig {
    secret: String,
    db: PathBuf,
    db_pool: Pool<Sqlite>,
    public_url: String,
    internal_ip: String,
    port: u32,
    protocol: Protocol,
    log: slog::Logger,
    empty_forward_url: String,
    brand_name: String,
}

impl ServerConfig {
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

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

static_loader! {
    static LOCALES = {
        locales: "./locales",
        fallback_language: "en",
    };
}

fn build_tera() -> Tera {
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
    ])
    .expect("failed to parse templates");
    tera
}

#[allow(clippy::future_not_send)]
async fn webservice(server_config: ServerConfig) -> std::io::Result<()> {
    let host_port = format!("{}:{}", &server_config.internal_ip, &server_config.port);

    slog_info!(
        server_config.log,
        "Running on: {}://{}/admin/login/",
        &server_config.protocol,
        host_port
    );
    slog_info!(
        server_config.log,
        "If the public url is set up correctly it should be accessible via: {}://{}/admin/login/",
        &server_config.protocol,
        &server_config.public_url
    );

    HttpServer::new(move || {
        let tera = build_tera(); //Tera::new("templates/**/*").expect("failed to initialize the templates");
        let generated = generate();
        App::new()
            .data(server_config.clone())
            .wrap(actix_slog::StructuredLogger::new(
                server_config.log.new(slog_o!("log_type" => "access")),
            ))
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(&[0; 32])
                    .name("auth-cookie")
                    .secure(false),
            ))
            .data(tera)
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
                    // login to the admin area
                    .route("/login/", web::get().to(views::login))
                    .route("/login/", web::post().to(views::process_login)),
            )
            // redirect to the url hidden behind the code
            .route("/{redirect_id}", web::get().to(views::redirect))
    })
    .bind(host_port)?
    .run()
    .await
}

#[actix_web::main]
async fn main() -> Result<(), std::io::Error> {
    match cli::setup().await {
        Ok(Some(server_config)) => webservice(server_config).await,
        Ok(None) => {
            std::thread::sleep(std::time::Duration::from_millis(100));
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("\nError: {}", e);
            std::thread::sleep(std::time::Duration::from_millis(100));
            std::process::exit(1);
        }
    }
}
