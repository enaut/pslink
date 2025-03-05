use dioxus::{logger::tracing::info, prelude::*};
use dioxus_i18n::t;

use crate::{navbar::Route, PslinkContext};

const LOGIN_CSS: Asset = asset!("/assets/styling/login.css");

#[component]
pub fn LoginScreen() -> Element {
    let mut username = use_signal(|| "".to_string());
    let mut password = use_signal(|| "".to_string());
    let PslinkContext { mut user } = use_context::<PslinkContext>();
    let nav = navigator();

    info!("Rendering login screen with username: {}", username);
    rsx! {
        document::Stylesheet { href: LOGIN_CSS }
        form { onsubmit: move |event| { info!("Submitted! {event:?}") },
            div { class: "center login",
                h1 { {t!("headline-login")} } // Headline on the login screen
                div {
                    label { {t!("username")} } // Username field label on the login screen
                    input {
                        r#type: "text",
                        autofocus: true,
                        value: "{username}",
                        placeholder: t!("username"), // Username field placeholder on the login screen
                        oninput: move |e| {
                            username.set(e.value());
                        },
                    }
                }
                div {
                    label { {t!("password")} } // Password field label on the login screen
                    input {
                        r#type: "password",
                        placeholder: t!("password"), // Password field placeholder on the login screen
                        value: "{password}",
                        oninput: move |e| {
                            password.set(e.value());
                        },
                    }
                }
                button {
                    onclick: move |_| {
                        async move {
                            match backend::auth_api::login(username.to_string(), password.to_string())
                                .await
                            {
                                Ok(u) => {
                                    user.set(Some(u));
                                    nav.push(Route::Home {});
                                }
                                Err(e) => {
                                    info!("Failed to login: {:?}", e);
                                }
                            }
                        }
                    },
                    {t!("login")} // Login button text
                }
            }
        }
    }
}
