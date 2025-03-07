//! This crate contains all shared fullstack server functions.
#[cfg(feature = "server")]
mod auth;
#[cfg(feature = "server")]
mod models;

pub mod auth_api;
pub mod link_api;
#[cfg(feature = "server")]
pub mod redirect_links;
pub mod user_api;

#[cfg(feature = "server")]
use pslink_shared::datatypes::Secret;
#[cfg(feature = "server")]
use tokio::sync::OnceCell;

#[cfg(feature = "server")]
use std::sync::LazyLock;
#[cfg(feature = "server")]

static DB: LazyLock<OnceCell<sqlx::SqlitePool>> = LazyLock::new(|| OnceCell::new());
#[cfg(feature = "server")]
pub(crate) async fn get_db() -> sqlx::SqlitePool {
    use dioxus::logger::tracing::info;

    println!("Getting DB");
    let db = match DB
        .get_or_try_init(|| sqlx::SqlitePool::connect("./links.db"))
        .await
    {
        Ok(db) => db.clone(),
        Err(_) => DB
            .get_or_try_init(|| sqlx::SqlitePool::connect("../links.db"))
            .await
            .expect("Could not connect to Database, tried: ./links.db and ../links.db")
            .clone(),
    };
    info!("Connected to Database");
    db
}

#[cfg(feature = "server")]
static SECRET: LazyLock<once_cell::sync::OnceCell<Secret>> =
    LazyLock::new(|| once_cell::sync::OnceCell::new());
#[cfg(feature = "server")]
pub(crate) fn get_secret() -> Secret {
    println!("Getting Secret");
    let db = SECRET
        .get_or_init(|| Secret::new(std::env::var("PSLINK_SECRET").unwrap()))
        .clone();
    db
}

#[cfg(feature = "server")]
pub fn launch_server(app: fn() -> Result<dioxus::prelude::VNode, dioxus::prelude::RenderError>) {
    info!("Starte den Server");
    use axum::routing::*;
    use axum_session::SessionConfig;
    use axum_session::SessionStore;
    use axum_session_auth::AuthConfig;
    use axum_session_auth::SessionSqlitePool;
    use dioxus::logger::tracing::info;
    use dioxus::prelude::DioxusRouterExt;
    use dioxus_fullstack::ServeConfig;
    use pslink_shared::datatypes::Secret;

    //simple_logger::SimpleLogger::new().init().unwrap();
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async move {
            let pool = get_db().await;

            //This Defaults as normal Cookies.
            //To enable Private cookies for integrity, and authenticity please check the next Example.
            let session_config = SessionConfig::default().with_table_name("test_table");
            let auth_config = AuthConfig::<i64>::default().with_anonymous_user_id(Some(0));
            let session_store =
                SessionStore::<SessionSqlitePool>::new(Some(pool.clone().into()), session_config)
                    .await
                    .unwrap();

            //User::create_user_tables(&pool).await;

            let admin = Router::new().serve_dioxus_application(ServeConfig::new().unwrap(), app);

            // build our application with some routes
            let axum_route = Router::new()
                // Server side render the application, serve static assets, and register server functions
                .nest("/app/", admin)
                .route("/:data", get(redirect_links::redirect))
                .route("/", get(redirect_links::redirect_empty))
                .layer(
                    axum_session_auth::AuthSessionLayer::<
                        auth::AuthAccount,
                        i64,
                        axum_session_auth::SessionSqlitePool,
                        sqlx::SqlitePool,
                    >::new(Some(pool.clone()))
                    .with_config(auth_config),
                )
                .layer(axum_session::SessionLayer::new(session_store))
                .with_state(pool)
                .with_state(get_secret());
            info!("Server configured");

            // run it
            // serve the app using the address passed by the CLI
            let addr = dioxus::cli_config::fullstack_address_or_localhost();
            let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

            axum::serve(listener, axum_route.into_make_service())
                .await
                .unwrap();
        });
}
