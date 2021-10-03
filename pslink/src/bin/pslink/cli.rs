use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, App, AppSettings,
    Arg, ArgMatches, SubCommand,
};
use dotenv::dotenv;
use pslink_shared::datatypes::{Secret, User};
use sqlx::{migrate::Migrator, Pool, Sqlite};
use std::{
    fs::File,
    io::{self, BufRead, Write},
    path::PathBuf,
};

use pslink::{
    models::{NewLink, NewUser, UserDbOperations},
    ServerConfig, ServerError,
};

use tracing::{error, info, trace, warn};

static MIGRATOR: Migrator = sqlx::migrate!();

/// generate the command line options available
#[allow(clippy::too_many_lines)]
fn generate_cli() -> App<'static, 'static> {
    app_from_crate!()
        .arg(
            Arg::with_name("database")
                .long("db")
                .help("The path of the sqlite database")
                .env("PSLINK_DATABASE")
                .default_value("links.db")
                .global(true),
        )
        .arg(
            Arg::with_name("port")
                .long("port")
                .short("p")
                .help("The port the pslink service will run on")
                .env("PSLINK_PORT")
                .default_value("8080")
                .global(true),
        )
        .arg(
            Arg::with_name("public_url")
                .long("public-url")
                .short("u")
                .help("The host url or the page that will be part of the short urls.")
                .env("PSLINK_PUBLIC_URL")
                .default_value("127.0.0.1:8080")
                .global(true),
        )
        .arg(
            Arg::with_name("empty_forward_url")
                .long("empty-url")
                .short("e")
                .help("The the url that / will redirect to. Usually your homepage.")
                .env("PSLINK_EMPTY_FORWARD_URL")
                .default_value("https://github.com/enaut/pslink")
                .global(true),
        )
        .arg(
            Arg::with_name("brand_name")
                .long("brand-name")
                .short("b")
                .help("The brand name that will appear in various places.")
                .env("PSLINK_BRAND_NAME")
                .default_value("Pslink")
                .global(true),
        )
        .arg(
            Arg::with_name("internal_ip")
                .long("hostip")
                .short("i")
                .help("The host (ip) that will run the pslink service")
                .env("PSLINK_IP")
                .default_value("127.0.0.1")
                .global(true),
        )
        .arg(
            Arg::with_name("protocol")
                .long("protocol")
                .short("t")
                .help(concat!(
                    "The protocol that is used in the qr-codes",
                    " (http results in slightly smaller codes in some cases)"
                ))
                .env("PSLINK_PROTOCOL")
                .default_value("http")
                .possible_values(&["http", "https"])
                .global(true),
        )
        .arg(
            Arg::with_name("secret")
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
            SubCommand::with_name("runserver")
                .about("Run the server")
                .display_order(1),
        )
        .subcommand(
            SubCommand::with_name("migrate-database")
                .about("Apply any pending migrations and exit")
                .display_order(2),
        )
        .subcommand(
            SubCommand::with_name("generate-env")
                .about("Generate an .env file template using default settings and exit")
                .display_order(2),
        )
        .subcommand(
            SubCommand::with_name("create-admin")
                .about("Create an admin user.")
                .display_order(2),
        )
        .subcommand(
            SubCommand::with_name("demo")
                .about("Create a database and demo user.")
                .display_order(3)
                .setting(AppSettings::Hidden),
        )
}

/// parse the options to the [`ServerConfig`] struct
async fn parse_args_to_config(config: ArgMatches<'_>) -> ServerConfig {
    let secret = config
        .value_of("secret")
        .expect("Failed to read the secret")
        .to_owned();
    let secret = if secret.len() < 5 {
        use rand::distributions::Alphanumeric;
        use rand::{thread_rng, Rng};

        if secret.is_empty() {
            warn!("No secret was found! Use the environment variable PSLINK_SECRET to set one.");
            warn!("If you change the secret all passwords will be invalid");
            warn!("Using an auto generated one for this run.");
        } else {
            warn!("The provided secret was too short. Using an auto generated one.");
        }

        thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect()
    } else {
        secret
    };
    let secret = Secret::new(secret);
    let db = config
        .value_of("database")
        .expect(concat!(
            "Neither the DATABASE_URL environment variable",
            " nor the command line parameters",
            " contain a valid database location."
        ))
        .parse::<PathBuf>()
        .expect("Failed to parse Database path.");
    let db_pool = Pool::<Sqlite>::connect(&db.display().to_string())
        .await
        .expect("Error: Failed to connect to database!");
    let public_url = config
        .value_of("public_url")
        .expect("Failed to read the host value")
        .to_owned();
    let empty_forward_url = config
        .value_of("empty_forward_url")
        .expect("Failed to read the empty_forward_url value")
        .to_owned();
    let brand_name = config
        .value_of("brand_name")
        .expect("Failed to read the brand_name value")
        .to_owned();
    let internal_ip = config
        .value_of("internal_ip")
        .expect("Failed to read the host value")
        .to_owned();
    let port = config
        .value_of("port")
        .expect("Failed to read the port value")
        .parse::<u32>()
        .expect("Failed to parse the port number");
    let protocol = config
        .value_of("protocol")
        .expect("Failed to read the protocol value")
        .parse::<pslink::Protocol>()
        .expect("Failed to parse the protocol");

    crate::ServerConfig {
        secret,
        db,
        db_pool,
        public_url,
        internal_ip,
        port,
        protocol,
        empty_forward_url,
        brand_name,
    }
}

/// Setup and launch the command
///
/// # Panics
/// This function panics if preconditions like the availability of the database are not met.
pub async fn setup() -> Result<Option<crate::ServerConfig>, ServerError> {
    // load the environment .env file if available.
    dotenv().ok();

    // Print launch info
    info!("Launching Pslink a 'Private short link generator'");

    let app = generate_cli();

    let config = app.get_matches();

    let db = config
        .value_of("database")
        .expect(concat!(
            "Neither the DATABASE_URL environment variable",
            " nor the command line parameters",
            " contain a valid database location."
        ))
        .parse::<PathBuf>()
        .expect("Failed to parse Database path.");

    if !db.exists() {
        trace!("No database file found {}", db.display());
        if !(config.subcommand_matches("migrate-database").is_none()
            | config.subcommand_matches("generate-env").is_none()
            | config.subcommand_matches("demo").is_none())
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
        }
        trace!("Creating database: {}", db.display());

        // create an empty database file. The if above makes sure that this file does not exist.
        File::create(db)?;
    };
    let server_config: crate::ServerConfig = parse_args_to_config(config.clone()).await;

    if let Some(_migrate_config) = config.subcommand_matches("generate-env") {
        return match generate_env_file(&server_config) {
            Ok(_) => Ok(None),
            Err(e) => Err(e),
        };
    }
    if let Some(_migrate_config) = config.subcommand_matches("migrate-database") {
        return match apply_migrations(&server_config).await {
            Ok(_) => Ok(None),
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
        let num_users = User::count_admins(&server_config).await;

        match num_users {
            Err(_) => {
                generate_env_file(&server_config).expect("Failed to generate env file.");
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
                create_admin(&new_admin, &server_config)
                    .await
                    .expect("Failed to create admin");
                add_example_links(&server_config).await;
                return Ok(Some(server_config));
            }
            _ => {
                return Err(ServerError::User("The database is not empty aborting because this could mean that creating a demo instance would lead in data loss.".to_string()));
            }
        }
    }

    if let Some(_runserver_config) = config.subcommand_matches("runserver") {
        let num_users = User::count_admins(&server_config).await?;

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
        println!("{}", config.usage());
        Err(ServerError::User("Print usage.".into()))
    }
}

async fn add_example_links(server_config: &ServerConfig) {
    NewLink {
        title: "Pslink Repository".to_owned(),
        target: "https://github.com/enaut/pslink".to_owned(),
        code: "pslink".to_owned(),
        author: 1,
        created_at: chrono::Local::now().naive_utc(),
    }
    .insert(server_config)
    .await
    .expect("Failed to insert example 1");

    NewLink {
        title: "Seed".to_owned(),
        target: "https://seed-rs.org/".to_owned(),
        code: "seed".to_owned(),
        author: 1,
        created_at: chrono::Local::now().naive_utc(),
    }
    .insert(server_config)
    .await
    .expect("Failed to insert example 1");

    NewLink {
        title: "actix".to_owned(),
        target: "https://actix.rs/".to_owned(),
        code: "actix".to_owned(),
        author: 1,
        created_at: chrono::Local::now().naive_utc(),
    }
    .insert(server_config)
    .await
    .expect("Failed to insert example 1");

    NewLink {
        title: "rust".to_owned(),
        target: "https://www.rust-lang.org/".to_owned(),
        code: "rust".to_owned(),
        author: 1,
        created_at: chrono::Local::now().naive_utc(),
    }
    .insert(server_config)
    .await
    .expect("Failed to insert example 1");
}

/// Interactively create a new admin user.
async fn request_admin_credentials(config: &ServerConfig) -> Result<(), ServerError> {
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

    print!("Please enter the password for {}: ", new_username);
    io::stdout().flush().unwrap();
    let password = rpassword::read_password().unwrap();
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

    create_admin(&new_admin, config).await
}

async fn create_admin(new_user: &NewUser, config: &ServerConfig) -> Result<(), ServerError> {
    new_user.insert_user(config).await?;
    let created_user = User::get_user_by_name(&new_user.username, config).await?;
    created_user.toggle_admin(config).await?;

    info!("Admin user created: {}", &new_user.username);

    Ok(())
}

/// Apply any pending migrations to the database. The migrations are embedded in the binary and don't need any additional files.
async fn apply_migrations(config: &ServerConfig) -> Result<(), ServerError> {
    info!(
        "Creating a database file and running the migrations in the file {}:",
        &config.db.display()
    );
    MIGRATOR.run(&config.db_pool).await?;
    Ok(())
}

/// The command line parameters provided or if missing the default parameters can be converted and written to a .env file. That way the configuration is saved and automatically reused for subsequent launches.
fn generate_env_file(server_config: &ServerConfig) -> Result<(), ServerError> {
    if std::path::Path::new(".env").exists() {
        return Err(ServerError::User(
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
