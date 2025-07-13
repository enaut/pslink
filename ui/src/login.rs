use dioxus::{
    logger::tracing::{info, trace},
    prelude::*,
};
use dioxus_i18n::t;
use std::rc::Rc;

use crate::{PslinkContext, navbar::Route};

const LOGIN_CSS: Asset = asset!("/assets/styling/login.css");

#[component]
pub fn LoginScreen() -> Element {
    let mut username = use_signal(|| "".to_string());
    let mut password = use_signal(|| "".to_string());
    let PslinkContext { mut user, .. } = use_context::<PslinkContext>();
    let nav = navigator();
    let mut username_field: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    let mut password_field: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    let mut nachricht: Signal<Option<String>> = use_signal(|| None);

    let _focus_grabber = use_resource(move || async move {
        if let Some(field) = username_field().as_ref() {
            match field.set_focus(true).await {
                Ok(_) => trace!("Username field focused"),
                Err(e) => info!("Failed to focus username field: {:?}", e),
            }
        } else {
            info!("Username field not available for focus");
        }
    });

    trace!("Rendering login screen with username: {}", username);
    rsx! {
        document::Stylesheet { href: LOGIN_CSS }
        div { class: "modal is-active",
            div { class: "modal-background" }
            div { class: "modal-card",
                header { class: "modal-card-head",
                    p { class: "modal-card-title", {t!("headline-login")} }
                }
                div { class: "modal-card-body",
                    div { class: "field is-horizontal is-wider",
                        div { class: "field-label is-normal",
                            label { class: "label", {t!("username")} }
                        }
                        div { class: "field-body",
                            div { class: "field",
                                p { class: "control",
                                    input {
                                        autofocus: true,
                                        onmounted: move |e| {
                                            username_field.set(Some(e.data()));
                                        },
                                        r#type: "text",
                                        class: "input",
                                        placeholder: t!("username"),
                                        value: "{username}",
                                        oninput: move |e| {
                                            username.set(e.value());
                                        },
                                    }
                                }
                            }
                        }
                    }
                    div { class: "field is-horizontal is-wider",
                        div { class: "field-label is-normal",
                            label { class: "label", {t!("password")} }
                        }
                        div { class: "field-body",
                            div { class: "field",
                                p { class: "control",
                                    input {
                                        r#type: "password",
                                        class: "input",
                                        placeholder: t!("password"),
                                        value: "{password}",
                                        oninput: move |e| {
                                            password.set(e.value());
                                        },
                                        onmounted: move |e| {
                                            password_field.set(Some(e.data()));
                                        },
                                    }
                                }
                            }
                        }
                    }
                    match nachricht().as_ref() {
                        Some(nachricht) => {
                            rsx! {
                                div { class: "notification is-danger", {nachricht.clone()} }
                            }
                        }
                        None => rsx! {},
                    }
                }
                footer { class: "modal-card-foot is-justify-content-flex-end",
                    div { class: "buttons",
                        button {
                            class: "button is-primary",
                            onclick: move |_| {
                                info!("Login button clicked with username: {}", username());
                                async move {
                                    match backend::auth_api::login(username(), password()).await {
                                        Ok(u) => {
                                            user.set(Some(u));
                                            nav.push(Route::Links {});
                                        }
                                        Err(e) => {
                                            let fehlernachricht = t!("failed-login", error : e.to_string());
                                            nachricht.set(Some(fehlernachricht));
                                            info!("Failed to login: {:?}", e);
                                        }
                                    }
                                }
                            },
                            {t!("login")}
                        }
                    }
                }
            }
        }
    }
}
