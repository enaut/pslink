use axum::http::StatusCode;
use axum::{
    extract::Path,
    response::{Html, IntoResponse, Redirect, Response},
};
use dioxus::logger::tracing::info;
use pslink_shared::datatypes::Link;

use crate::models::{LinkDbOperations as _, NewClick};

pub async fn redirect(Path(data): Path<String>) -> Result<impl IntoResponse, StatusCode> {
    info!("Redirecting to {:?}", data);
    let link = Link::get_link_by_code(&data).await;
    info!("link: {:?}", link);
    match link {
        Ok(link) => {
            NewClick::new(link.id).insert_click().await.unwrap();
            Ok(redirect_builder(&link.target))
        }
        Err(_e) => {
            info!("Link was not found: http://pslink3.de/{}", &data);
            let mut response = Html(
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
            )
            .into_response();
            *response.status_mut() = StatusCode::NOT_FOUND;
            Ok(response)
        }
    }
}

pub async fn redirect_empty() -> impl IntoResponse {
    redirect(Path("".to_string())).await.unwrap()
}
fn redirect_builder(target: &str) -> Response {
    Redirect::temporary(target).into_response()
}
