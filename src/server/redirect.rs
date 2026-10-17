//! HTTP redirect handlers for short links and SSR error page rendering.

use axum::{
    Extension,
    body::Body,
    extract::Path,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use dioxus::{logger::tracing::info, prelude::*};

use crate::{
    domain::BULMA_CSS,
    server::{
        db::{self, NewClick},
        state::AppState,
    },
};

pub async fn redirect(
    Extension(state): Extension<AppState>,
    Path(data): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    info!("Redirecting to {:?}", data);
    match db::get_link_by_code(&state.db, &data).await {
        Ok(link) => {
            if let Err(e) = NewClick::new(link.id).insert_click(&state.db).await {
                info!("Failed to record click for link {}: {}", link.id, e);
            }
            Ok(redirect_builder(&link.target))
        }
        Err(_e) => {
            info!("Link was not found: {}", &data);
            let response = generate_error_page(
                "This link was either deleted or does not exist!",
                StatusCode::NOT_FOUND,
                &state.config.brand_name,
            );
            Ok(response)
        }
    }
}

pub async fn redirect_empty(Extension(state): Extension<AppState>) -> impl IntoResponse {
    if let Ok(link) = db::get_link_by_code(&state.db, "").await {
        let _ = NewClick::new(link.id).insert_click(&state.db).await;
        redirect_builder(&link.target)
    } else {
        redirect_builder(&state.config.empty_forward_url)
    }
}

fn redirect_builder(target: &str) -> Response {
    Redirect::temporary(target).into_response()
}

/// Generates a styled HTML error page using Bulma CSS.
#[must_use]
pub fn generate_error_page(
    error_message: &str,
    status_code: StatusCode,
    brand_name: &str,
) -> axum::http::Response<Body> {
    let bulma_path = BULMA_CSS.bundled().bundled_path().to_string();
    let content = rsx! {
        section { class: "section",
            div { class: "container",
                div { class: "columns is-centered",
                    div { class: "column is-half",
                        div { class: "box has-text-centered",
                            h4 { class: "title is-2 has-text-info", "Error on the webpage of" }
                            h3 { class: "title is-2",
                                img { src: "/favicon.ico", alt: "{brand_name}" }
                                " {brand_name}"
                            }
                            h4 { class: "subtitle is-4 mt-5 has-text-danger", "{status_code}" }
                            p { class: "subtitle is-4 mb-5", "{error_message}" }
                            a { href: "/", class: "button is-info", "Back to the main Page" }
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
