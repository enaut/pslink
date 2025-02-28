use dioxus::{logger::tracing::info, prelude::*};

use crate::{navbar::Route, PslinkContext};

const BLOG_CSS: Asset = asset!("/assets/styling/login.css");

#[component]
pub fn LoginScreen() -> Element {
    let mut username = use_signal(|| "".to_string());
    let mut password = use_signal(|| "".to_string());
    let PslinkContext { mut user } = use_context::<PslinkContext>();
    let nav = navigator();

    info!("Rendering login screen with username: {}", username);
    rsx! {
        document::Stylesheet { href: BLOG_CSS }
        form { onsubmit: move |event| { info!("Submitted! {event:?}") },
            div { class: "center login",
                h1 { "Login {user:?}" }
                div {
                    label { "Benutzername" }
                    input {
                        r#type: "text",
                        autofocus: true,
                        value: "{username}",
                        placeholder: "Benutzername",
                        oninput: move |e| {
                            username.set(e.value());
                        },
                    }
                }
                div {
                    label { "Passwort" }
                    input {
                        r#type: "password",
                        placeholder: "Passwort",
                        value: "{password}",
                        oninput: move |e| {
                            info!("Setting password to: {}", e.value());
                            password.set(e.value());
                        },
                    }
                }
                button {
                    onclick: move |_| {
                        async move {
                            info!("Logging in with username: {}", username);
                            info!("Setting password to: {}", password);
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
                    "Login"
                }
            }
        }
    }
}
