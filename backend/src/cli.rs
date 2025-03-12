use clap::{Arg, ArgMatches, Command, command};
use dioxus::logger::tracing::{error, info, trace, warn};
use dioxus::prelude::ServerFnError;
use dotenv::dotenv;
use pslink_shared::datatypes::{Secret, User};
use sqlx::migrate::Migrator;
use std::fmt::Display;
use std::io::IsTerminal;
use std::str::FromStr;
use std::{
    fs::File,
    io::{self, BufRead, BufReader, Write},
    path::PathBuf,
};

use crate::models::{NewLink, NewUser, UserDbOperations as _};
use crate::{get_db, init_db, init_secret};

static MIGRATOR: Migrator = sqlx::migrate!();

/// The qr-code can contain two different protocolls
#[derive(Debug, Clone)]
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
        match s {
            "http" => Ok(Self::Http),
            "https" => Ok(Self::Https),
            _ => Err(ServerFnError::new("Failed to parse Protocol".to_owned())),
        }
    }
}

/// The configuration of the server. It is accessible by the views and other parts of the program. Globally valid settings should be stored here.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub db: PathBuf,
    pub public_url: String,
    pub internal_ip: String,
    pub port: u16,
    pub protocol: Protocol,
    pub secret: Secret,
}

/// The configuration can be serialized into an environment-file.
impl ServerConfig {
    #[must_use]
    pub fn to_env_strings(&self) -> Vec<String> {
        vec![
            format!("PSLINK_DATABASE=\"{}\"\n", self.db.display()),
            format!("PSLINK_PORT={}\n", self.port),
            format!("PSLINK_PUBLIC_URL=\"{}\"\n", self.public_url),
            format!("PSLINK_IP=\"{}\"\n", self.internal_ip),
            format!("PSLINK_PROTOCOL=\"{}\"\n", self.protocol),
            concat!(
                "# The SECRET_KEY variable is used for password encryption.\n",
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
            format!("DEMO=\"{}\"\n", self.secret.is_random),
        ]
    }
}

fn generate_cli() -> Command {
    command!()
        .subcommand(
            Command::new("backend")
                .hide(true)
                .about("Run the server")
                .display_order(1),
        )
        .arg(
            Arg::new("database")
                .long("db")
                .help("The path of the sqlite database")
                .env("PSLINK_DATABASE")
                .default_value("./links.db")
                .global(true),
        )
        .arg(
            Arg::new("port")
                .long("port")
                .short('p')
                .help("The port the pslink service will run on")
                .env("PSLINK_PORT")
                .value_parser(clap::value_parser!(u16))
                .global(true),
        )
        .arg(
            Arg::new("public_url")
                .long("public-url")
                .short('u')
                .help("The host url or the page that will be part of the short urls.")
                .env("PSLINK_PUBLIC_URL")
                .default_value("127.0.0.1:8080")
                .global(true),
        )
        .arg(
            Arg::new("empty_forward_url")
                .long("empty-url")
                .short('e')
                .help("The the url that / will redirect to. Usually your homepage.")
                .env("PSLINK_EMPTY_FORWARD_URL")
                .default_value("https://github.com/enaut/pslink")
                .global(true),
        )
        .arg(
            Arg::new("brand_name")
                .long("brand-name")
                .short('b')
                .help("The Brandname that will apper in various places.")
                .env("PSLINK_BRAND_NAME")
                .default_value("Pslink")
                .global(true),
        )
        .arg(
            Arg::new("internal_ip")
                .long("hostip")
                .short('i')
                .help("The host (ip) that will run the pslink service")
                .env("PSLINK_IP")
                .global(true),
        )
        .arg(
            Arg::new("demo")
                .long("demo")
                .help("The host (ip) that will run the pslink service")
                .env("DEMO")
                .global(true),
        )
        .arg(
            Arg::new("protocol")
                .long("protocol")
                .short('t')
                .help(concat!(
                    "The protocol that is used in the qr-codes",
                    " (http results in slightly smaller codes in some cases)"
                ))
                .env("PSLINK_PROTOCOL")
                .default_value("http")
                .value_parser(["http", "https"])
                .global(true),
        )
        .arg(
            Arg::new("secret")
                .long("secret")
                .help(concat!(
                    "The secret that is used to encrypt the",
                    " password database keep this as inaccessible as possible.",
                    " As command line parameters are visible",
                    " to all users",
                    " it is not wise to use this as",
                    " a command line parameter but rather as an environment variable.",
                ))
                .env("PSLINK_SECRET")
                .default_value("")
                .global(true),
        )
        .subcommand(
            Command::new("runserver")
                .about("Run the server")
                .display_order(1),
        )
        .subcommand(
            Command::new("migrate-database")
                .about("Apply any pending migrations and exit")
                .display_order(2),
        )
        .subcommand(
            Command::new("generate-env")
                .about("Generate an .env file template using default settings and exit")
                .display_order(2),
        )
        .subcommand(
            Command::new("create-admin")
                .about("Create an admin user.")
                .display_order(2),
        )
        .subcommand(
            Command::new("demo")
                .about("Create a database and demo user.")
                .display_order(3)
                .hide(true),
        )
}

/// parse the options to the [`ServerConfig`] struct
async fn parse_args_to_config(config: ArgMatches) -> ServerConfig {
    info!("Parsing the arguments");
    let secret = config
        .get_one::<String>("secret")
        .expect("Failed to read the secret")
        .to_owned();
    let demo = config
        .get_one::<String>("demo")
        .unwrap_or(&"false".to_string())
        .parse::<bool>()
        .expect("Failed to parse the demo value");
    let secret = if secret.len() < 5 {
        if secret.is_empty() {
            warn!("No secret was found! Use the environment variable PSLINK_SECRET to set one.");
            warn!("If you change the secret all passwords will be invalid");
            warn!("Using an auto generated one for this run.");
        } else {
            warn!("The provided secret was too short. Using an auto generated one.");
        }
        Secret::random()
    } else {
        let mut secret = Secret::new(secret);
        secret.is_random = demo;
        secret
    };
    let db = config
        .get_one::<String>("database")
        .expect(concat!(
            "Neither the DATABASE_URL environment variable",
            " nor the command line parameters",
            " contain a valid database location."
        ))
        .parse::<PathBuf>()
        .expect("Failed to parse Database path.");
    let public_url = config
        .get_one::<String>("public_url")
        .expect("Failed to read the host value")
        .to_owned();
    let internal_ip = dioxus::cli_config::server_ip().map(|s| s.to_string());
    let internal_ip = config
        .get_one::<String>("internal_ip")
        .or_else(|| internal_ip.as_ref())
        .or(Some(&"127.0.0.1".to_string()))
        .expect("Failed to read the host value")
        .to_owned();
    let port = dioxus::cli_config::server_port();
    let port = *config
        .get_one::<u16>("port")
        .or_else(|| port.as_ref())
        .or(Some(&8080))
        .expect("Failed to read the port value");
    let protocol = config
        .get_one::<String>("protocol")
        .expect("Failed to read the protocol value")
        .parse::<Protocol>()
        .expect("Failed to parse the protocol");
    info!("Arguments parsed");
    ServerConfig {
        db,
        public_url,
        internal_ip,
        port: port,
        protocol,
        secret,
    }
}

/// Setup and launch the command
///
/// This function is the entry point for the server. It parses the command line arguments and sets up the database and the server configuration.
/// It also handles the creation of the database, the migrations, and the creation of the admin user.
///
/// If a cli command is provided the function will execute the command and return `Ok(None)`.
///
/// # Panics
/// This function panics if preconditions like the availability of the database are not met.
pub async fn setup() -> Result<Option<ServerConfig>, ServerFnError> {
    // load the environment .env file if available.
    dotenv().ok();

    // Print launch info
    info!("Launching Pslink a 'Private short link generator'");
    println!(
        "Command line arguments: {:?}",
        std::env::args().collect::<Vec<_>>()
    );

    let app = generate_cli();
    let config = app.clone().get_matches();

    let mut server_config: ServerConfig = parse_args_to_config(config.clone()).await;
    init_secret(server_config.secret.clone());
    if config.subcommand().is_none() {
        // if the variable DIOXUS_CLI_ENABLED is true run the server
        let dioxus_cli =
            std::env::var("DIOXUS_CLI_ENABLED").map_or(false, |s| s.parse().unwrap_or(false));
        if dioxus_cli {
            server_config.internal_ip = dioxus::cli_config::server_ip()
                .expect("Cli should be enabled")
                .to_string();
            server_config.port = dioxus::cli_config::server_port().expect("Cli should be enabled");

            // Check if database exists
            if !server_config.db.exists() {
                error!("Database not found at {}", server_config.db.display());
                error!("Do you want to create a demo database? If so use the command:");
                error!("target/dx/web/debug/web/server demo");
                error!("Afterwards restart the dx command.");
                return Err(ServerFnError::new("Database not found".to_string()));
            } else {
                init_db(&server_config.db.to_string_lossy()).await;
                info!(
                    "Starting the server with the following configuration: {} {}",
                    server_config.internal_ip, server_config.port
                );
                return Ok(Some(server_config));
            }
        } else {
            println!("{}", generate_cli().render_usage());
            return Err(ServerFnError::new(
                "The command is missing try `runserver`".to_string(),
            ));
        }
    }

    let db = server_config.db.clone();
    trace!("Checking if the database exists at {}", db.display());
    if db.exists() {
        init_db(&db.to_string_lossy()).await;
    } else {
        trace!("No database file found {}", db.display());
        if !(config.subcommand_matches("migrate-database").is_some()
            | config.subcommand_matches("generate-env").is_some()
            | config.subcommand_matches("demo").is_some())
        {
            let msg = format!(
                concat!(
                    "Database not found at {}!",
                    "Create a new database with: `pslink migrate-database`",
                    "or adjust the database path."
                ),
                db.display()
            );
            error!("{}", msg);
            eprintln!("{}", msg);
            return Ok(None);
        } else {
            warn!("Database not found at {}!", db.display());
        }
    };

    trace!("Evaluation of the subcommands");

    if let Some(_migrate_config) = config.subcommand_matches("generate-env") {
        return match generate_env_file(&server_config) {
            Ok(_) => Ok(None),
            Err(e) => Err(e),
        };
    }
    if let Some(_migrate_config) = config.subcommand_matches("migrate-database") {
        return match apply_migrations(&server_config).await {
            Ok(_) => {
                info!("successfuly migrated the database.");
                Ok(None)
            }
            Err(e) => Err(e),
        };
    }
    if let Some(_create_config) = config.subcommand_matches("create-admin") {
        return match request_admin_credentials(&server_config).await {
            Ok(_) => Ok(None),
            Err(e) => Err(e),
        };
    }

    if let Some(_runserver_config) = config.subcommand_matches("demo") {
        return generate_demo_data(server_config).await;
    }

    if let Some(_runserver_config) = config.subcommand_matches("runserver") {
        let num_users = User::count_admins().await?;

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
        Ok(Some(server_config))
    } else {
        Err(ServerFnError::new("No command was provided.".to_string()))
    }
}

async fn add_example_links() {
    let links = vec![
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
        .insert()
        .await
        .expect("Failed to insert example 1");
    }
}

/// Interactively create a new admin user.
async fn request_admin_credentials(config: &ServerConfig) -> Result<(), ServerFnError> {
    info!("Creating an admin user.");
    let sin = io::stdin();

    // wait for logging:
    std::thread::sleep(std::time::Duration::from_millis(100));

    print!("Please enter the Username of the admin: ");
    io::stdout().flush().unwrap();
    let new_username = sin.lock().lines().next().unwrap().unwrap();

    print!("Please enter the email address for {}: ", new_username);
    io::stdout().flush().unwrap();
    let new_email = sin.lock().lines().next().unwrap().unwrap();

    io::stdout().flush().unwrap();
    let password = if sin.lock().is_terminal() {
        info!("Reading the password from terminal for {}: ", new_username);
        rpassword::prompt_password(format!("Please enter the password for {}: ", new_username))
            .unwrap()
    } else {
        info!("Reading the password from buffer for {}: ", new_username);
        let mut stdin = BufReader::new(io::stdin());
        rpassword::read_password_from_bufread(&mut stdin).unwrap()
    };
    info!(
        "Creating {} ({}) with given password ",
        &new_username, &new_email
    );

    let new_admin = NewUser::new(
        new_username.clone(),
        new_email.clone(),
        &password,
        &config.secret,
    )?;

    create_admin(&new_admin).await
}

async fn create_admin(new_user: &NewUser) -> Result<(), ServerFnError> {
    new_user.insert_user().await?;
    let created_user = User::get_user_by_name(&new_user.username).await?;
    created_user.toggle_admin().await?;

    info!("Admin user created: {}", &new_user.username);

    Ok(())
}

/// Apply any pending migrations to the database. The migrations are embedded in the binary and don't need any additional files.
async fn apply_migrations(config: &ServerConfig) -> Result<(), ServerFnError> {
    info!(
        "Creating a database file and running the migrations in the file {}:",
        &config.db.display()
    );
    if config.db.exists() {
        return Err(ServerFnError::new("The database is not empty aborting because this could mean that creating a demo instance would lead in data loss.".to_string()));
    } else {
        File::create(&config.db)?;
        init_db(&config.db.to_string_lossy()).await;
    }
    let pool = get_db().await;
    MIGRATOR.run(&pool).await?;
    info!("Migrations applied successfully.");
    Ok(())
}

/// The command line parameters provided or if missing the default parameters can be converted and written to a .env file. That way the configuration is saved and automatically reused for subsequent launches.
fn generate_env_file(server_config: &ServerConfig) -> Result<(), ServerFnError> {
    if std::path::Path::new(".env").exists() {
        return Err(ServerFnError::new(
            "ERROR: There already is a .env file - ABORT!".to_string(),
        ));
    }

    info!(
        r#"Creating a .env file with default options
            The SECRET_KEY variable is used for password encryption.
            If it is changed all existing passwords are invalid."#
    );

    let mut file = std::fs::File::create(".env")?;
    let conf_file_content = server_config.to_env_strings();

    for line in &conf_file_content {
        file.write_all(line.as_bytes())
            .expect("failed to write .env file");
    }
    info!("Successfully created the env file!");

    Ok(())
}

async fn generate_demo_data(
    server_config: ServerConfig,
) -> Result<Option<ServerConfig>, ServerFnError> {
    if server_config.db.exists() {
        return Err(ServerFnError::new("The database is not empty aborting because this could mean that creating a demo instance would lead in data loss.".to_string()));
    } else {
        info!("Creating a demo database.");
        generate_env_file(&server_config).expect("Failed to generate env file.");

        dotenv().ok();
        apply_migrations(&server_config)
            .await
            .expect("Failed to apply migrations.");

        let new_admin = NewUser::new(
            "demo".to_string(),
            "demo@teilgedanken.de".to_string(),
            "demo",
            &server_config.secret,
        )
        .expect("Failed to generate new user credentials.");
        create_admin(&new_admin)
            .await
            .expect("Failed to create admin");
        add_example_links().await;
        return Ok(None);
    }
}
