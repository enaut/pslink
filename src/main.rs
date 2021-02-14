#[macro_use]
extern crate diesel;
#[macro_use]
extern crate log;
mod forms;
pub mod models;
mod queries;
pub mod schema;
mod views;

use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpResponse, HttpServer};
use diesel::prelude::*;

use dotenv::dotenv;
use models::NewUser;
use qrcode::types::QrError;
use tera::Tera;

#[derive(Debug)]
pub enum ServerError {
    Argonautic,
    Diesel(diesel::result::Error),
    Environment,
    Template(tera::Error),
    Qr(QrError),
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
            Self::Argonautic => HttpResponse::InternalServerError().json("Argonautica Error."),
            Self::Diesel(e) => {
                HttpResponse::InternalServerError().json(format!("Diesel Error.{}", e))
            }
            Self::Environment => HttpResponse::InternalServerError().json("Environment Error."),
            Self::Template(e) => {
                HttpResponse::InternalServerError().json(format!("Template Error. {:?}", e))
            }
            Self::Qr(e) => {
                HttpResponse::InternalServerError().json(format!("Qr Code Error. {:?}", e))
            }
            Self::User(data) => HttpResponse::InternalServerError().json(data),
        }
    }
}

impl From<std::env::VarError> for ServerError {
    fn from(e: std::env::VarError) -> Self {
        error!("Environment error {:?}", e);
        Self::Environment
    }
}

/* impl From<r2d2::Error> for ServerError {
    fn from(_: r2d2::Error) -> ServerError {
        ServerError::R2D2Error
    }
} */

impl From<diesel::result::Error> for ServerError {
    fn from(err: diesel::result::Error) -> Self {
        error!("Database error {:?}", err);
        Self::Diesel(err)
    }
}

impl From<argonautica::Error> for ServerError {
    fn from(e: argonautica::Error) -> Self {
        error!("Authentication error {:?}", e);
        Self::Argonautic
    }
}
impl From<tera::Error> for ServerError {
    fn from(e: tera::Error) -> Self {
        error!("Template error {:?}", e);
        Self::Template(e)
    }
}
impl From<QrError> for ServerError {
    fn from(e: QrError) -> Self {
        error!("Template error {:?}", e);
        Self::Qr(e)
    }
}

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let connection = queries::establish_connection().expect("Failed to connect to database!");
    let num_users: i64 = schema::users::dsl::users
        .select(diesel::dsl::count_star())
        .first(&connection)
        .expect("Failed to count the users");

    if num_users < 1 {
        // It is ok to use expect in this block since it is only run on the start. And if something fails it is probably something major.
        use schema::users;
        use std::io::{self, BufRead, Write};
        warn!("No usere available Creating one!");
        let sin = io::stdin();

        print!("Please enter the Username of the admin: ");
        io::stdout().flush().unwrap();
        let username = sin.lock().lines().next().unwrap().unwrap();

        print!("Please enter the emailadress for {}: ", username);
        io::stdout().flush().unwrap();
        let email = sin.lock().lines().next().unwrap().unwrap();

        print!("Please enter the password for {}: ", username);
        io::stdout().flush().unwrap();
        let password = sin.lock().lines().next().unwrap().unwrap();
        println!(
            "Creating {} ({}) with password {}",
            &username, &email, &password
        );

        let new_admin =
            NewUser::new(username, email, password).expect("Invalid Input failed to create User");

        diesel::insert_into(users::table)
            .values(&new_admin)
            .execute(&connection)
            .expect("Failed to create the user!");
    }

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
                                    .route("/{redirect_id}", web::get().to(views::view_link))
                                    .route("/", web::get().to(views::view_link_fhs)),
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
                            .route("/set_admin/{user_id}", web::get().to(views::toggle_admin)),
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
