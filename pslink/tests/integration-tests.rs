// use assert_cmd::prelude::*; // Add methods on commands
// use predicates::prelude::*;
// use std::process::Command; // Used for writing assertions

// use shared::datatypes::Secret;

// #[test]
// fn test_help_of_command_for_breaking_changes() {
//     let output = test_bin::get_test_bin("pslink")
//         .output()
//         .expect("Failed to start pslink");
//     assert!(String::from_utf8_lossy(&output.stdout).contains("USAGE"));

//     let output = test_bin::get_test_bin("pslink")
//         .args(&["--help"])
//         .output()
//         .expect("Failed to start pslink");
//     let outstring = String::from_utf8_lossy(&output.stdout);

//     let args = &[
//         "USAGE",
//         "-h",
//         "--help",
//         "-b",
//         "-e",
//         "-i",
//         "-p",
//         "-t",
//         "-u",
//         "runserver",
//         "create-admin",
//         "generate-env",
//         "migrate-database",
//         "help",
//     ];

//     for s in args {
//         assert!(
//             outstring.contains(s),
//             "{} was not found in the help - this is a breaking change",
//             s
//         );
//     }
// }

// #[test]
// fn test_generate_env() {
//     use std::io::BufRead;
//     let tmp_dir = tempdir::TempDir::new("pslink_test_env").expect("create temp dir");
//     let output = test_bin::get_test_bin("pslink")
//         .args(&["generate-env", "--secret", "abcdefghijklmnopqrstuvw"])
//         .current_dir(&tmp_dir)
//         .output()
//         .expect("Failed to start pslink");
//     let envfile = tmp_dir.path().join(".env");
//     let dbfile = tmp_dir.path().join("links.db");
//     println!("{}", envfile.display());
//     println!("{}", dbfile.display());
//     println!("{}", String::from_utf8_lossy(&output.stdout));
//     assert!(envfile.exists(), "No .env-file was created!");
//     assert!(dbfile.exists(), "No database-file was created!");

//     let envfile = std::fs::File::open(envfile).unwrap();
//     let envcontent: Vec<Result<String, _>> = std::io::BufReader::new(envfile).lines().collect();
//     assert!(
//         envcontent
//             .iter()
//             .any(|s| s.as_ref().unwrap().starts_with("PSLINK_PORT=")),
//         "Failed to find PSLINK_PORT in the generated .env file."
//     );
//     assert!(
//         envcontent
//             .iter()
//             .any(|s| s.as_ref().unwrap().starts_with("PSLINK_SECRET=")),
//         "Failed to find PSLINK_SECRET in the generated .env file."
//     );
//     assert!(
//         !envcontent.iter().any(|s| {
//             let r = s.as_ref().unwrap().contains("***SECRET***");
//             r
//         }),
//         "It seems that a censored secret was used in the .env file."
//     );
//     assert!(
//         envcontent.iter().any(|s| {
//             let r = s.as_ref().unwrap().contains("abcdefghijklmnopqrstuvw");
//             r
//         }),
//         "The secret has not made it into the .env file!"
//     );
//     let output = test_bin::get_test_bin("pslink")
//         .args(&["generate-env"])
//         .current_dir(&tmp_dir)
//         .output()
//         .expect("Failed to start pslink");
//     let second_out = String::from_utf8_lossy(&output.stdout);
//     assert!(!second_out.contains("secret"));
// }

// #[actix_rt::test]
// async fn test_migrate_database() {
//     use std::io::Write;
//     #[derive(serde::Serialize, Debug)]
//     pub struct Count {
//         pub number: i32,
//     }

//     let tmp_dir = tempdir::TempDir::new("pslink_test_env").expect("create temp dir");
//     // generate .env file
//     let _output = test_bin::get_test_bin("pslink")
//         .args(&["generate-env"])
//         .current_dir(&tmp_dir)
//         .output()
//         .expect("Failed generate .env");

//     // migrate the database
//     let output = test_bin::get_test_bin("pslink")
//         .args(&["migrate-database"])
//         .current_dir(&tmp_dir)
//         .output()
//         .expect("Failed to migrate the database");
//     println!("{}", String::from_utf8_lossy(&output.stdout));

//     // check if the users table exists by counting the number of admins.
//     let db_pool = sqlx::pool::Pool::<sqlx::sqlite::Sqlite>::connect(
//         &tmp_dir.path().join("links.db").display().to_string(),
//     )
//     .await
//     .expect("Error: Failed to connect to database!");
//     let num = sqlx::query_as!(Count, "select count(*) as number from users where role = 2")
//         .fetch_one(&db_pool)
//         .await
//         .unwrap();
//     // initially no admin is present
//     assert_eq!(num.number, 0, "Failed to create the database!");

//     // create a new admin
//     let mut input = test_bin::get_test_bin("pslink")
//         .args(&["create-admin"])
//         .current_dir(&tmp_dir)
//         .stdin(std::process::Stdio::piped())
//         .stdout(std::process::Stdio::piped())
//         .spawn()
//         .expect("Failed to migrate the database");
//     let mut procin = input.stdin.take().unwrap();

//     procin.write_all(b"test\n").unwrap();
//     procin.write_all(b"test@mail.test\n").unwrap();
//     procin.write_all(b"testpw\n").unwrap();

//     let r = input.wait().unwrap();
//     println!("Exitstatus is: {}", r);

//     println!("{}", String::from_utf8_lossy(&output.stdout));
//     let num = sqlx::query_as!(Count, "select count(*) as number from users where role = 2")
//         .fetch_one(&db_pool)
//         .await
//         .unwrap();
//     // now 1 admin is there
//     assert_eq!(num.number, 1, "Failed to create an admin!");
// }

// async fn run_server() {
//     use std::io::Write;
//     #[derive(serde::Serialize, Debug)]
//     pub struct Count {
//         pub number: i32,
//     }
//     let tmp_dir = tempdir::TempDir::new("pslink_test_env").expect("create temp dir");
//     // generate .env file
//     let _output = Command::cargo_bin("pslink")
//         .expect("Failed to get binary executable")
//         .args(&["generate-env", "--secret", "abcdefghijklmnopqrstuvw"])
//         .current_dir(&tmp_dir)
//         .output()
//         .expect("Failed generate .env");
//     // migrate the database
//     let output = Command::cargo_bin("pslink")
//         .args(&["migrate-database"])
//         .current_dir(&tmp_dir)
//         .output()
//         .expect("Failed to migrate the database");

//     // create a database connection.
//     let db_pool = sqlx::pool::Pool::<sqlx::sqlite::Sqlite>::connect(
//         &tmp_dir.path().join("links.db").display().to_string(),
//     )
//     .await
//     .expect("Error: Failed to connect to database!"); // create a new admin
//     let mut input = test_bin::get_test_bin("pslink")
//         .args(&["create-admin"])
//         .current_dir(&tmp_dir)
//         .stdin(std::process::Stdio::piped())
//         .stdout(std::process::Stdio::piped())
//         .spawn()
//         .expect("Failed to migrate the database");
//     let mut procin = input.stdin.take().unwrap();

//     procin.write_all(b"test\n").unwrap();
//     procin.write_all(b"test@mail.test\n").unwrap();
//     procin.write_all(b"testpw\n").unwrap();

//     let r = input.wait().unwrap();
//     println!("Exitstatus is: {}", r);

//     println!("{}", String::from_utf8_lossy(&output.stdout));
//     let num = sqlx::query_as!(Count, "select count(*) as number from users where role = 2")
//         .fetch_one(&db_pool)
//         .await
//         .unwrap();
//     // now 1 admin is there
//     assert_eq!(
//         num.number, 1,
//         "Failed to create an admin! See previous tests!"
//     );

//     let server_config = pslink::ServerConfig {
//         secret: Secret::new("abcdefghijklmnopqrstuvw".to_string()),
//         db: std::path::PathBuf::from("links.db"),
//         db_pool,
//         public_url: "localhost:8080".to_string(),
//         internal_ip: "localhost".to_string(),
//         port: 8080,
//         protocol: pslink::Protocol::Http,
//         empty_forward_url: "https://github.com/enaut/pslink".to_string(),
//         brand_name: "Pslink".to_string(),
//     };

//     let server = pslink::main::webservice(server_config);

//     let _neveruse = tokio::spawn(server);
// }

// #[actix_rt::test]
// async fn test_web_paths() {
//     run_server().await;

//     // We need to bring in `reqwest`
//     // to perform HTTP requests against our application.
//     let client = reqwest::Client::builder()
//         .cookie_store(true)
//         .redirect(reqwest::redirect::Policy::none())
//         .build()
//         .unwrap();

//     // Act
//     let response = client
//         .get("http://localhost:8080/")
//         .send()
//         .await
//         .expect("Failed to execute request.");

//     // The basic redirection is working!
//     assert!(response.status().is_redirection());
//     let location = response.headers().get("location").unwrap();
//     assert!(location.to_str().unwrap().contains("github"));

//     // Act
//     let response = client
//         .get("http://localhost:8080/admin/login/")
//         .send()
//         .await
//         .expect("Failed to execute request.");

//     // The Loginpage is reachable and contains a password field!
//     assert!(response.status().is_success());
//     let content = response.text().await.unwrap();
//     assert!(
//         content.contains(r#"<input type="password"#),
//         "No password field was found!"
//     );

//     // Act
//     let formdata = &[("username", "test"), ("password", "testpw")];
//     let response = client
//         .post("http://localhost:8080/admin/login/")
//         .form(formdata)
//         .send()
//         .await
//         .expect("Failed to execute request.");

//     // It is possible to login
//     assert!(response.status().is_redirection());
//     let location = response.headers().get("location").unwrap();
//     assert_eq!("/admin/index/", location.to_str().unwrap());
//     assert!(
//         response.headers().get("set-cookie").is_some(),
//         "A auth cookie is not set even though authentication succeeds"
//     );

//     // After login this should return a redirect
//     let response = client
//         .get("http://localhost:8080/admin/login/")
//         .send()
//         .await
//         .expect("Failed to execute request.");

//     // The Loginpage redirects to link index when logged in
//     assert!(
//         response.status().is_redirection(),
//         "/admin/login/ is not redirecting correctly when logged in!"
//     );
//     let location = response.headers().get("location").unwrap();
//     assert_eq!("/admin/index/", location.to_str().unwrap());

//     // After login this should return a redirect
//     let response = client
//         .get("http://localhost:8080/admin/index/")
//         .send()
//         .await
//         .expect("Failed to execute request.");

//     // The Loginpage redirects to link index when logged in
//     assert!(
//         response.status().is_success(),
//         "Could not access /admin/index/"
//     );
//     let content = response.text().await.unwrap();
//     assert!(
//         content.contains(r#"<a href="/admin/logout/">"#),
//         "No Logout Button was found on /admin/index/!"
//     );

//     // Act title=haupt&target=http%3A%2F%2Fdas.geht%2Fjetzt%2F&code=tpuah
//     let formdata = &[
//         ("title", "haupt"),
//         ("target", "https://das.geht/jetzt/"),
//         ("code", "tpuah"),
//     ];
//     let response = client
//         .post("http://localhost:8080/admin/submit/")
//         .form(formdata)
//         .send()
//         .await
//         .expect("Failed to execute request.");

//     // It is possible to login
//     assert!(response.status().is_redirection());
//     let location = response.headers().get("location").unwrap();
//     assert_eq!("/admin/view/link/tpuah", location.to_str().unwrap());

//     // Act
//     let response = client
//         .get("http://localhost:8080/tpuah")
//         .send()
//         .await
//         .expect("Failed to execute request.");

//     // The basic redirection is working!
//     assert!(response.status().is_redirection());
//     let location = response.headers().get("location").unwrap();
//     assert!(location
//         .to_str()
//         .unwrap()
//         .contains("https://das.geht/jetzt/"));
// }
