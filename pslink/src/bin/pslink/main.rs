extern crate sqlx;

mod cli;

use std::sync::LazyLock;

use opentelemetry_otlp::WithExportConfig as _;
use pslink::ServerConfig;

use opentelemetry::trace::TracerProvider;
use tracing::instrument;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

const APP_NAME: &str = "pslink";

static RESOURCE: LazyLock<opentelemetry_sdk::Resource> = LazyLock::new(|| {
    opentelemetry_sdk::Resource::new(vec![opentelemetry::KeyValue::new(
        opentelemetry_semantic_conventions::resource::SERVICE_NAME,
        APP_NAME,
    )])
});

/// Compose multiple layers into a `tracing`'s subscriber.
fn init_telemetry() {
    // Start a new otlp trace pipeline.
    // Spans are exported in batch - recommended setup for a production application.
    opentelemetry::global::set_text_map_propagator(
        opentelemetry_sdk::propagation::TraceContextPropagator::new(),
    );
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://127.0.0.1:4317"),
        )
        .with_trace_config(
            opentelemetry_sdk::trace::Config::default().with_resource(RESOURCE.clone()),
        )
        .install_batch(opentelemetry_sdk::runtime::TokioCurrentThread)
        .expect("Failed to install OpenTelemetry tracer.")
        .tracer_builder(APP_NAME)
        .build();

    // Filter based on level - trace, debug, info, warn, error
    // Tunable via `RUST_LOG` env variable
    let env_filter = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("info"));
    // Create a `tracing` layer using the otlp tracer
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    // Create a `tracing` layer to emit spans as structured logs to stdout
    let formatting_layer =
        tracing_bunyan_formatter::BunyanFormattingLayer::new(APP_NAME.into(), std::io::stdout);
    // Combined them all together in a `tracing` subscriber
    let subscriber = Registry::default()
        .with(env_filter)
        .with(telemetry)
        .with(tracing_bunyan_formatter::JsonStorageLayer)
        .with(formatting_layer);
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to install `tracing` subscriber.")
}

#[instrument]
#[actix_web::main]
async fn main() -> std::result::Result<(), std::io::Error> {
    init_telemetry();

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
