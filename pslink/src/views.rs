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
use qrcode::{render::svg, QrCode};
use queries::{authenticate, Role};
use shared::apirequests::{
    general::{Message, Status},
    links::{LinkDelta, LinkRequestForm, QrCodeRequest, SvgQrCodeResponse},
    users::{LoginUser, UserDelta, UserRequestForm},
};
use tera::{Context, Tera};
use tracing::{error, info, instrument, warn};

use crate::forms::LinkForm;
use crate::models::NewUser;
use crate::queries;
use crate::ServerError;

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
fn detect_language(request: &HttpRequest) -> Result<String, ServerError> {
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
    let available = convert_vec_str_to_langids_lossy(&["de", "en"]);
    let default: LanguageIdentifier = "en"
        .parse()
        .map_err(|_| ServerError::User("Failed to parse a langid.".to_owned()))?;

    let supported = negotiate_languages(
        &requested,
        &available,
        Some(&default),
        NegotiationStrategy::Filtering,
    );
    let languagecode = supported
        .get(0)
        .map_or("en".to_string(), std::string::ToString::to_string);
    Ok(languagecode)
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
          <title>Server integration example</title>
        </head>
        <body>
          <section id="app"><div class="lds-ellipsis">Loading: <div></div><div></div><div></div><div></div></div></section>
          <script type="module">
            import init from '/app/pkg/app.js';
            init('/app/pkg/app_bg.wasm');
          </script>
        </body>
        </html>"#,
    ))
}

/// Show the list of all available links if a user is authenticated

#[instrument(skip(id, tera))]
pub async fn index(
    tera: web::Data<Tera>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    match queries::list_all_allowed(&id, &config, LinkRequestForm::default()).await {
        Ok(links) => {
            let mut data = Context::new();
            data.insert("user", &links.user);
            data.insert("title", &format!("Links der {}", &config.brand_name,));
            data.insert("links_per_users", &links.list);
            let rendered = tera.render("index.html", &data)?;
            Ok(HttpResponse::Ok().body(rendered))
        }
        Err(e) => {
            error!("Failed to get the links: {:?}", e);
            Ok(redirect_builder("/admin/login/"))
        }
    }
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

/// Show the list of all available links if a user is authenticated
#[instrument(skip(id, tera))]
pub async fn index_users(
    tera: web::Data<Tera>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    if let Ok(users) = queries::list_users(&id, &config, UserRequestForm::default()).await {
        let mut data = Context::new();
        data.insert("user", &users.user);
        data.insert("title", &format!("Benutzer der {}", &config.brand_name,));
        data.insert("users", &users.list);

        let rendered = tera.render("index_users.html", &data)?;
        Ok(HttpResponse::Ok().body(rendered))
    } else {
        Ok(redirect_builder("/admin/login"))
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
        Role::NotAuthenticated | Role::Disabled => Ok(HttpResponse::Unauthorized().finish()),
        Role::Regular { user } | Role::Admin { user } => Ok(HttpResponse::Ok().json2(&user)),
    }
}

#[instrument(skip(id, tera))]
pub async fn view_link_empty(
    tera: web::Data<Tera>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    view_link(tera, config, id, web::Path::from("".to_owned())).await
}

pub async fn get_qr_code_json(
    config: web::Data<crate::ServerConfig>,
    qr_request: web::Json<QrCodeRequest>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    if let Ok(link) = queries::get_link(&id, &qr_request.link_id, &config).await {
        let host = config.public_url.to_string();
        let qr = QrCode::with_error_correction_level(
            &format!("http://{}/{}", &host, &link.item.code),
            qrcode::EcLevel::L,
        )?;

        let svg = qr
            .render()
            .min_dimensions(100, 100)
            .dark_color(svg::Color("#000000"))
            .light_color(svg::Color("#ffffff"))
            .build();

        Ok(HttpResponse::Ok().json2(&SvgQrCodeResponse { svg }))
    } else {
        Ok(redirect_builder("/admin/login/"))
    }
}

#[instrument(skip(id, tera))]
pub async fn view_link(
    tera: web::Data<Tera>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
    link_id: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    if let Ok(link) = queries::get_link(&id, &link_id.0, &config).await {
        let host = config.public_url.to_string();
        let protocol = config.protocol.to_string();
        let qr = QrCode::with_error_correction_level(
            &format!("http://{}/{}", &host, &link.item.code),
            qrcode::EcLevel::L,
        )?;

        let svg = qr
            .render()
            .min_dimensions(100, 100)
            .dark_color(svg::Color("#000000"))
            .light_color(svg::Color("#ffffff"))
            .build();

        let mut data = Context::new();
        data.insert("user", &link.user);
        data.insert(
            "title",
            &format!("Links {} der {}", &link.item.code, &config.brand_name,),
        );
        data.insert("link", &link.item);
        data.insert("qr", &svg);
        data.insert("host", &host);
        data.insert("protocol", &protocol);

        let rendered = tera.render("view_link.html", &data)?;
        Ok(HttpResponse::Ok().body(rendered))
    } else {
        Ok(redirect_builder("/admin/login/"))
    }
}

#[instrument(skip(id, tera))]
pub async fn view_profile(
    tera: web::Data<Tera>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
    user_id: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    info!("Viewing Profile!");
    if let Ok(query) = queries::get_user(&id, &user_id.0, &config).await {
        let mut data = Context::new();
        data.insert("user", &query.user);
        data.insert(
            "title",
            &format!(
                "Benutzer {} der {}",
                &query.item.username, &config.brand_name,
            ),
        );
        data.insert("viewed_user", &query.item);

        let rendered = tera.render("view_profile.html", &data)?;
        Ok(HttpResponse::Ok().body(rendered))
    } else {
        // Parsing error -- do something else
        Ok(redirect_builder("/admin/login/"))
    }
}

#[instrument(skip(id, tera))]
pub async fn edit_profile(
    tera: web::Data<Tera>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
    user_id: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    info!("Editing Profile!");
    if let Ok(query) = queries::get_user(&id, &user_id.0, &config).await {
        let mut data = Context::new();
        data.insert("user", &query.user);
        data.insert(
            "title",
            &format!(
                "Benutzer {} der {}",
                &query.user.username, &config.brand_name,
            ),
        );
        data.insert("user", &query.user);

        let rendered = tera.render("edit_profile.html", &data)?;
        Ok(HttpResponse::Ok().body(rendered))
    } else {
        Ok(redirect_builder("/admin/login/"))
    }
}

#[instrument(skip(id))]
pub async fn process_edit_profile(
    data: web::Form<NewUser>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
    user_id: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    let query = queries::update_user(&id, &user_id.0, &config, &data).await?;
    Ok(redirect_builder(&format!(
        "admin/view/profile/{}",
        query.user.username
    )))
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

#[instrument(skip(id, tera))]
pub async fn signup(
    tera: web::Data<Tera>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    match queries::authenticate(&id, &config).await? {
        queries::Role::Admin { user } => {
            let mut data = Context::new();
            data.insert("title", "Ein Benutzerkonto erstellen");
            data.insert("user", &user);

            let rendered = tera.render("signup.html", &data)?;
            Ok(HttpResponse::Ok().body(rendered))
        }
        queries::Role::Regular { .. }
        | queries::Role::NotAuthenticated
        | queries::Role::Disabled => Ok(redirect_builder("/admin/login/")),
    }
}

#[instrument(skip(id))]
pub async fn process_signup(
    data: web::Form<NewUser>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    info!("Creating a User: {:?}", &data);
    match queries::create_user(&id, &data, &config).await {
        Ok(item) => {
            Ok(HttpResponse::Ok().body(format!("Successfully saved user: {}", item.item.username)))
        }
        Err(e) => Err(e),
    }
}

#[instrument(skip(id))]
pub async fn process_create_user_json(
    config: web::Data<crate::ServerConfig>,
    form: web::Json<UserDelta>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    info!("Listing Users to Json api");
    match queries::create_user_json(&id, &form, &config).await {
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
    match queries::update_user_json(&id, &form, &config).await {
        Ok(item) => Ok(HttpResponse::Ok().json2(&Status::Success(Message {
            message: format!("Successfully saved user: {}", item.item.username),
        }))),
        Err(e) => Err(e),
    }
}

#[instrument(skip(id))]
pub async fn toggle_admin(
    data: web::Path<String>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    let update = queries::toggle_admin(&id, &data.0, &config).await?;
    Ok(redirect_builder(&format!(
        "/admin/view/profile/{}",
        update.item.id
    )))
}

#[instrument(skip(id))]
pub async fn set_language(
    data: web::Path<String>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    queries::set_language(&id, &data.0, &config).await?;
    Ok(redirect_builder("/admin/index/"))
}

#[instrument(skip(tera, id))]
pub async fn login(
    tera: web::Data<Tera>,
    id: Identity,
    config: web::Data<crate::ServerConfig>,
    req: HttpRequest,
) -> Result<HttpResponse, ServerError> {
    let language_code = detect_language(&req).unwrap_or_else(|_| "en".to_string());
    info!("Detected languagecode: {}", &language_code);
    let mut data = Context::new();
    data.insert("title", "Login");
    data.insert("language", &language_code);

    if id.identity().is_some() {
        if let Ok(r) = authenticate(&id, &config).await {
            match r {
                Role::Admin { user } | Role::Regular { user } => {
                    info!(
                        "This user ({}) is already logged in redirecting to /admin/index/",
                        user.username
                    );
                    return Ok(redirect_builder("/admin/index/"));
                }
                Role::Disabled | Role::NotAuthenticated => (),
            }
        }
        warn!("Invalid user session. The user might be deleted or something tampered with the cookies.");
        id.forget();
    }

    let rendered = tera.render("login.html", &data)?;
    Ok(HttpResponse::Ok().body(rendered))
}

#[instrument(skip(id))]
pub async fn process_login(
    data: web::Form<LoginUser>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    let user = queries::get_user_by_name(&data.username, &config).await;

    match user {
        Ok(u) => {
            if let Some(hash) = u.password.secret {
                let secret = &config.secret;
                let valid = Verifier::default()
                    .with_hash(hash)
                    .with_password(&data.password)
                    .with_secret_key(secret.secret.as_ref().expect("No secret available"))
                    .verify()?;

                if valid {
                    info!("Log-in of user: {}", &u.username);
                    let session_token = u.username;
                    id.remember(session_token);
                    Ok(redirect_builder("/admin/index/"))
                } else {
                    Ok(redirect_builder("/admin/login/"))
                }
            } else {
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
                    Ok(redirect_builder("/admin/login/"))
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
    Ok(redirect_builder("/admin/login/"))
}

#[instrument(skip(tera))]
pub async fn redirect(
    tera: web::Data<Tera>,
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
            let mut data = Context::new();
            data.insert("title", "Wurde gel\u{f6}scht");
            let language = detect_language(&req).unwrap_or_else(|_| "en".to_string());
            data.insert("language", &language);
            let rendered = tera.render("not_found.html", &data)?;
            Ok(HttpResponse::NotFound().body(rendered))
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
    let new_link = queries::create_link_json(&id, data, &config).await;
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
    let new_link = queries::update_link_json(&id, data, &config).await;
    match new_link {
        Ok(item) => Ok(HttpResponse::Ok().json2(&Status::Success(Message {
            message: format!("Successfully updated link: {}", item.item.code),
        }))),
        Err(e) => Err(e),
    }
}

#[instrument(skip(id))]
pub async fn create_link(
    tera: web::Data<Tera>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    match queries::authenticate(&id, &config).await? {
        queries::Role::Admin { user } | queries::Role::Regular { user } => {
            let mut data = Context::new();
            data.insert("title", "Einen Kurzlink erstellen");

            data.insert("user", &user);
            let rendered = tera.render("submission.html", &data)?;
            Ok(HttpResponse::Ok().body(rendered))
        }
        queries::Role::NotAuthenticated | queries::Role::Disabled => {
            Ok(redirect_builder("/admin/login/"))
        }
    }
}

#[instrument(skip(id))]
pub async fn process_link_creation(
    data: web::Form<LinkForm>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    let new_link = queries::create_link(&id, data, &config).await?;
    Ok(redirect_builder(&format!(
        "/admin/view/link/{}",
        new_link.item.code
    )))
}

#[instrument(skip(id))]
pub async fn edit_link(
    tera: web::Data<Tera>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
    link_id: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    if let Ok(query) = queries::get_link(&id, &link_id.0, &config).await {
        let mut data = Context::new();
        data.insert("title", "Submit a Post");
        data.insert("link", &query.item);

        data.insert("user", &query.user);
        let rendered = tera.render("edit_link.html", &data)?;
        return Ok(HttpResponse::Ok().body(rendered));
    }
    Ok(redirect_builder("/admin/login/"))
}
pub async fn process_link_edit(
    data: web::Form<LinkForm>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
    link_code: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    match queries::update_link(&id, &link_code.0, data, &config).await {
        Ok(query) => Ok(redirect_builder(&format!(
            "/admin/view/link/{}",
            &query.item.code
        ))),
        Err(e) => Err(e),
    }
}

#[instrument(skip(id))]
pub async fn process_link_delete(
    id: Identity,
    config: web::Data<crate::ServerConfig>,
    link_code: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    queries::delete_link(&id, &link_code.0, &config).await?;
    Ok(redirect_builder("/admin/login/"))
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
