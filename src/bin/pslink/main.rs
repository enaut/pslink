extern crate sqlx;

mod cli;

use pslink::ServerConfig;

use tracing::instrument;
use tracing::{subscriber::set_global_default, Subscriber};
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

#[instrument]
#[actix_web::main]
async fn main() -> std::result::Result<(), std::io::Error> {
    let subscriber = get_subscriber("fhs.li", "info");
    init_subscriber(subscriber);

    match cli::setup().await {
        Ok(Some(server_config)) => {
            pslink::webservice(server_config)
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
