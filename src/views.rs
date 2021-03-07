use std::time::SystemTime;

use actix_identity::Identity;
use actix_web::{
    http::header::{CacheControl, CacheDirective, ContentType, Expires},
    web, HttpResponse,
};
use argonautica::Verifier;
use image::{DynamicImage, ImageOutputFormat, Luma};
use qrcode::{render::svg, QrCode};
use tera::{Context, Tera};

use super::forms::LinkForm;
use super::models::{LoginUser, NewUser};
use crate::queries;
use crate::ServerError;

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

/// Show the list of all available links if a user is authenticated
pub(crate) async fn index(
    tera: web::Data<Tera>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    if let Ok(links) = queries::list_all_allowed(&id, &config) {
        let mut data = Context::new();
        data.insert("user", &links.user);
        data.insert("title", "Links der Freien Hochschule Stuttgart");
        data.insert("links_per_users", &links.list);
        let rendered = tera.render("index.html", &data)?;
        Ok(HttpResponse::Ok().body(rendered))
    } else {
        Ok(redirect_builder("/admin/login/"))
    }
}

/// Show the list of all available links if a user is authenticated
pub(crate) async fn index_users(
    tera: web::Data<Tera>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    if let Ok(users) = queries::list_users(&id, &config) {
        let mut data = Context::new();
        data.insert("user", &users.user);
        data.insert("title", "Benutzer der Freien Hochschule Stuttgart");
        data.insert("users", &users.list);

        let rendered = tera.render("index_users.html", &data)?;
        Ok(HttpResponse::Ok().body(rendered))
    } else {
        Ok(redirect_builder("/admin/login"))
    }
}
pub(crate) async fn view_link_fhs(
    tera: web::Data<Tera>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    view_link(tera, config, id, web::Path::from("".to_owned())).await
}

pub(crate) async fn view_link(
    tera: web::Data<Tera>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
    link_id: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    if let Ok(link) = queries::get_link(&id, &link_id.0, &config) {
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
            &format!("Links {} der Freien Hochschule Stuttgart", &link.item.code),
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

pub(crate) async fn view_profile(
    tera: web::Data<Tera>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
    user_id: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    slog_info!(config.log, "Viewing Profile!");
    if let Ok(query) = queries::get_user(&id, &user_id.0, &config) {
        let mut data = Context::new();
        data.insert("user", &query.user);
        data.insert(
            "title",
            &format!(
                "Benutzer {} der Freien Hochschule Stuttgart",
                &query.item.username
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

pub(crate) async fn edit_profile(
    tera: web::Data<Tera>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
    user_id: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    slog_info!(config.log, "Editing Profile!");
    if let Ok(query) = queries::get_user(&id, &user_id.0, &config) {
        let mut data = Context::new();
        data.insert("user", &query.user);
        data.insert(
            "title",
            &format!(
                "Benutzer {} der Freien Hochschule Stuttgart",
                &query.user.username
            ),
        );
        data.insert("user", &query.user);

        let rendered = tera.render("edit_profile.html", &data)?;
        Ok(HttpResponse::Ok().body(rendered))
    } else {
        Ok(redirect_builder("/admin/login/"))
    }
}

pub(crate) async fn process_edit_profile(
    data: web::Form<NewUser>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
    user_id: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    if let Ok(query) = queries::update_user(&id, &user_id.0, &config, &data) {
        Ok(redirect_builder(&format!(
            "admin/view/profile/{}",
            query.user.username
        )))
    } else {
        Ok(redirect_builder("/admin/index/"))
    }
}

pub(crate) async fn download_png(
    id: Identity,
    config: web::Data<crate::ServerConfig>,
    link_code: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    match queries::get_link(&id, &link_code.0, &config) {
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

pub(crate) async fn signup(
    tera: web::Data<Tera>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    match queries::authenticate(&id, &config)? {
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

pub(crate) async fn process_signup(
    data: web::Form<NewUser>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    slog_info!(config.log, "Creating a User: {:?}", &data);
    if let Ok(item) = queries::create_user(&id, &data, &config) {
        Ok(HttpResponse::Ok().body(format!("Successfully saved user: {}", item.item.username)))
    } else {
        Ok(redirect_builder("/admin/login/"))
    }
}

pub(crate) async fn toggle_admin(
    data: web::Path<String>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    let update = queries::toggle_admin(&id, &data.0, &config)?;
    Ok(redirect_builder(&format!(
        "/admin/view/profile/{}",
        update.item.id
    )))
}

pub(crate) async fn login(
    tera: web::Data<Tera>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    let mut data = Context::new();
    data.insert("title", "Login");

    if let Some(_id) = id.identity() {
        return Ok(redirect_builder("/admin/index/"));
    }

    let rendered = tera.render("login.html", &data)?;
    Ok(HttpResponse::Ok().body(rendered))
}

pub(crate) async fn process_login(
    data: web::Form<LoginUser>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    let user = queries::get_user_by_name(&data.username, &config);

    match user {
        Ok(u) => {
            let secret = &config.secret;
            let valid = Verifier::default()
                .with_hash(&u.password)
                .with_password(&data.password)
                .with_secret_key(secret)
                .verify()?;

            if valid {
                slog_info!(config.log, "Log-in of user: {}", &u.username);
                let session_token = u.username;
                id.remember(session_token);
                Ok(redirect_builder("/admin/index/"))
            } else {
                Ok(redirect_builder("/admin/login/"))
            }
        }
        Err(e) => {
            slog_info!(config.log, "Failed to login: {}", e);
            Ok(redirect_builder("/admin/login/"))
        }
    }
}

pub(crate) async fn logout(id: Identity) -> Result<HttpResponse, ServerError> {
    id.forget();
    Ok(redirect_builder("/admin/login/"))
}

pub(crate) async fn redirect(
    tera: web::Data<Tera>,
    config: web::Data<crate::ServerConfig>,
    data: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    slog_info!(config.log, "Redirecting to {:?}", data);
    let link = queries::get_link_simple(&data.0, &config);
    slog_info!(config.log, "link: {:?}", link);
    match link {
        Ok(link) => {
            queries::click_link(link.id, &config)?;
            Ok(redirect_builder(&link.target))
        }
        Err(ServerError::Diesel(e)) => {
            slog_info!(
                config.log,
                "Link was not found: http://{}/{} \n {}",
                &config.public_url,
                &data.0,
                e
            );
            let mut data = Context::new();
            data.insert("title", "Wurde gel\u{f6}scht");
            let rendered = tera.render("not_found.html", &data)?;
            Ok(HttpResponse::NotFound().body(rendered))
        }
        Err(e) => Err(e),
    }
}

pub(crate) async fn redirect_fhs() -> Result<HttpResponse, ServerError> {
    Ok(redirect_builder(
        "https://www.freie-hochschule-stuttgart.de",
    ))
}

pub(crate) async fn create_link(
    tera: web::Data<Tera>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    match queries::authenticate(&id, &config)? {
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

pub(crate) async fn process_link_creation(
    data: web::Form<LinkForm>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    let new_link = queries::create_link(&id, data, &config)?;
    Ok(redirect_builder(&format!(
        "/admin/view/link/{}",
        new_link.item.code
    )))
}

pub(crate) async fn edit_link(
    tera: web::Data<Tera>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
    link_id: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    if let Ok(query) = queries::get_link(&id, &link_id.0, &config) {
        let mut data = Context::new();
        data.insert("title", "Submit a Post");
        data.insert("link", &query.item);

        data.insert("user", &query.user);
        let rendered = tera.render("edit_link.html", &data)?;
        return Ok(HttpResponse::Ok().body(rendered));
    }
    Ok(redirect_builder("/admin/login/"))
}
pub(crate) async fn process_link_edit(
    data: web::Form<LinkForm>,
    config: web::Data<crate::ServerConfig>,
    id: Identity,
    link_code: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    match queries::update_link(&id, &link_code.0, &data, &config) {
        Ok(query) => Ok(redirect_builder(&format!(
            "/admin/view/link/{}",
            &query.item.code
        ))),
        Err(e) => Err(e),
    }
}

pub(crate) async fn process_link_delete(
    id: Identity,
    config: web::Data<crate::ServerConfig>,
    link_code: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    queries::delete_link(&id, &link_code.0, &config)?;
    Ok(redirect_builder("/admin/login/"))
}
