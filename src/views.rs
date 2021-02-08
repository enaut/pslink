use std::time::SystemTime;

use crate::ServerError;
use actix_identity::Identity;
use actix_web::{
    http::header::{CacheControl, CacheDirective, ContentType, Expires},
    web, HttpResponse,
};
use argonautica::Verifier;
use diesel::{prelude::*, result::Error::NotFound, sqlite::SqliteConnection};
use dotenv::dotenv;
use image::{DynamicImage, ImageOutputFormat, Luma};
use qrcode::{render::svg, QrCode};
use tera::{Context, Tera};

use super::forms::LinkForm;
use super::models::{Link, LoginUser, NewLink, NewUser, User};

fn establish_connection() -> Result<SqliteConnection, ServerError> {
    dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")?;

    match SqliteConnection::establish(&database_url) {
        Ok(c) => Ok(c),
        Err(e) => {
            info!("Error connecting to database: {}, {}", database_url, e);
            Err(ServerError::User(
                "Error connecting to Database".to_string(),
            ))
        }
    }
}

fn redirect_builder(target: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .set(CacheControl(vec![
            CacheDirective::NoCache,
            CacheDirective::NoStore,
            CacheDirective::MustRevalidate,
        ]))
        .set(Expires(SystemTime::now().into()))
        .set_header(actix_web::http::header::LOCATION, target.clone())
        .body(format!("Redirect to {}", target.clone()))
}

/// Show the list of all available links if a user is authenticated
pub(crate) async fn index(
    tera: web::Data<Tera>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    use super::schema::links::dsl::links;
    use super::schema::users::dsl::users;
    if let Some(id) = id.identity() {
        let connection = establish_connection()?;
        let all_links: Vec<(Link, User)> = links.inner_join(users).load(&connection)?;

        let mut data = Context::new();
        data.insert("name", &id);
        data.insert("title", "Links der Freien Hochschule Stuttgart");
        data.insert("links_per_users", &all_links);

        let rendered = tera.render("index.html", &data)?;
        Ok(HttpResponse::Ok().body(rendered))
    } else {
        Ok(redirect_builder("/admin/login/"))
    }
}

pub(crate) async fn view_link(
    tera: web::Data<Tera>,
    id: Identity,
    link_id: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    use super::schema::links::dsl::{code, links};
    if let Some(id) = id.identity() {
        let connection = establish_connection()?;
        let link: Link = links
            .filter(code.eq(&link_id.0))
            .first::<Link>(&connection)?;

        let qr = QrCode::with_error_correction_level(
            &format!("http://fhs.li/{}", &link_id),
            qrcode::EcLevel::L,
        )
        .unwrap();
        let svg = qr
            .render()
            .min_dimensions(200, 200)
            .dark_color(svg::Color("#000000"))
            .light_color(svg::Color("#ffffff"))
            .build();

        let mut data = Context::new();
        data.insert("name", &id);
        data.insert(
            "title",
            &format!("Links {} der Freien Hochschule Stuttgart", link_id.0),
        );
        data.insert("link", &link);
        data.insert("qr", &svg);

        let rendered = tera.render("view_link.html", &data)?;
        Ok(HttpResponse::Ok().body(rendered))
    } else {
        Ok(redirect_builder("/admin/login/"))
    }
}

pub(crate) async fn view_profile(
    tera: web::Data<Tera>,
    identity: Identity,
    user_id: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    use super::schema::users::dsl::{id, users};
    println!("Viewing Profile!");
    if let Some(identity) = identity.identity() {
        let connection = establish_connection()?;
        if let Ok(uid) = user_id.parse::<i32>() {
            let user = users.filter(id.eq(&uid)).first::<User>(&connection)?;

            let mut data = Context::new();
            data.insert("name", &identity);
            data.insert(
                "title",
                &format!(
                    "Benutzer {} der Freien Hochschule Stuttgart",
                    &user.username
                ),
            );
            data.insert("user", &user);

            let rendered = tera.render("view_profile.html", &data)?;
            Ok(HttpResponse::Ok().body(rendered))
        } else {
            Ok(redirect_builder("/admin/index/"))
        }
    } else {
        Ok(redirect_builder("/admin/login/"))
    }
}

pub(crate) async fn edit_profile(
    tera: web::Data<Tera>,
    identity: Identity,
    user_id: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    use super::schema::users::dsl::{id, users};
    println!("Viewing Profile!");
    if let Some(identity) = identity.identity() {
        let connection = establish_connection()?;
        if let Ok(uid) = user_id.parse::<i32>() {
            let user = users.filter(id.eq(&uid)).first::<User>(&connection)?;

            let mut data = Context::new();
            data.insert("name", &identity);
            data.insert(
                "title",
                &format!(
                    "Benutzer {} der Freien Hochschule Stuttgart",
                    &user.username
                ),
            );
            data.insert("user", &user);

            let rendered = tera.render("edit_profile.html", &data)?;
            Ok(HttpResponse::Ok().body(rendered))
        } else {
            Ok(redirect_builder("/admin/index/"))
        }
    } else {
        Ok(redirect_builder("/admin/login/"))
    }
}

pub(crate) async fn process_edit_profile(
    data: web::Form<NewUser>,
    id: Identity,
    user_id: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    if let Some(_id) = id.identity() {
        use super::schema::users::dsl::{email, id, password, username, users};

        if let Ok(uid) = user_id.parse::<i32>() {
            println!("Updating userinfo: ");
            let connection = establish_connection()?;
            diesel::update(users.filter(id.eq(uid)))
                .set((
                    username.eq(data.username.clone()),
                    email.eq(data.email.clone()),
                ))
                .execute(&connection)?;
            if data.password.len() > 3 {
                let hash = NewUser::hash_password(data.password.clone())?;
                diesel::update(users.filter(id.eq(uid)))
                    .set((password.eq(hash),))
                    .execute(&connection)?;
            }
            Ok(HttpResponse::Ok().body(format!("Successfully saved user: {}", data.username)))
        } else {
            Ok(redirect_builder("/admin/index/"))
        }
    } else {
        Ok(redirect_builder("/admin/login/"))
    }
}

pub(crate) async fn download_png(
    id: Identity,
    link_id: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    if let Some(_id) = id.identity() {
        use super::schema::links::dsl::{code, links};
        let connection = establish_connection()?;
        if let Ok(_link) = links.filter(code.eq(&link_id.0)).first::<Link>(&connection) {
            let qr = QrCode::with_error_correction_level(
                &format!("http://fhs.li/{}", &link_id),
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
        } else {
            Ok(redirect_builder("/admin/index/"))
        }
    } else {
        Ok(redirect_builder("/admin/login/"))
    }
}

pub(crate) async fn signup(
    tera: web::Data<Tera>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    if let Some(id) = id.identity() {
        let mut data = Context::new();
        data.insert("title", "Sign Up");
        data.insert("name", &id);

        let rendered = tera.render("signup.html", &data)?;
        Ok(HttpResponse::Ok().body(rendered))
    } else {
        Ok(redirect_builder("/admin/login/"))
    }
}

pub(crate) async fn process_signup(
    data: web::Form<NewUser>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    if let Some(_id) = id.identity() {
        use super::schema::users;

        let connection = establish_connection()?;
        let new_user = NewUser::new(
            data.username.clone(),
            data.email.clone(),
            data.password.clone(),
        )?;

        diesel::insert_into(users::table)
            .values(&new_user)
            .execute(&connection)?;

        println!("{:?}", data);
        Ok(HttpResponse::Ok().body(format!("Successfully saved user: {}", data.username)))
    } else {
        Ok(redirect_builder("/admin/login/"))
    }
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
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    use super::schema::users::dsl::{username, users};

    let connection = establish_connection()?;
    let user = users
        .filter(username.eq(&data.username))
        .first::<User>(&connection);

    match user {
        Ok(u) => {
            dotenv().ok();
            let secret = std::env::var("SECRET_KEY")?;
            let valid = Verifier::default()
                .with_hash(&u.password)
                .with_password(&data.password)
                .with_secret_key(&secret)
                .verify()?;

            if valid {
                println!("Login of user: {}", &u.username);
                let session_token = u.username;
                id.remember(session_token);
                Ok(redirect_builder("/admin/index/"))
            } else {
                Ok(redirect_builder("/admin/login/"))
            }
        }
        Err(e) => {
            println!("Failed to login: {}", e);
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
    data: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    use super::schema::links::dsl::{code, links};
    let connection = establish_connection()?;

    let link = links.filter(code.eq(&data.0)).first::<Link>(&connection);
    match link {
        Ok(link) => Ok(redirect_builder(&link.target)),
        Err(NotFound) => {
            let mut data = Context::new();
            data.insert("title", "Wurde gelöscht");
            let rendered = tera.render("not_found.html", &data)?;
            Ok(HttpResponse::NotFound().body(rendered))
        }
        Err(e) => Err(e.into()),
    }
}

pub(crate) async fn redirect_fhs() -> Result<HttpResponse, ServerError> {
    Ok(redirect_builder(
        "https://www.freie-hochschule-stuttgart.de",
    ))
}

pub(crate) async fn submission(
    tera: web::Data<Tera>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    if let Some(id) = id.identity() {
        let mut data = Context::new();
        data.insert("title", "Submit a Post");

        data.insert("name", &id);
        let rendered = tera.render("submission.html", &data)?;
        return Ok(HttpResponse::Ok().body(rendered));
    }
    Ok(redirect_builder("/admin/login/"))
}

pub(crate) async fn process_submission(
    data: web::Form<LinkForm>,
    id: Identity,
) -> Result<HttpResponse, ServerError> {
    if let Some(id) = id.identity() {
        use super::schema::users::dsl::{username, users};

        let connection = establish_connection()?;
        let user: Result<User, diesel::result::Error> =
            users.filter(username.eq(id)).first(&connection);

        match user {
            Ok(u) => {
                use super::schema::links;
                let new_post = NewLink::from_link_form(data.into_inner(), u.id);

                diesel::insert_into(links::table)
                    .values(&new_post)
                    .execute(&connection)?;

                return Ok(redirect_builder("/admin/index/"));
            }
            Err(_e) => Ok(redirect_builder("/admin/login/")),
        }
    } else {
        Ok(redirect_builder("/admin/login/"))
    }
}
