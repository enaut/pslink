extern crate sqlx;

mod cli;
mod views;

use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{web, App, HttpServer};
use anyhow::{Context, Result};
use fluent_templates::{static_loader, FluentLoader};
use tera::Tera;

use pslink::{ServerConfig, ServerError};

use tracing::instrument;
use tracing::{error, info, trace};
use tracing::{subscriber::set_global_default, Subscriber};
use tracing_actix_web::TracingLogger;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

/// Compose multiple layers into a `tracing`'s subscriber.
#[must_use]
pub fn get_subscriber(name: &str, env_filter: &str) -> impl Subscriber + Send + Sync {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    // Create a jaeger exporter pipeline for a `trace_demo` service.
    let tracer = opentelemetry_jaeger::new_pipeline()
        .with_service_name(name)
        .install_simple()
        .expect("Error initializing Jaeger exporter");
    let formatting_layer = tracing_subscriber::fmt::layer().with_target(false);

    // Create a layer with the configured tracer
    let otel_layer = OpenTelemetryLayer::new(tracer);

    // Use the tracing subscriber `Registry`, or any other subscriber
    // that impls `LookupSpan`
    Registry::default()
        .with(otel_layer)
        .with(env_filter)
        .with(formatting_layer)
}

/// Register a subscriber as global default to process span data.
///
/// It should only be called once!
pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    set_global_default(subscriber).expect("Failed to set subscriber");
}

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

static_loader! {
    static LOCALES = {
        locales: "./locales",
        fallback_language: "en",
    };
}

#[instrument]
fn build_tera() -> Result<Tera> {
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
    .context("Failed to load Templates")?;
    Ok(tera)
}

#[allow(clippy::future_not_send, clippy::too_many_lines)]
async fn webservice(server_config: ServerConfig) -> Result<()> {
    let host_port = format!("{}:{}", &server_config.internal_ip, &server_config.port);
    info!(
        "Running on: {}://{}/admin/login/",
        &server_config.protocol, host_port
    );
    info!(
        "If the public url is set up correctly it should be accessible via: {}://{}/admin/login/",
        &server_config.protocol, &server_config.public_url
    );
    let tera = build_tera()?;
    trace!("The tera templates are ready");

    HttpServer::new(move || {
        let generated = generate();
        App::new()
            .data(server_config.clone())
            .wrap(TracingLogger)
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(&[0; 32])
                    .name("auth-cookie")
                    .secure(false),
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
                    // login to the admin area
                    .route("/login/", web::get().to(views::login))
                    .route("/login/", web::post().to(views::process_login)),
            )
            // redirect to the url hidden behind the code
            .route("/{redirect_id}", web::get().to(views::redirect))
    })
    .bind(host_port)
    .context("Failed to bind to port")
    .map_err(|e| {
        error!("Failed to bind to port!");
        e
    })?
    .run()
    .await
    .context("Failed to run the webservice")
}

#[instrument]
#[actix_web::main]
async fn main() -> std::result::Result<(), ServerError> {
    let subscriber = get_subscriber("fhs.li", "info");
    init_subscriber(subscriber);

    match cli::setup().await {
        Ok(Some(server_config)) => webservice(server_config).await.map_err(|e| {
            println!("{:?}", e);
            std::thread::sleep(std::time::Duration::from_millis(100));
            std::process::exit(0);
        }),
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
