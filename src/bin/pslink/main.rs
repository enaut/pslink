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
mod views;
use pslink::ServerConfig;

use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{web, App, HttpServer};

use fluent_templates::{static_loader, FluentLoader};
use tera::Tera;

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
        ("admin.html", include_str!("../../../templates/admin.html")),
        ("base.html", include_str!("../../../templates/base.html")),
        (
            "edit_link.html",
            include_str!("../../../templates/edit_link.html"),
        ),
        (
            "edit_profile.html",
            include_str!("../../../templates/edit_profile.html"),
        ),
        (
            "index_users.html",
            include_str!("../../../templates/index_users.html"),
        ),
        ("index.html", include_str!("../../../templates/index.html")),
        ("login.html", include_str!("../../../templates/login.html")),
        (
            "not_found.html",
            include_str!("../../../templates/not_found.html"),
        ),
        (
            "signup.html",
            include_str!("../../../templates/signup.html"),
        ),
        (
            "submission.html",
            include_str!("../../../templates/submission.html"),
        ),
        (
            "view_link.html",
            include_str!("../../../templates/view_link.html"),
        ),
        (
            "view_profile.html",
            include_str!("../../../templates/view_profile.html"),
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