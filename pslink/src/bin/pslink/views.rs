use std::time::SystemTime;

use actix_identity::Identity;
use actix_web::{
    http::header::{CacheControl, CacheDirective, ContentType, Expires},
    web, HttpRequest, HttpResponse,
};
use argonautica::Verifier;
use fluent_langneg::{
    convert_vec_str_to_langids_lossy, negotiate_languages, parse_accepted_languages,
    NegotiationStrategy,
};
use fluent_templates::LanguageIdentifier;
use image::{DynamicImage, ImageOutputFormat, Luma};
use pslink::queries::{authenticate, RoleGuard};
use qrcode::QrCode;
use shared::{
    apirequests::{
        general::{Message, Status},
        links::{LinkDelta, LinkRequestForm},
        users::{LoginUser, UserDelta, UserRequestForm},
    },
    datatypes::Lang,
};
use tracing::{error, info, instrument, warn};

use pslink::queries;
use pslink::ServerError;

#[instrument]
fn redirect_builder(target: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .set(CacheControl(vec![
            CacheDirective::NoCache,
            CacheDirective::NoStore,
            CacheDirective::MustRevalidate,
        ]))
        .set(Expires(SystemTime::now().into()))
        .set_header(actix_web::http::header::LOCATION, target)
        .body(format!("Redirect to {}", target))
}

#[instrument]
fn detect_language(request: &HttpRequest) -> Result<Lang, ServerError> {
    let requested = parse_accepted_languages(
        request
            .headers()
            .get(actix_web::http::header::ACCEPT_LANGUAGE)
            .ok_or_else(|| ServerError::User("Failed to get Accept_Language".to_owned()))?
            .to_str()
            .map_err(|_| {
                ServerError::User("Failed to convert Accept_language to str".to_owned())
            })?,
    );
    info!("accepted languages: {:?}", requested);
    let available = convert_vec_str_to_langids_lossy(&["de", "en"]);
    info!("available languages: {:?}", available);
    let default: LanguageIdentifier = "en"
        .parse()
        .map_err(|_| ServerError::User("Failed to parse a langid.".to_owned()))?;

    let supported = negotiate_languages(
        &requested,
        &available,
        Some(&default),
        NegotiationStrategy::Filtering,
    );
    info!("supported languages: {:?}", supported);

    if let Some(languagecode) = supported.get(0) {
        info!("Supported Language: {}", languagecode);
        Ok(languagecode
            .to_string()
            .parse()
            .expect("Failed to parse 2 language"))
    } else {
        info!("Unsupported language using default!");
        Ok("enEN".parse::<Lang>().unwrap())
    }
}

#[instrument()]
pub async fn wasm_app(config: web::Data<crate::ServerConfig>) -> Result<HttpResponse, ServerError> {
    Ok(HttpResponse::Ok().body(
        r#"<!DOCTYPE html>
        <html>
        <head>
          <meta charset="utf-8" />
          <meta name="viewport" content="width=device-width, initial-scale=1, shrink-to-fit=no" />
          <meta name="author" content="Franz Dietrich">
          <meta http-equiv="robots" content="[noindex|nofollow]">
          <link rel="stylesheet" href="/static/style.css">
          <link rel="stylesheet" href="/static/admin.css">
          <title>Pslink your urls</title>
        </head>
        <body>
          <section id="app"><div class="lds-ellipsis">Loading: <div></div><div></div><div></div><div></div></div></section>
          <script type="module">
            import init from '/static/wasm/app.js';
            init('/static/wasm/app_bg.wasm');
          </script>
        </body>
        </html>"#,
    ))
}

#[instrument(skip(id))]
pub async fn index_json(
    config: web::Data<crate::ServerConfig>,
    form: web::Json<LinkRequestForm>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    info!("Listing Links to Json api");
    match queries::list_all_allowed(&id, &config, form.0).await {
        Ok(links) => Ok(HttpResponse::Ok().json2(&links.list)),
        Err(e) => {
            error!("Failed to access database: {:?}", e);
            warn!("Not logged in - redirecting to login page");
            Ok(HttpResponse::Unauthorized().body("Failed"))
        }
    }
}

#[instrument(skip(id))]
pub async fn index_users_json(
    config: web::Data<crate::ServerConfig>,
    form: web::Json<UserRequestForm>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    info!("Listing Users to Json api");
    if let Ok(users) = queries::list_users(&id, &config, form.0).await {
        Ok(HttpResponse::Ok().json2(&users.list))
    } else {
        Ok(redirect_builder("/admin/login"))
    }
}

pub async fn get_logged_user_json(
    config: web::Data<crate::ServerConfig>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    let user = authenticate(&id, &config).await?;
    match user {
        RoleGuard::NotAuthenticated | RoleGuard::Disabled => {
            Ok(HttpResponse::Unauthorized().finish())
        }
        RoleGuard::Regular { user } | RoleGuard::Admin { user } => {
            Ok(HttpResponse::Ok().json2(&user))
        }
    }
}

#[instrument(skip(id))]
pub async fn download_png(
    id: Identity,
    config: web::Data<crate::ServerConfig>,
    link_code: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    match queries::get_link(&id, &link_code.0, &config).await {
        Ok(query) => {
            let qr = QrCode::with_error_correction_level(
                &format!("http://{}/{}", config.public_url, &query.item.code),
                qrcode::EcLevel::L,
            )
            .unwrap();
            let png = qr.render::<Luma<u8>>().quiet_zone(false).build();
            let mut temporary_data = std::io::Cursor::new(Vec::new());
            DynamicImage::ImageLuma8(png)
                .write_to(&mut temporary_data, ImageOutputFormat::Png)
                .unwrap();
            let image_data = temporary_data.into_inner();
            Ok(HttpResponse::Ok().set(ContentType::png()).body(image_data))
        }
        Err(e) => Err(e),
    }
}

#[instrument(skip(id))]
pub async fn process_create_user_json(
    config: web::Data<crate::ServerConfig>,
    data: web::Json<UserDelta>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    info!("Listing Users to Json api");
    match queries::create_user(&id, data.into_inner(), &config).await {
        Ok(item) => Ok(HttpResponse::Ok().json2(&Status::Success(Message {
            message: format!("Successfully saved user: {}", item.item.username),
        }))),
        Err(e) => Err(e),
    }
}

#[instrument(skip(id))]
pub async fn process_update_user_json(
    config: web::Data<crate::ServerConfig>,
    form: web::Json<UserDelta>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    info!("Listing Users to Json api");
    match queries::update_user(&id, &form, &config).await {
        Ok(item) => Ok(HttpResponse::Ok().json2(&Status::Success(Message {
            message: format!("Successfully saved user: {}", item.item.username),
        }))),
        Err(e) => Err(e),
    }
}

#[instrument(skip(id))]
pub async fn toggle_admin(
    user: web::Json<UserDelta>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    let update = queries::toggle_admin(&id, user.id, &config).await?;
    Ok(HttpResponse::Ok().json2(&Status::Success(Message {
        message: format!(
            "Successfully changed privileges or user: {}",
            update.item.username
        ),
    })))
}

#[instrument(skip(id))]
pub async fn get_language(
    id: Option<Identity>,
    config: web::Data<crate::ServerConfig>,
    req: HttpRequest,
) -> Result<HttpResponse, ServerError> {
    if let Some(id) = id {
        let user = authenticate(&id, &config).await?;
        match user {
            RoleGuard::NotAuthenticated | RoleGuard::Disabled => {
                Ok(HttpResponse::Ok().json2(&detect_language(&req)?))
            }
            RoleGuard::Regular { user } | RoleGuard::Admin { user } => {
                Ok(HttpResponse::Ok().json2(&user.language))
            }
        }
    } else {
        Ok(HttpResponse::Ok().json2(&detect_language(&req)?))
    }
}

#[instrument(skip(id))]
pub async fn set_language(
    data: web::Json<Lang>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    queries::set_language(&id, data.0, &config).await?;
    Ok(HttpResponse::Ok().json2(&data.0))
}

#[instrument(skip(id))]
pub async fn process_login_json(
    data: web::Json<LoginUser>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    // query the username to see if a user by that name exists.
    let user = queries::get_user_by_name(&data.username, &config).await;

    match user {
        Ok(u) => {
            // get the password hash
            if let Some(hash) = &u.password.secret {
                // get the servers secret
                let secret = &config.secret;
                // validate the secret
                let valid = Verifier::default()
                    .with_hash(hash)
                    .with_password(&data.password)
                    .with_secret_key(secret.secret.as_ref().expect("No secret available"))
                    .verify()?;

                // login the user
                if valid {
                    info!("Log-in of user: {}", &u.username);
                    let session_token = u.username.clone();
                    id.remember(session_token);
                    Ok(HttpResponse::Ok().json2(&u))
                } else {
                    info!("Invalid password for user: {}", &u.username);
                    Ok(HttpResponse::Unauthorized().json2(&Status::Error(Message {
                        message: "Failed to Login".to_string(),
                    })))
                }
            } else {
                // should fail earlier if secret is missing.
                Ok(HttpResponse::Unauthorized().json2(&Status::Error(Message {
                    message: "Failed to Login".to_string(),
                })))
            }
        }
        Err(e) => {
            info!("Failed to login: {}", e);
            Ok(HttpResponse::Unauthorized().json2(&Status::Error(Message {
                message: "Failed to Login".to_string(),
            })))
        }
    }
}

#[instrument(skip(id))]
pub async fn logout(id: Identity) -> Result<HttpResponse, ServerError> {
    info!("Logging out the user");
    id.forget();
    Ok(redirect_builder("/app/"))
}

#[instrument()]
pub async fn redirect(
    config: web::Data<crate::ServerConfig>,
    data: web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse, ServerError> {
    info!("Redirecting to {:?}", data);
    let link = queries::get_link_simple(&data.0, &config).await;
    info!("link: {:?}", link);
    match link {
        Ok(link) => {
            queries::click_link(link.id, &config).await?;
            Ok(redirect_builder(&link.target))
        }
        Err(ServerError::Database(e)) => {
            info!(
                "Link was not found: http://{}/{} \n {}",
                &config.public_url, &data.0, e
            );
            Ok(HttpResponse::NotFound().body(
                r#"<!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="utf-8">
                <title>{{title}}</title>
                <meta name="author" content="Franz Dietrich">
                <meta http-equiv="robots" content="[noindex|nofollow]">
                <link rel="stylesheet" href="/static/style.css">
            </head>
            <body>
                <div class="content">
                This link was either deleted or does not exist.
                </div>
            </body>
            </html>"#,
            ))
        }
        Err(e) => Err(e),
    }
}

#[instrument]
pub async fn redirect_empty(
    config: web::Data<crate::ServerConfig>,
) -> Result<HttpResponse, ServerError> {
    Ok(redirect_builder(&config.empty_forward_url))
}

#[instrument(skip(id))]
pub async fn process_create_link_json(
    config: web::Data<crate::ServerConfig>,
    data: web::Json<LinkDelta>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    let new_link = queries::create_link(&id, data.into_inner(), &config).await;
    match new_link {
        Ok(item) => Ok(HttpResponse::Ok().json2(&Status::Success(Message {
            message: format!("Successfully saved link: {}", item.item.code),
        }))),
        Err(e) => Err(e),
    }
}

#[instrument(skip(id))]
pub async fn process_update_link_json(
    config: web::Data<crate::ServerConfig>,
    data: web::Json<LinkDelta>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    let new_link = queries::update_link(&id, data.into_inner(), &config).await;
    match new_link {
        Ok(item) => Ok(HttpResponse::Ok().json2(&Status::Success(Message {
            message: format!("Successfully updated link: {}", item.item.code),
        }))),
        Err(e) => Err(e),
    }
}

#[instrument(skip(id))]
pub async fn process_delete_link_json(
    id: Identity,
    config: web::Data<crate::ServerConfig>,
    data: web::Json<LinkDelta>,
) -> Result<HttpResponse, ServerError> {
    queries::delete_link(&id, &data.code, &config).await?;
    Ok(HttpResponse::Ok().json2(&Status::Success(Message {
        message: format!("Successfully deleted link: {}", &data.code),
    })))
}
