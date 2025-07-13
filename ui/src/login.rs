use dioxus::{logger::tracing::info, prelude::*};
use dioxus_i18n::t;
use std::rc::Rc;

use crate::{PslinkContext, navbar::Route};

const LOGIN_CSS: Asset = asset!("/assets/styling/login.css");

// Sichere Funktion zum Überprüfen der Enter-Taste
fn is_enter_key_safe(e: &KeyboardEvent) -> bool {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| e.key() == Key::Enter)) {
        Ok(result) => result,
        Err(_) => {
            info!("Failed to check key - Chrome undefined key issue detected");
            false
        }
    }
}

fn get_value_safe(e: Event<FormData>) -> String {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| e.value())) {
        Ok(value) => value,
        Err(_) => {
            info!("Failed to get value - Chrome undefined value issue detected");
            String::new()
        }
    }
}

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
                Ok(_) => info!("Username field focused"),
                Err(e) => info!("Failed to focus username field: {:?}", e),
            }
        } else {
            info!("Username field not available for focus");
        }
    });

    info!("Rendering login screen with username: {}", username);
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
                                            info!("Username field mounted");
                                            username_field.set(Some(e.data()));
                                        },
                                        r#type: "text",
                                        class: "input",
                                        placeholder: t!("username"),
                                        value: "{username}",
                                        oninput: move |e| {
                                            info!("Username input changed: {}", e.value());
                                            username.set(get_value_safe(e));
                                        },
                                        onkeydown: move |e: KeyboardEvent| {
                                            info!("Username keydown event received");
                                            if is_enter_key_safe(&e) {
                                                info!("Enter key detected in username field");
                                                e.prevent_default();
                                                let password_field = password_field.clone();
                                                spawn(async move {
                                                    if let Some(field) = password_field().as_ref() {
                                                        match field.set_focus(true).await {
                                                            Ok(_) => info!("Password field focused"),
                                                            Err(e) => info!("Failed to focus password field: {:?}", e),
                                                        }
                                                    } else {
                                                        info!("Password field not available for focus");
                                                    }
                                                });
                                            }
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
                                            info!("Password input changed: {}", e.value());
                                            password.set(get_value_safe(e));
                                        },
                                        onmounted: move |e| {
                                            info!("Password field mounted");
                                            password_field.set(Some(e.data()));
                                        },
                                        onkeydown: move |e: KeyboardEvent| {
                                            info!("Password keydown event received");
                                            if is_enter_key_safe(&e) {
                                                info!("Enter key detected in password field - submitting login");
                                                e.prevent_default();
                                                spawn({
                                                    let username = username.clone();
                                                    let password = password.clone();
                                                    let mut user = user.clone();
                                                    let nav = nav.clone();
                                                    let mut nachricht = nachricht.clone();
                                                    async move {
                                                        match backend::auth_api::login(username(), password()).await {
                                                            Ok(u) => {
                                                                user.set(Some(u));
                                                                nav.push(Route::Links {});
                                                            }
                                                            Err(e) => {
                                                                let fehlernachricht = t!(
                                                                    "failed-login", error : e.to_string()
                                                                );
                                                                nachricht.set(Some(fehlernachricht));
                                                                info!("Failed to login: {:?}", e);
                                                            }
                                                        }
                                                    }
                                                });
                                            }
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
                                info!("Password: {}", password());
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
