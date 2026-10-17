//! Server runtime initialization and Axum router setup.

pub mod auth;
pub mod checker;
pub mod cli;
pub mod db;
pub mod redirect;
pub mod state;

use dioxus::{logger::tracing::info, prelude::ServerFnError};

pub(crate) trait ServerResultExt<T> {
    fn sfe(self) -> Result<T, ServerFnError>;
}

impl<T, E: std::fmt::Display> ServerResultExt<T> for Result<T, E> {
    fn sfe(self) -> Result<T, ServerFnError> {
        self.map_err(ServerFnError::new)
    }
}

pub fn launch_pslink(app: fn() -> Result<dioxus::prelude::VNode, dioxus::prelude::RenderError>) {
    dioxus::logger::init(dioxus::logger::tracing::Level::INFO).unwrap();

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async move {
            match cli::setup().await {
                Ok(Some(app_state)) => {
                    println!("Starting the server");
                    let ip = app_state.config.internal_ip.clone();
                    let port = app_state.config.port;
                    let server = launch_server(app, app_state);
                    println!("Server started");

                    println!(
                        "Pslink server started:\n\tLog in at: http://{}:{}/app/",
                        std::env::var("DIOXUS_DEVSERVER_IP").unwrap_or(ip),
                        std::env::var("DIOXUS_DEVSERVER_PORT").unwrap_or_else(|_| port.to_string())
                    );
                    tokio::select! {
                        _ = tokio::signal::ctrl_c() => {
                            println!("Received Ctrl+C, shutting down server...");
                        }
                        _ = server => {
                            println!("Server stopped");
                        }
                    }
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
        });
}

async fn launch_server(
    app: fn() -> Result<dioxus::prelude::VNode, dioxus::prelude::RenderError>,
    app_state: state::AppState,
) {
    info!("Starte den Server");
    use axum::{Extension, Router, routing::get};
    use axum_session::{Key, SessionConfig, SessionStore};
    use axum_session_auth::AuthConfig;
    use dioxus::prelude::DioxusRouterExt;
    use dioxus::server::ServeConfig;

    let pool = app_state.db.clone();
    checker::spawn_periodic_checker(app_state.clone());
    let bind_addr = format!("{}:{}", app_state.config.internal_ip, app_state.config.port);

    // Derive a 64-byte cookie key from the configured server secret
    let secret_bytes = app_state
        .config
        .secret
        .secret
        .as_deref()
        .unwrap_or("default-secret")
        .as_bytes();
    let mut key_material = [0u8; 64];
    for (i, b) in key_material.iter_mut().enumerate() {
        *b = secret_bytes[i % secret_bytes.len()].wrapping_add(i as u8);
    }
    let session_config = SessionConfig::default()
        .with_table_name("sessions")
        .with_key(Key::from(&key_material));

    let auth_config = AuthConfig::<i64>::default().with_anonymous_user_id(Some(0));
    let session_store = SessionStore::<axum_session_sqlx::SessionSqlitePool>::new(
        Some(pool.clone().into()),
        session_config,
    )
    .await
    .unwrap();

    let admin = Router::new().serve_dioxus_application(ServeConfig::default(), app);

    let axum_route = Router::new()
        .nest("/app/", admin)
        .route("/{data}", get(redirect::redirect))
        .route("/", get(redirect::redirect_empty))
        .layer(
            axum_session_auth::AuthSessionLayer::<
                auth::AuthAccount,
                i64,
                axum_session_sqlx::SessionSqlitePool,
                sqlx::SqlitePool,
            >::new(Some(pool))
            .with_config(auth_config),
        )
        .layer(axum_session::SessionLayer::new(session_store))
        .layer(Extension(app_state));
    info!("Server configured");

    let listener = tokio::net::TcpListener::bind(bind_addr).await.unwrap();

    axum::serve(listener, axum_route.into_make_service())
        .await
        .unwrap();
}
