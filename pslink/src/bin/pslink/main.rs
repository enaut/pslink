extern crate sqlx;

mod cli;

use actix_files::Files;
use actix_identity::IdentityMiddleware;
use actix_session::SessionMiddleware;
use actix_web::{
    cookie::Key,
    middleware::Compat,
    web::{self, Data},
    App, HttpServer,
};
use pslink::{views, ServerConfig};

use opentelemetry::global;
use opentelemetry_appender_tracing::layer;
use opentelemetry_sdk::logs::LoggerProvider;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::TracerProvider;
use opentelemetry_sdk::Resource;
use opentelemetry_stdout::SpanExporter;
use tracing::{error, info, instrument};
use tracing_actix_web::TracingLogger;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

const APP_NAME: &str = "pslink";

fn init_tracer() {
    global::set_text_map_propagator(TraceContextPropagator::new());
    let provider = TracerProvider::builder()
        .with_simple_exporter(SpanExporter::default())
        .build();
    global::set_tracer_provider(provider);
}

#[instrument]
#[actix_web::main]
async fn main() -> std::result::Result<(), std::io::Error> {
    init_tracer();
    let _tracer = global::tracer(APP_NAME);
    let exporter = opentelemetry_stdout::LogExporter::default();
    let provider: LoggerProvider = LoggerProvider::builder()
        .with_resource(Resource::empty())
        .with_simple_exporter(exporter)
        .build();
    let layer = layer::OpenTelemetryTracingBridge::new(&provider);
    tracing_subscriber::registry()
        .with(layer)
        .with(tracing_subscriber::filter::LevelFilter::INFO)
        .init();

    match cli::setup().await {
        Ok(Some(server_config)) => {
            println!("Starting the server");
            let service = webservice(server_config.clone());
            println!("Service built");
            let server = service
                .await
                .map_err(|e| {
                    println!("{:?}", e);
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    std::process::exit(0);
                })
                .expect("Failed to launch the service");
            println!("Server started");
            println!(
                "Server running on: http://{}:{}",
                server_config.internal_ip, server_config.port
            );
            println!(
                "Log in at: http://{}:{}/app/",
                server_config.internal_ip, server_config.port
            );
            server.await
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

/// Launch the pslink-web-service
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

    let secret = Key::generate();
    let server = HttpServer::new(move || {
        let cookie_store = actix_session::storage::CookieSessionStore::default();
        let store_middleware = SessionMiddleware::builder(cookie_store, secret.clone())
            .cookie_content_security(actix_session::config::CookieContentSecurity::Private)
            .cookie_name("pslink-session".to_string())
            .cookie_path("/".to_owned())
            .build();
        let generated = generate();
        let logger = Compat::new(TracingLogger::default());
        App::new()
            .app_data(Data::new(server_config.clone()))
            .wrap(logger)
            .wrap(IdentityMiddleware::default())
            .wrap(store_middleware)
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
                    )
                    .default_service(web::to(views::to_admin)),
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
