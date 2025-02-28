#[cfg(feature = "server")]
use crate::models::UserDbOperations as _;
use dioxus::{
    logger::tracing::info,
    prelude::{server, server_fn, ServerFnError},
};
use pslink_shared::{apirequests::users::SessionInfo, datatypes::User};

#[server(Login)]
pub async fn login(username: String, password: String) -> Result<User, ServerFnError> {
    use argon2::{Params, PasswordVerifier as _};
    let auth = crate::auth::get_session().await?;
    let secret = crate::get_secret();

    let user = User::get_user_by_name(&username).await;
    match user {
        Ok(user) => {
            let user_password = user.password.secret.clone().expect("Secret available");
            let parsed_hash = match argon2::PasswordHash::new(&user_password) {
                Ok(h) => h,
                Err(e) => {
                    info!(
                        "Failed to parse password hash for {}: {}",
                        &user.username, e
                    );
                    return Err(ServerFnError::new("Unauthorized"));
                }
            };
            match argon2::Argon2::new_with_secret(
                &secret.secret.clone().unwrap().into_bytes(),
                argon2::Algorithm::default(),
                argon2::Version::default(),
                Params::default(),
            )
            .expect("Failed to create argon2 instance")
            .verify_password(password.as_bytes(), &parsed_hash)
            {
                Ok(_) => {
                    auth.login_user(user.id);
                    Ok(user)
                }
                Err(e) => {
                    info!("Failed to verify password for {}: {}", &user.username, e);
                    return Err(ServerFnError::new("Unauthorized"));
                }
            }
        }
        Err(e) => Err(e.into()),
    }
}

#[server(Logout)]
pub async fn logout() -> Result<(), ServerFnError> {
    let auth = crate::auth::get_session().await?;
    auth.logout_user();
    Ok(())
}

#[server(GetSessionInfo)]
pub async fn get_session_info() -> Result<SessionInfo, ServerFnError> {
    let auth = crate::auth::get_session().await;
    let auth = match auth {
        Ok(auth) => auth,
        Err(e) => {
            println!("Error: {:?}", e);
            return Err(e);
        }
    };
    //tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    let user = auth.current_user;
    let user = match user {
        Some(aa) => aa.get_user(),
        None => None,
    };
    Ok(SessionInfo { user })
}
