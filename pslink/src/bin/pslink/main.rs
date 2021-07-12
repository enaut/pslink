extern crate sqlx;

mod cli;
mod views;

use actix_files::Files;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{web, App, HttpServer};
use fluent_templates::static_loader;
use pslink::ServerConfig;

use tracing::instrument;
use tracing::{subscriber::set_global_default, Subscriber};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

use tracing::{error, info};
use tracing_actix_web::TracingLogger;

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

#[instrument]
#[actix_web::main]
async fn main() -> std::result::Result<(), std::io::Error> {
    let subscriber = get_subscriber("pslink", "info");
    init_subscriber(subscriber);

    match cli::setup().await {
        Ok(Some(server_config)) => {
            webservice(server_config)
                .await
                .map_err(|e| {
                    println!("{:?}", e);
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    std::process::exit(0);
                })
                .expect("Failed to launch the service")
                .await
        }
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

// include the static files into the binary
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

static_loader! {
    static LOCALES = {
        locales: "../locales",
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
    .map_err(|e| {
        error!("Failed to bind to port!");
        e
    })?
    .run();
    Ok(server)
}
