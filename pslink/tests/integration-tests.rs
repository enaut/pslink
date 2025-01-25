use assert_cmd::prelude::*; // Add methods on commands
use reqwest::header::HeaderMap;
use serde_json::json;
use std::{collections::HashMap, io::Read, time::Duration};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::TcpListener,
    process::{Child, Command},
    time::sleep,
};

#[actix_rt::test]
async fn test_help_of_command_for_breaking_changes() {
    let output = get_test_bin("pslink")
        .output()
        .await
        .expect("Failed to start pslink");
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage"));

    let output = get_test_bin("pslink")
        .args(["--help"])
        .output()
        .await
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

#[actix_rt::test]
async fn test_generate_env() {
    use std::io::BufRead;
    let tmp_dir = tempdir::TempDir::new("pslink_test_env").expect("create temp dir");
    let output = get_test_bin("pslink")
        .args(["generate-env", "--secret", "abcdefghijklmnopqrstuvw"])
        .current_dir(&tmp_dir)
        .output()
        .await
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
    let output = get_test_bin("pslink")
        .args(["generate-env"])
        .current_dir(&tmp_dir)
        .output()
        .await
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
    let _output = get_test_bin("pslink")
        .args(["generate-env"])
        .current_dir(&tmp_dir)
        .output()
        .await
        .expect("Failed generate .env");
    println!("Generated .env file");

    // migrate the database
    let output = get_test_bin("pslink")
        .args(["migrate-database"])
        .current_dir(&tmp_dir)
        .output()
        .await
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
    let mut input = get_test_bin("pslink")
        .args(["create-admin"])
        .current_dir(&tmp_dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to migrate the database");
    let mut procin = input.stdin.take().unwrap();

    async {
        println!("Writing username");
        println!("Bytes written: {}", procin.write(b"test\n").await.unwrap());
        procin.flush().await.unwrap();
    }
    .await;
    async {
        println!("Writing email");
        procin.write_all(b"test@mail.test\n").await.unwrap();
        procin.flush().await.unwrap();
    }
    .await;
    async {
        println!("Writing password");
        procin.write_all(b"testpw\n").await.unwrap();
        procin.flush().await.unwrap();
        drop(procin);
    }
    .await;
    //read_output(&mut procout);
    println!("Waiting for process to finish");

    let r = input.wait().await.unwrap();
    println!("Exitstatus is: {:?}", r);
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let num = sqlx::query_as!(Count, "select count(*) as number from users where role = 2")
        .fetch_one(&db_pool)
        .await
        .unwrap();
    // now 1 admin is there
    assert_eq!(num.number, 1, "Failed to create an admin!");
}

struct RunningServer {
    server: Child,
    port: i32,
}

impl Drop for RunningServer {
    fn drop(&mut self) {
        tokio::task::block_in_place(|| async {
            self.server.kill().await.unwrap();
        });
    }
}

async fn run_server() -> RunningServer {
    use std::io::Write;

    use rand::thread_rng;
    use rand::Rng;

    #[derive(serde::Serialize, Debug)]
    pub struct Count {
        pub number: i64,
    }

    let mut rng = thread_rng();
    let port = rng.gen_range(12000..20000);
    let tmp_dir = tempdir::TempDir::new("pslink_test_env").expect("create temp dir");
    // generate .env file
    let _output = get_test_bin("pslink")
        .args(&[
            "generate-env",
            "--secret",
            "abcdefghijklmnopqrstuvw",
            "--port",
            &port.to_string(),
        ])
        .current_dir(&tmp_dir)
        .output()
        .await
        .expect("Failed generate .env");
    // migrate the database
    let output = get_test_bin("pslink")
        .args(&["migrate-database"])
        .current_dir(&tmp_dir)
        .output()
        .await
        .expect("Failed to migrate the database");

    // create a database connection.
    let db_pool = sqlx::pool::Pool::<sqlx::sqlite::Sqlite>::connect(
        &tmp_dir.path().join("links.db").display().to_string(),
    )
    .await
    .expect("Error: Failed to connect to database!"); // create a new admin
    let mut input = get_test_bin("pslink")
        .args(&["create-admin"])
        .current_dir(&tmp_dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to migrate the database");
    let mut procin = input.stdin.take().unwrap();

    procin.write_all(b"test\n").await.unwrap();
    procin.write_all(b"test@mail.test\n").await.unwrap();
    procin.write_all(b"testpw\n").await.unwrap();

    let r = input.wait().await.unwrap();
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

    let mut server = get_test_bin("pslink")
        .args(&["runserver"])
        .current_dir(&tmp_dir)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    // Wait until the server signals it is up and running.
    let mut sout = server.stdout.take().unwrap();
    let mut buffer = [0; 15];
    println!("Running the webserver for testing #############");
    loop {
        let num = sout.read(&mut buffer[..]).await.unwrap();
        println!("{}", num);
        let t = std::str::from_utf8(&buffer).unwrap();
        println!("{:?}", std::str::from_utf8(&buffer));
        if num > 0 && t.contains("/app") {
            break;
        }
    }

    RunningServer { server, port }
}

async fn wait_for_server(url: &str, retries: u32, delay: Duration) -> Result<(), String> {
    let client = reqwest::Client::new();
    for x in 0..retries {
        let response = client.get(url).send().await;
        if response.is_ok() {
            return Ok(());
        }
        println!("Server not ready, retrying ({})...", x);
        sleep(delay).await;
    }
    Err("Server not ready".to_string())
}

#[actix_rt::test]
async fn test_web_paths() {
    let server = run_server().await;
    let base_url = format!("localhost:{}/", server.port);
    println!("# BaseUrl: {}", base_url);

    // Wartezeit und Wiederholungsversuche hinzufügen
    wait_for_server(&base_url, 120, Duration::from_millis(100))
        .await
        .unwrap();

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
        .get(&base_url.clone())
        .send()
        .await
        .expect("Failed to execute request.");

    // The basic redirection is working!
    assert!(response.status().is_redirection());
    let location = response.headers().get("location").unwrap();
    assert!(location.to_str().unwrap().contains("github"));

    let app_url = base_url.clone() + "app/";
    // Act
    let json_data = json!({
        "username": "test",
        "password": "testpw"
    });
    let response = client
        .get(&app_url.clone())
        .send()
        .await
        .expect("Failed to execute request.");

    println!("{:?}", response);

    // The app page is reachable and contains the wasm file!
    assert!(response.status().is_success());
    let content = response.text().await.unwrap();
    assert!(
        content.contains(r#"init('/static/wasm/app_bg.wasm');"#),
        "The app page has unexpected content!"
    );

    // Act
    let mut formdata = HashMap::new();
    formdata.insert("username", "test");
    formdata.insert("password", "testpw");
    let response = client
        .post(&(base_url.clone() + "admin/json/login_user/"))
        .json(&formdata)
        .send()
        .await
        .expect("Failed to execute request.");

    println!("Login response:\n {:?}", response);

    // It is possible to login
    assert!(response.status().is_success());

    // Extract the cookie as it is not automatically saved for some reason.
    let cookie = {
        response
            .headers()
            .get("set-cookie")
            .expect("A auth cookie is not set even though authentication succeeds")
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string()
    };
    println!("{:?}", cookie);
    assert!(cookie.starts_with("auth-cookie="));

    let content = response.text().await.unwrap();
    println!("Content: {:?}", content);
    assert!(content.contains(r#""id":1"#), "id missing in content");

    let mut custom_headers = HeaderMap::new();
    custom_headers.insert("content-type", "application/json".parse().unwrap());
    custom_headers.insert("Cookie", cookie.parse().unwrap());

    // After login this should return an empty list
    let query = client
        .post(&(base_url.clone() + "admin/json/list_links/"))
        .headers(custom_headers.clone())
        .body(r#"{"filter":{"Code":{"sieve":""},"Description":{"sieve":""},"Target":{"sieve":""},"Author":{"sieve":""},"Statistics":{"sieve":""}},"order":null,"amount":20}"#).build().unwrap();
    println!("{:?}", query);
    let response = client
        .execute(query)
        .await
        .expect("Failed to execute request.");
    println!("List urls response:\n {:?}", response);

    // Make sure the list was retrieved and the status codes are correct
    assert!(response.status().is_success());

    // Make sure that the content is an empty list as until now no links were created.
    let content = response.text().await.unwrap();
    println!("Content: {:?}", content);
    assert!(content.contains(r#"[]"#), "id missing in content");

    // Create a link
    let query = client
        .post(&(base_url.clone() + "admin/json/create_link/"))
        .headers(custom_headers.clone())
        .body(r#"{"edit":"Create","id":null,"title":"ein testlink","target":"https://github.com/enaut/pslink","code":"test","author":0,"created_at":null}"#)
        .build()
        .unwrap();
    println!("{:?}", query);
    let response = client
        .execute(query)
        .await
        .expect("Failed to execute request.");
    println!("List urls response:\n {:?}", response);

    // Make sure the status codes are correct
    assert!(response.status().is_success());

    // Make sure that the content is a success message
    let content = response.text().await.unwrap();
    println!("Content: {:?}", content);
    assert!(
        content.contains(r#""Success":"#),
        "Make sure the link creation response contains Success"
    );

    // After inserting a link make sure the link is saved
    let query = client
        .post(&(base_url.clone() + "admin/json/list_links/"))
        .headers(custom_headers.clone())
        .body(r#"{"filter":{"Code":{"sieve":""},"Description":{"sieve":""},"Target":{"sieve":""},"Author":{"sieve":""},"Statistics":{"sieve":""}},"order":null,"amount":20}"#).build().unwrap();
    println!("{:?}", query);
    let response = client
        .execute(query)
        .await
        .expect("Failed to execute request.");
    println!("List urls response:\n {:?}", response);

    // Make sure the list was retrieved and the status codes are correct
    assert!(response.status().is_success());

    // Make sure that the content now contains the newly created link
    let content = response.text().await.unwrap();
    println!("Content: {:?}", content);
    assert!(
        content.contains(r#""target":"https://github.com/enaut/pslink","code":"test""#),
        "the new target and the new code are not in the result"
    );

    // Create a duplicate which should fail
    let query = client
        .post(&(base_url.clone() + "admin/json/create_link/"))
        .headers(custom_headers.clone())
        .body(r#"{"edit":"Create","id":null,"title":"ein testlink","target":"https://github.com/enaut/pslink","code":"test","author":0,"created_at":null}"#)
        .build()
        .unwrap();
    println!("{:?}", query);
    let response = client
        .execute(query)
        .await
        .expect("Failed to execute request.");
    println!("List urls response:\n {:?}", response);

    // Make sure the status codes are correct
    assert!(response.status().is_server_error());

    // Make sure that the content is a error message
    let content = response.text().await.unwrap();
    println!("Content: {:?}", content);
    assert!(
        content.contains(r#"error"#),
        "Make sure the link creation response contains error"
    );

    // Create a second link
    let query = client
        .post(&(base_url.clone() + "admin/json/create_link/"))
        .headers(custom_headers.clone())
        .body(r#"{"edit":"Create","id":null,"title":"ein second testlink","target":"https://crates.io/crates/pslink","code":"x","author":0,"created_at":null}"#)
        .build()
        .unwrap();
    println!("{:?}", query);
    let response = client
        .execute(query)
        .await
        .expect("Failed to execute request.");
    println!("List urls response:\n {:?}", response);

    // Make sure the status codes are correct
    assert!(response.status().is_success());

    // Make sure that the content is a success message
    let content = response.text().await.unwrap();
    println!("Content: {:?}", content);
    assert!(
        content.contains(r#""Success":"#),
        "Make sure the link creation response contains Success"
    );

    // After inserting a link make sure the link is saved
    let query = client
        .post(&(base_url.clone() + "admin/json/list_links/"))
        .headers(custom_headers.clone())
        .body(r#"{"filter":{"Code":{"sieve":""},"Description":{"sieve":""},"Target":{"sieve":""},"Author":{"sieve":""},"Statistics":{"sieve":""}},"order":null,"amount":20}"#).build().unwrap();
    println!("{:?}", query);
    let response = client
        .execute(query)
        .await
        .expect("Failed to execute request.");
    println!("List urls response:\n {:?}", response);

    // Make sure the list was retrieved and the status codes are correct
    assert!(response.status().is_success());

    // Make sure that the content now contains the newly created link
    let content = response.text().await.unwrap();
    println!("Content: {:?}", content);
    assert!(
        content.contains(r#""target":"https://crates.io/crates/pslink","code":"x""#),
        "the new target and the new code are not in the result"
    );
    assert!(
        content.contains(r#""target":"https://github.com/enaut/pslink","code":"test""#),
        "the new target and the new code are not in the result"
    );

    // After inserting two links make sure the filters work (searching for a description containing se)
    let query = client
        .post(&(base_url.clone() + "admin/json/list_links/"))
        .headers(custom_headers.clone())
        .body(r#"{"filter":{"Code":{"sieve":""},"Description":{"sieve":"se"},"Target":{"sieve":""},"Author":{"sieve":""},"Statistics":{"sieve":""}},"order":null,"amount":20}"#).build().unwrap();
    println!("{:?}", query);
    let response = client
        .execute(query)
        .await
        .expect("Failed to execute request.");
    println!("List urls response:\n {:?}", response);

    // Make sure the list was retrieved and the status codes are correct
    assert!(response.status().is_success());

    // Make sure that the content now contains the newly created link
    let content = response.text().await.unwrap();
    println!("Content: {:?}", content);
    // Code x should be in the result but not code test
    assert!(
        content.contains(r#""target":"https://crates.io/crates/pslink","code":"x""#),
        "the new target and the new code are not in the result"
    );
    assert!(
        !content.contains(r#""target":"https://github.com/enaut/pslink","code":"test""#),
        "the new target and the new code are not in the result"
    );

    // Make sure we are redirected correctly.
    let response = client
        .get(&(base_url.clone() + "test"))
        .send()
        .await
        .expect("Failed to execute request.");

    // The basic redirection is working!
    assert!(response.status().is_redirection());
    let location = response.headers().get("location").unwrap();
    assert!(location
        .to_str()
        .unwrap()
        .contains("https://github.com/enaut/pslink"));

    // And for the second link - also check that casing is correctly ignored
    let response = client
        .get(&(base_url.clone() + "X"))
        .send()
        .await
        .expect("Failed to execute request.");

    // The basic redirection is working!
    assert!(response.status().is_redirection());
    let location = response.headers().get("location").unwrap();
    assert!(location
        .to_str()
        .unwrap()
        .contains("https://crates.io/crates/pslink"));
    drop(server);
}

/// Returns the crate's binary as a `Command` that can be used for integration
/// tests.
///
/// # Arguments
///
/// * `bin_name` - The name of the binary you want to test.
///
/// # Remarks
///
/// It panics on error. This is by design so the test that uses it fails.
pub fn get_test_bin(bin_name: &str) -> tokio::process::Command {
    // Create full path to binary
    let mut path = get_test_bin_dir();
    path.push(bin_name);
    path.set_extension(std::env::consts::EXE_EXTENSION);

    assert!(path.exists());

    // Create command
    tokio::process::Command::new(path.into_os_string())
}

/// Returns the directory of the crate's binary.
///
/// # Remarks
///
/// It panics on error. This is by design so the test that uses it fails.
fn get_test_bin_dir() -> std::path::PathBuf {
    // Cargo puts the integration test binary in target/debug/deps
    let current_exe =
        std::env::current_exe().expect("Failed to get the path of the integration test binary");
    let current_dir = current_exe
        .parent()
        .expect("Failed to get the directory of the integration test binary");

    let test_bin_dir = current_dir
        .parent()
        .expect("Failed to get the binary folder");
    test_bin_dir.to_owned()
}
