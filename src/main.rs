#[macro_use]
extern crate diesel;
#[macro_use]
extern crate log;
mod forms;
pub mod models;
pub mod schema;
mod views;

use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpResponse, HttpServer};

use dotenv::dotenv;
use tera::Tera;

#[derive(Debug)]
pub enum ServerError {
    Argonautic,
    Diesel,
    Environment,
    Template(tera::Error),
    User(String),
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Test")
    }
}

impl actix_web::error::ResponseError for ServerError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ServerError::Argonautic => {
                HttpResponse::InternalServerError().json("Argonautica Error.")
            }
            ServerError::Diesel => HttpResponse::InternalServerError().json("Diesel Error."),
            ServerError::Environment => {
                HttpResponse::InternalServerError().json("Environment Error.")
            }
            ServerError::Template(e) => {
                HttpResponse::InternalServerError().json(format!("Template Error. {:?}", e))
            }
            ServerError::User(data) => HttpResponse::InternalServerError().json(data),
        }
    }
}

impl From<std::env::VarError> for ServerError {
    fn from(e: std::env::VarError) -> ServerError {
        error!("Environment error {:?}", e);
        ServerError::Environment
    }
}

/* impl From<r2d2::Error> for ServerError {
    fn from(_: r2d2::Error) -> ServerError {
        ServerError::R2D2Error
    }
} */

impl From<diesel::result::Error> for ServerError {
    fn from(err: diesel::result::Error) -> ServerError {
        error!("Database error {:?}", err);
        match err {
            diesel::result::Error::NotFound => ServerError::User("Username not found.".to_string()),
            _ => ServerError::Diesel,
        }
    }
}

impl From<argonautica::Error> for ServerError {
    fn from(e: argonautica::Error) -> ServerError {
        error!("Authentication error {:?}", e);
        ServerError::Argonautic
    }
}
impl From<tera::Error> for ServerError {
    fn from(e: tera::Error) -> ServerError {
        error!("Template error {:?}", e);
        ServerError::Template(e)
    }
}

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    println!("Running on: http://127.0.0.1:8156/admin/login/");
    HttpServer::new(|| {
        let tera = Tera::new("templates/**/*").expect("failed to initialize the templates");
        let generated = generate();
        App::new()
            .wrap(Logger::default())
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(&[0; 32])
                    .name("auth-cookie")
                    .secure(false),
            ))
            .data(tera)
            .service(actix_web_static_files::ResourceFiles::new(
                "/static", generated,
            ))
            // directly go to the main page of Freie-Hochschule-Stuttgart
            .route("/", web::get().to(views::redirect_fhs))
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
                                    .route("/{redirect_id}", web::get().to(views::view_link)),
                            )
                            .service(
                                web::scope("/profile")
                                    .route("/{user_id}", web::get().to(views::view_profile)),
                            ),
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
    .bind("127.0.0.1:8156")?
    .run()
    .await
}
