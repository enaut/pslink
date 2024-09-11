use serde_json::json;

#[test]
fn test_help_of_command_for_breaking_changes() {
    let output = test_bin::get_test_bin("pslink")
        .output()
        .expect("Failed to start pslink");
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage"));

    let output = test_bin::get_test_bin("pslink")
        .args(["--help"])
        .output()
        .expect("Failed to start pslink");
    let outstring = String::from_utf8_lossy(&output.stdout);

    let args = &[
        "Usage",
        "-h",
        "--help",
        "-b",
        "-e",
        "-i",
        "-p",
        "-t",
        "-u",
        "runserver",
        "create-admin",
        "generate-env",
        "migrate-database",
        "help",
    ];

    for s in args {
        assert!(
            outstring.contains(s),
            "{} was not found in the help - this is a breaking change",
            s
        );
    }
}

#[test]
fn test_generate_env() {
    use std::io::BufRead;
    let tmp_dir = tempdir::TempDir::new("pslink_test_env").expect("create temp dir");
    let output = test_bin::get_test_bin("pslink")
        .args(["generate-env", "--secret", "abcdefghijklmnopqrstuvw"])
        .current_dir(&tmp_dir)
        .output()
        .expect("Failed to start pslink");
    let envfile = tmp_dir.path().join(".env");
    let dbfile = tmp_dir.path().join("links.db");
    println!("{}", envfile.display());
    println!("{}", dbfile.display());
    println!("{}", String::from_utf8_lossy(&output.stdout));
    assert!(envfile.exists(), "No .env-file was created!");
    assert!(dbfile.exists(), "No database-file was created!");

    let envfile = std::fs::File::open(envfile).unwrap();
    let envcontent: Vec<Result<String, _>> = std::io::BufReader::new(envfile).lines().collect();
    assert!(
        envcontent
            .iter()
            .any(|s| s.as_ref().unwrap().starts_with("PSLINK_PORT=")),
        "Failed to find PSLINK_PORT in the generated .env file."
    );
    assert!(
        envcontent
            .iter()
            .any(|s| s.as_ref().unwrap().starts_with("PSLINK_SECRET=")),
        "Failed to find PSLINK_SECRET in the generated .env file."
    );
    assert!(
        !envcontent.iter().any(|s| {
            let r = s.as_ref().unwrap().contains("***SECRET***");
            r
        }),
        "It seems that a censored secret was used in the .env file."
    );
    assert!(
        envcontent.iter().any(|s| {
            let r = s.as_ref().unwrap().contains("abcdefghijklmnopqrstuvw");
            r
        }),
        "The secret has not made it into the .env file!"
    );
    let output = test_bin::get_test_bin("pslink")
        .args(["generate-env"])
        .current_dir(&tmp_dir)
        .output()
        .expect("Failed to start pslink");
    let second_out = String::from_utf8_lossy(&output.stdout);
    assert!(!second_out.contains("secret"));
}

#[actix_rt::test]
async fn test_migrate_database() {
    use std::io::Write;
    #[derive(serde::Serialize, Debug)]
    pub struct Count {
        pub number: i64,
    }
    println!("Starting test_migrate_database");
    let tmp_dir = tempdir::TempDir::new("pslink_test_env").expect("create temp dir");
    println!("Created temp dir");
    // generate .env file
    let _output = test_bin::get_test_bin("pslink")
        .args(["generate-env"])
        .current_dir(&tmp_dir)
        .output()
        .expect("Failed generate .env");
    println!("Generated .env file");

    // migrate the database
    let output = test_bin::get_test_bin("pslink")
        .args(["migrate-database"])
        .current_dir(&tmp_dir)
        .output()
        .expect("Failed to migrate the database");
    println!("Output: {}", String::from_utf8_lossy(&output.stdout));

    // check if the users table exists by counting the number of admins.
    let db_pool = sqlx::pool::Pool::<sqlx::sqlite::Sqlite>::connect(
        &tmp_dir.path().join("links.db").display().to_string(),
    )
    .await
    .expect("Error: Failed to connect to database!");
    println!("Connected to database");
    let num = sqlx::query_as!(Count, "select count(*) as number from users where role = 2")
        .fetch_one(&db_pool)
        .await
        .unwrap();
    println!("Number of admins: {}", num.number);
    // initially no admin is present
    assert_eq!(num.number, 0, "Failed to create the database!");

    println!("Creating an admin");
    // create a new admin
    let mut input = test_bin::get_test_bin("pslink")
        .args(["create-admin"])
        .current_dir(&tmp_dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to migrate the database");
    let mut procin = input.stdin.take().unwrap();

    async {
        println!("Writing username");
        println!("Bytes written: {}", procin.write(b"test\n").unwrap());
        procin.flush().unwrap();
    }
    .await;
    async {
        println!("Writing email");
        procin.write_all(b"test@mail.test\n").unwrap();
        procin.flush().unwrap();
    }
    .await;
    async {
        println!("Writing password");
        procin.write_all(b"testpw\n").unwrap();
        procin.flush().unwrap();
        drop(procin);
    }
    .await;
    //read_output(&mut procout);
    println!("Waiting for process to finish");

    let r = input.wait().unwrap();
    println!("Exitstatus is: {:?}", r);
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let num = sqlx::query_as!(Count, "select count(*) as number from users where role = 2")
        .fetch_one(&db_pool)
        .await
        .unwrap();
    // now 1 admin is there
    assert_eq!(num.number, 1, "Failed to create an admin!");
}

async fn run_server() -> (
    tokio::task::JoinHandle<std::result::Result<(), std::io::Error>>,
    tempdir::TempDir,
) {
    use std::io::Write;
    #[derive(serde::Serialize, Debug)]
    pub struct Count {
        pub number: i64,
    }
    let tmp_dir = tempdir::TempDir::new("pslink_test_env").expect("create temp dir");
    // generate .env file
    let _output = test_bin::get_test_bin("pslink")
        .args(["generate-env", "--secret", "abcdefghijklmnopqrstuvw"])
        .current_dir(&tmp_dir)
        .output()
        .expect("Failed generate .env");
    // migrate the database
    let output = test_bin::get_test_bin("pslink")
        .args(["migrate-database"])
        .current_dir(&tmp_dir)
        .output()
        .expect("Failed to migrate the database");

    // create a database connection.
    let db_pool = sqlx::pool::Pool::<sqlx::sqlite::Sqlite>::connect(
        &tmp_dir.path().join("links.db").display().to_string(),
    )
    .await
    .expect("Error: Failed to connect to database!"); // create a new admin
    let mut input = test_bin::get_test_bin("pslink")
        .args(&["create-admin"])
        .current_dir(&tmp_dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to migrate the database");
    let mut procin = input.stdin.take().unwrap();

    procin.write_all(b"test\n").unwrap();
    procin.write_all(b"test@mail.test\n").unwrap();
    procin.write_all(b"testpw\n").unwrap();

    let r = input.wait().unwrap();
    println!("Exitstatus is: {}", r);

    println!("{}", String::from_utf8_lossy(&output.stdout));
    let num = sqlx::query_as!(Count, "select count(*) as number from users where role = 2")
        .fetch_one(&db_pool)
        .await
        .unwrap();
    // now 1 admin is there
    assert_eq!(
        num.number, 1,
        "Failed to create an admin! See previous tests!"
    );

    let server_config = pslink::ServerConfig {
        secret: shared::datatypes::Secret::new("abcdefghijklmnopqrstuvw".to_string()),
        db: std::path::PathBuf::from("links.db"),
        db_pool,
        public_url: "localhost:8085".to_string(),
        internal_ip: "localhost".to_string(),
        port: 8085,
        protocol: pslink::Protocol::Http,
        empty_forward_url: "https://github.com/enaut/pslink".to_string(),
        brand_name: "Pslink".to_string(),
    };

    let server = pslink::webservice(server_config);

    let server_handle = tokio::spawn(
        server
            .await
            .map_err(|e| {
                println!("{:?}", e);
                std::thread::sleep(std::time::Duration::from_millis(100));
                std::process::exit(0);
            })
            .expect("Failed to launch the service"),
    );
    println!("Server started");
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    println!("Waited for startup");
    (server_handle, tmp_dir)
}

#[actix_rt::test]
async fn test_web_paths() {
    let (server_handle, _tmp_dir) = run_server().await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let cookie_store = reqwest_cookie_store::CookieStore::new(None);
    let cookie_store = reqwest_cookie_store::CookieStoreMutex::new(cookie_store);
    let cookie_store = std::sync::Arc::new(cookie_store);
    // We need to bring in `reqwest`
    // to perform HTTP requests against our application.
    let client = reqwest::Client::builder()
        .cookie_provider(std::sync::Arc::clone(&cookie_store))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    // Act
    let response = client
        .get("http://localhost:8085/")
        .send()
        .await
        .expect("Failed to execute request.");

    // The basic redirection is working!
    assert!(response.status().is_redirection());
    let location = response.headers().get("location").unwrap();
    assert!(location.to_str().unwrap().contains("github"));

    // Act
    let json_data = json!({
        "username": "test",
        "password": "testpw"
    });
    let response = client
        .post("http://localhost:8085/admin/json/login_user/")
        .json(&json_data)
        .send()
        .await
        .expect("Failed to execute request.");
    // It is possible to login
    assert!(response.status().is_success());
    let response_text = response.text().await.unwrap();
    assert!(response_text.contains("\"username\":\"test\""));
    assert!(response_text.contains("\"email\":\"test@mail.test\""));
    assert!(!response_text.contains("testpw"));

    let json_data = json!({
        "filter": {
            "Code": {"sieve": ""},
            "Description": {"sieve": ""},
            "Target": {"sieve": ""},
            "Author": {"sieve": ""},
            "Statistics": {"sieve": ""}
        },
        "order": null,
        "offset": 0,
        "amount": 60
    });
    // After login accessing the main page should work
    let response = client
        .post("http://localhost:8085/admin/json/list_links/")
        .json(&json_data)
        .send()
        .await
        .expect("Failed to execute request.");
    println!("Response of /admin/json/list_links/: {:?}", response);

    // The Loginpage redirects to link index when logged in
    assert!(
        response.status().is_success(),
        "Could not get list of links: {}",
        response.text().await.unwrap()
    );
    let content = response.text().await.unwrap();
    assert!(
        content.contains(r#"[]"#),
        "The list of links is not empty: {}",
        content
    );

    // Act title=haupt&target=http%3A%2F%2Fdas.geht%2Fjetzt%2F&code=tpuah
    use serde_json::json;

    let json_data = json!({
        "edit": "Create",
        "id": null,
        "title": "mytite",
        "target": "https://github.com/enaut/pslink/",
        "code": "mycode",
        "author": 0,
        "created_at": null
    });
    let response = client
        .post("http://localhost:8085/admin/json/create_link/")
        .json(&json_data)
        .send()
        .await
        .expect("Failed to execute request.");

    // It is possible to login
    assert!(response.status().is_success());
    let response_text = response.text().await.unwrap();
    println!("Response of /admin/json/create_link/: {:?}", response_text);
    assert!(response_text.contains("\"message\":\"Successfully saved link: mycode\""));

    // Act
    let response = client
        .get("http://localhost:8085/mycode")
        .send()
        .await
        .expect("Failed to execute request.");

    // The basic redirection is working!
    assert!(response.status().is_redirection());
    let location = response.headers().get("location").unwrap();
    assert!(location
        .to_str()
        .unwrap()
        .contains("https://github.com/enaut/pslink/"));

    server_handle.abort();
}
