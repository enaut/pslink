//! Command-line interface and server configuration management.

use std::{
    fmt::Display,
    fs::File,
    io::{self, BufRead, IsTerminal, Write},
    path::PathBuf,
    str::FromStr,
};

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use dioxus::{
    logger::tracing::{error, info, trace, warn},
    prelude::ServerFnError,
};
use dotenv::dotenv;
use sqlx::{
    SqlitePool,
    migrate::Migrator,
    sqlite::SqliteConnectOptions,
};

use crate::{
    domain::models::Secret,
    server::{
        ServerResultExt,
        db::{self, NewLink, NewUser},
        state::AppState,
    },
};

pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// The protocol used in generated short URLs and QR codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Protocol {
    Http,
    Https,
}

impl Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http => f.write_str("http"),
            Self::Https => f.write_str("https"),
        }
    }
}

impl FromStr for Protocol {
    type Err = ServerFnError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "http" => Ok(Self::Http),
            "https" => Ok(Self::Https),
            _ => Err(ServerFnError::new("Failed to parse Protocol".to_owned())),
        }
    }
}

/// Globally valid server configuration settings.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub db: PathBuf,
    pub public_url: String,
    pub empty_forward_url: String,
    pub brand_name: String,
    pub internal_ip: String,
    pub port: u16,
    pub protocol: Protocol,
    pub secret: Secret,
    pub data_download_secret: Secret,
    /// Hours between two sweeps that check all link targets. `0` disables the periodic check.
    pub check_interval_hours: u64,
    /// How many checks are kept per link.
    pub check_history: u32,
}

impl ServerConfig {
    /// Format the configured base URL (`{protocol}://{public_url}`).
    #[must_use]
    pub fn base_url(&self, fallback_host: Option<&str>) -> String {
        let host = if self.public_url.trim().is_empty() {
            fallback_host.unwrap_or("localhost:8080")
        } else {
            self.public_url.trim().trim_end_matches('/')
        };
        if host.starts_with("http://") || host.starts_with("https://") {
            host.to_string()
        } else {
            format!("{}://{}", self.protocol, host)
        }
    }

    /// Serialize the configuration into `.env` file lines.
    #[must_use]
    pub fn to_env_strings(&self) -> Vec<String> {
        vec![
            format!("PSLINK_DATABASE=\"{}\"\n", self.db.display()),
            format!("PSLINK_PORT={}\n", self.port),
            format!("PSLINK_PUBLIC_URL=\"{}\"\n", self.public_url),
            format!("PSLINK_EMPTY_FORWARD_URL=\"{}\"\n", self.empty_forward_url),
            format!("PSLINK_BRAND_NAME=\"{}\"\n", self.brand_name),
            format!("PSLINK_IP=\"{}\"\n", self.internal_ip),
            format!("PSLINK_PROTOCOL=\"{}\"\n", self.protocol),
            concat!(
                "# The PSLINK_SECRET variable is used for password encryption.\n",
                "# If it is changed all existing passwords are invalid.\n"
            )
            .to_owned(),
            format!(
                "PSLINK_SECRET=\"{}\"\n",
                self.secret
                    .secret
                    .as_ref()
                    .expect("A Secret was not specified!")
            ),
            concat!(
                "# The PSLINK_DATA_DOWNLOAD_SECRET variable is used for database export authentication.\n",
                "# Change this to a strong random value for security.\n"
            )
            .to_owned(),
            format!(
                "PSLINK_DATA_DOWNLOAD_SECRET=\"{}\"\n",
                self.data_download_secret
                    .secret
                    .as_ref()
                    .expect("A Data Download Secret was not specified!")
            ),
            format!("DEMO=\"{}\"\n", self.secret.is_random),
            "# Hours between two checks of all link targets (0 disables the periodic check).\n"
                .to_owned(),
            format!("PSLINK_CHECK_INTERVAL_HOURS={}\n", self.check_interval_hours),
        ]
    }
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about)]
struct CliArgs {
    /// The path of the sqlite database
    #[arg(
        long = "db",
        env = "PSLINK_DATABASE",
        default_value = "./links.db",
        global = true
    )]
    database: PathBuf,

    /// The port the pslink service will run on
    #[arg(short = 'p', long = "port", env = "PSLINK_PORT", global = true)]
    port: Option<u16>,

    /// The host url or the page that will be part of the short urls
    #[arg(
        short = 'u',
        long = "public-url",
        env = "PSLINK_PUBLIC_URL",
        default_value = "127.0.0.1:8080",
        global = true
    )]
    public_url: String,

    /// The url that / will redirect to when not overridden in the database
    #[arg(
        short = 'e',
        long = "empty-url",
        env = "PSLINK_EMPTY_FORWARD_URL",
        default_value = "https://github.com/enaut/pslink",
        global = true
    )]
    empty_forward_url: String,

    /// The Brandname that will appear in various places
    #[arg(
        short = 'b',
        long = "brand-name",
        env = "PSLINK_BRAND_NAME",
        default_value = "Pslink",
        global = true
    )]
    brand_name: String,

    /// The host (ip) that will run the pslink service
    #[arg(short = 'i', long = "hostip", env = "PSLINK_IP", global = true)]
    internal_ip: Option<String>,

    /// Whether this instance is running in ephemeral demo mode
    #[arg(long = "demo", env = "DEMO", default_value_t = false, global = true)]
    demo: bool,

    /// The protocol that is used in the qr-codes (http results in slightly smaller codes in some cases)
    #[arg(
        short = 't',
        long = "protocol",
        env = "PSLINK_PROTOCOL",
        default_value = "http",
        global = true
    )]
    protocol: Protocol,

    /// The secret that is used to encrypt the password database. Keep this as inaccessible as possible.
    #[arg(long = "secret", env = "PSLINK_SECRET", default_value = "", global = true)]
    secret: String,

    /// The secret that is used to authenticate database export requests.
    #[arg(
        long = "data-download-secret",
        env = "PSLINK_DATA_DOWNLOAD_SECRET",
        default_value = "",
        global = true
    )]
    data_download_secret: String,

    /// Hours between two checks of all link targets (0 disables the periodic check)
    #[arg(
        long = "check-interval-hours",
        env = "PSLINK_CHECK_INTERVAL_HOURS",
        default_value_t = 6,
        global = true
    )]
    check_interval_hours: u64,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
enum Commands {
    #[command(hide = true, about = "Run the server", display_order = 1)]
    Backend,
    #[command(about = "Run the server", display_order = 1)]
    Runserver,
    #[command(
        name = "migrate-database",
        about = "Apply any pending migrations and exit",
        display_order = 2
    )]
    MigrateDatabase,
    #[command(
        name = "generate-env",
        about = "Generate an .env file template using default settings and exit",
        display_order = 2
    )]
    GenerateEnv,
    #[command(
        name = "create-admin",
        about = "Create an admin user.",
        display_order = 2
    )]
    CreateAdmin,
    #[command(
        hide = true,
        about = "Create a database and demo user.",
        display_order = 3
    )]
    Demo,
}

impl CliArgs {
    fn into_server_config(self) -> ServerConfig {
        info!("Parsing the arguments");
        let secret = if self.secret.len() < 5 {
            if self.secret.is_empty() {
                warn!("No secret was found! Use the environment variable PSLINK_SECRET to set one.");
                warn!("If you change the secret all passwords will be invalid");
                warn!("Using an auto generated one for this run.");
            } else {
                warn!("The provided secret was too short. Using an auto generated one.");
            }
            Secret::random()
        } else {
            let mut secret = Secret::new(self.secret);
            secret.is_random = self.demo;
            secret
        };

        let data_download_secret = if self.data_download_secret.len() < 5 {
            if self.data_download_secret.is_empty() {
                warn!(
                    "No data download secret was found! Use the environment variable PSLINK_DATA_DOWNLOAD_SECRET to set one."
                );
                warn!("Using an auto generated one for this run.");
            } else {
                warn!(
                    "The provided data download secret was too short. Using an auto generated one."
                );
            }
            Secret::random()
        } else {
            Secret::new(self.data_download_secret)
        };

        let cli_ip = dioxus::cli_config::server_ip().map(|s| s.to_string());
        let internal_ip = self
            .internal_ip
            .or(cli_ip)
            .unwrap_or_else(|| "127.0.0.1".to_string());

        let cli_port = dioxus::cli_config::server_port();
        let port = self.port.or(cli_port).unwrap_or(8080);

        info!("Arguments parsed");
        ServerConfig {
            db: self.database,
            public_url: self.public_url,
            empty_forward_url: self.empty_forward_url,
            brand_name: self.brand_name,
            internal_ip,
            port,
            protocol: self.protocol,
            secret,
            data_download_secret,
            check_interval_hours: self.check_interval_hours,
            check_history: 20,
        }
    }
}

async fn connect_db(db_path: &std::path::Path, create_if_missing: bool) -> Result<SqlitePool, ServerFnError> {
    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(create_if_missing);
    let pool = SqlitePool::connect_with(options).await.sfe()?;
    info!("Connected to Database at {}", db_path.display());
    Ok(pool)
}

/// Setup and launch the CLI or return [`AppState`] if the server should start.
///
/// # Errors
/// Returns [`ServerFnError`] if CLI setup, migrations, or database connection fails.
pub async fn setup() -> Result<Option<AppState>, ServerFnError> {
    dotenv().ok();

    info!("Launching Pslink a 'Private short link generator'");
    let args = CliArgs::parse();
    let command = args.command.clone();
    let mut server_config = args.into_server_config();

    let Some(command) = command else {
        let dioxus_cli = std::env::var("DIOXUS_CLI_ENABLED")
            .ok()
            .and_then(|s| s.parse::<bool>().ok())
            .unwrap_or(false);
        if dioxus_cli {
            error!(
                "The Dioxus CLI is enabled, but no command was provided. Please use `--args=\"runserver\"` to start the server."
            );
        }
        println!("{}", CliArgs::command().render_usage());
        return Err(ServerFnError::new(
            "The command is missing try `runserver`".to_string(),
        ));
    };

    match command {
        Commands::GenerateEnv => {
            generate_env_file(&server_config)?;
            Ok(None)
        }
        Commands::MigrateDatabase => {
            let pool = apply_migrations(&server_config).await?;
            pool.close().await;
            info!("Successfully migrated the database.");
            Ok(None)
        }
        Commands::Demo => {
            generate_demo_data(&server_config).await?;
            Ok(None)
        }
        Commands::CreateAdmin => {
            if !server_config.db.exists() {
                let msg = format!(
                    "Database not found at {}! Create a new database with: `pslink migrate-database` or adjust the database path.",
                    server_config.db.display()
                );
                error!("{}", msg);
                eprintln!("{}", msg);
                return Ok(None);
            }
            let pool = connect_db(&server_config.db, false).await?;
            request_admin_credentials(&pool, &server_config).await?;
            pool.close().await;
            Ok(None)
        }
        Commands::Runserver | Commands::Backend => {
            let dioxus_cli = std::env::var("DIOXUS_CLI_ENABLED")
                .ok()
                .and_then(|s| s.parse::<bool>().ok())
                .unwrap_or(false);

            if dioxus_cli {
                info!("Dioxus CLI enabled: {}", dioxus_cli);
                if let Some(ip) = dioxus::cli_config::server_ip() {
                    server_config.internal_ip = ip.to_string();
                }
                if let Some(port) = dioxus::cli_config::server_port() {
                    server_config.port = port;
                }
            }

            if !server_config.db.exists() {
                let msg = format!(
                    "Database not found at {}! Create a new database with: `pslink migrate-database` or adjust the database path.",
                    server_config.db.display()
                );
                error!("{}", msg);
                eprintln!("{}", msg);
                return if dioxus_cli {
                    Err(ServerFnError::new("Database not found".to_string()))
                } else {
                    Ok(None)
                };
            }

            let pool = connect_db(&server_config.db, false).await?;
            let num_users = db::count_admins(&pool).await?;

            if num_users.number < 1 {
                warn!(concat!(
                    "No admin user created you will not be",
                    " able to do anything as the service is invite only.",
                    " Create a user with `pslink create-admin`"
                ));
            } else {
                trace!("At least one admin user is found.");
            }
            trace!("Initialization finished starting the service.");
            Ok(Some(AppState::new(pool, server_config)))
        }
    }
}

async fn add_example_links(pool: &SqlitePool) -> Result<(), ServerFnError> {
    let links = [
        (
            "Default for the empty url-code",
            "",
            "https://github.com/enaut/pslink",
        ),
        (
            "Pslink Repository",
            "pslink",
            "https://github.com/enaut/pslink",
        ),
        ("Dioxus", "dioxus", "https://dioxuslabs.com/"),
        ("Axum", "axum", "https://github.com/tokio-rs/axum"),
        ("Rust", "rust", "https://www.rust-lang.org/"),
    ];

    for (title, code, url) in links {
        NewLink {
            title: title.to_owned(),
            target: url.to_owned(),
            code: code.to_owned(),
            author: 1,
            created_at: chrono::Local::now().naive_utc(),
        }
        .insert(pool)
        .await?;
    }
    Ok(())
}

/// Interactively create a new admin user.
async fn request_admin_credentials(
    pool: &SqlitePool,
    config: &ServerConfig,
) -> Result<(), ServerFnError> {
    info!("Creating an admin user.");
    let sin = io::stdin();

    std::thread::sleep(std::time::Duration::from_millis(100));

    print!("Please enter the Username of the admin: ");
    io::stdout().flush().sfe()?;
    let new_username = sin
        .lock()
        .lines()
        .next()
        .ok_or_else(|| ServerFnError::new("Failed to read username"))?
        .sfe()?;

    print!("Please enter the email address for {}: ", new_username);
    io::stdout().flush().sfe()?;
    let new_email = sin
        .lock()
        .lines()
        .next()
        .ok_or_else(|| ServerFnError::new("Failed to read email"))?
        .sfe()?;

    io::stdout().flush().sfe()?;
    let password = if sin.lock().is_terminal() {
        info!("Reading the password from terminal for {}: ", new_username);
        rpassword::prompt_password(format!("Please enter the password for {}: ", new_username))
            .sfe()?
    } else {
        info!("Reading the password from buffer for {}: ", new_username);
        sin.lock()
            .lines()
            .next()
            .ok_or_else(|| ServerFnError::new("Failed to read password"))?
            .sfe()?
    };
    info!(
        "Creating {} ({}) with given password ",
        &new_username, &new_email
    );

    let new_admin = NewUser::new(
        new_username,
        new_email,
        &password,
        &config.secret,
    )?;

    db::create_admin(pool, &new_admin).await
}

/// Apply any pending migrations to the database (creating the SQLite file if it does not yet exist).
pub async fn apply_migrations(config: &ServerConfig) -> Result<SqlitePool, ServerFnError> {
    info!(
        "Applying database migrations to {}:",
        &config.db.display()
    );
    if !config.db.exists() {
        if let Some(parent) = config.db.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).sfe()?;
        }
        File::create(&config.db).sfe()?;
    }
    let pool = connect_db(&config.db, true).await?;
    MIGRATOR.run(&pool).await.sfe()?;
    info!("Migrations applied successfully.");
    Ok(pool)
}

/// Write a `.env` file with the current configuration if no non-empty `.env` file exists.
pub fn generate_env_file(server_config: &ServerConfig) -> Result<(), ServerFnError> {
    let env_path = std::path::Path::new(".env");

    if env_path.exists() && env_path.metadata().map(|m| m.len() > 0).unwrap_or(false) {
        return Err(ServerFnError::new(
            "ERROR: There already is a non-empty .env file - ABORT!".to_string(),
        ));
    }

    info!(
        r#"Creating a .env file with default options
            The PSLINK_SECRET variable is used for password encryption.
            If it is changed all existing passwords are invalid."#
    );

    let mut file = File::create(".env").sfe()?;
    let conf_file_content = server_config.to_env_strings();

    for line in &conf_file_content {
        file.write_all(line.as_bytes()).sfe()?;
    }
    info!("Successfully created the env file!");

    Ok(())
}

async fn generate_demo_data(server_config: &ServerConfig) -> Result<(), ServerFnError> {
    if server_config.db.exists()
        && server_config
            .db
            .metadata()
            .map(|m| m.len() > 0)
            .unwrap_or(false)
    {
        return Err(ServerFnError::new(
            "The database is not empty aborting because this could mean that creating a demo instance would lead in data loss."
                .to_string(),
        ));
    }

    info!("Creating a demo database.");
    generate_env_file(server_config)?;
    dotenv().ok();

    let pool = apply_migrations(server_config).await?;
    let new_admin = NewUser::new(
        "demo".to_string(),
        "demo@teilgedanken.de".to_string(),
        "demo",
        &server_config.secret,
    )?;
    db::create_admin(&pool, &new_admin).await?;
    add_example_links(&pool).await?;
    pool.close().await;
    Ok(())
}
