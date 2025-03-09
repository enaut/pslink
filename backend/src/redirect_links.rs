use axum::body::Body;
use axum::http::StatusCode;
use axum::{
    extract::Path,
    response::{Html, IntoResponse, Redirect, Response},
};
use dioxus::logger::tracing::info;
use dioxus::prelude::*;
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
            let response = generate_error_page(
                "This link was either deleted or does not exist!",
                StatusCode::NOT_FOUND,
            );

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

/// Generates a styled HTML error page using Bulma CSS
pub fn generate_error_page(
    error_message: &str,
    status_code: StatusCode,
) -> axum::http::Response<Body> {
    let bulma_path = pslink_shared::BULMA_CSS.bundled().bundled_path();
    let content = rsx! {
        section { class: "section",
            div { class: "container",
                div { class: "columns is-centered",
                    div { class: "column is-half",
                        div { class: "box has-text-centered",
                            h1 { class: "title is-2 has-text-danger", "{status_code}" }
                            p { class: "subtitle is-4 mb-5", "{error_message}" }
                            a { href: "/", class: "button is-primary", "Back to Home" }
                        }
                    }
                }
            }
        }
    };
    let content = dioxus::ssr::render_element(content);
    let html_page = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Error {status_code} - Link not found</title>
    <link rel="stylesheet" href="/app/assets/{bulma_path}">
</head>
<body>
{content}
</body>
</html>"#
    );
    let mut response = Html(html_page).into_response();
    *response.status_mut() = status_code;
    response
}
