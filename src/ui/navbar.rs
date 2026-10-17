use dioxus::{logger::tracing::info, prelude::*};
use dioxus_i18n::{prelude::i18n, t, unic_langid::langid};

use crate::{
    api::{auth_api::get_session_info, user_api::set_user_language},
    domain::{BULMA_CSS, models::Lang},
    ui::{PslinkContext, home::Home, links::Links, login::LoginScreen, users::Users},
};

const NAVBAR_CSS: Asset = asset!("/assets/styling/navbar.css");

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(WebNavbar)]
    #[route("/login")]
    LoginScreen {},
    #[route("/links")]
    Links {},
    #[route("/users")]
    Users {},
    #[route("/")]
    Home {},
    #[route("/:..route")]
    PageNotFound { route: Vec<String> },
}

#[component]
pub fn Navbar(children: Element) -> Element {
    let mut language_selector = i18n();
    let PslinkContext {
        user: mut user_signal,
        hostname: mut hostname_signal,
        base_url: mut base_url_signal,
        brand_name: mut brand_name_signal,
        is_demo: mut is_demo_signal,
    } = use_context::<PslinkContext>();

    let change_to_english = move |_| async move {
        info!("Changing to English");
        language_selector.set_language(langid!("en-US"));
        if user_signal().is_some() {
            let _ = set_user_language(Lang::EnUS).await;
        }
    };
    let change_to_german = move |_| async move {
        info!("Changing to German");
        language_selector.set_language(langid!("de-DE"));
        if user_signal().is_some() {
            let _ = set_user_language(Lang::DeDE).await;
        }
    };
    let nav = navigator();

    let _session_loader = use_resource(move || async move {
        if let Ok(session) = get_session_info().await {
            if let Some(user) = session.user {
                user_signal.set(Some(user));
            } else {
                info!("No user found in session");
                nav.push(Route::LoginScreen {});
            }
            hostname_signal.set(session.hostname);
            base_url_signal.set(session.base_url);
            brand_name_signal.set(session.brand_name);
            is_demo_signal.set(session.is_demo);
        }
    });

    rsx!(
        nav {
            document::Stylesheet { href: NAVBAR_CSS }
            document::Stylesheet { href: BULMA_CSS }
            if user_signal().is_some() {
                ol {
                    li {
                        Link { to: Route::Links {}, {t!("short_urls")} }
                    }
                    li {
                        Link { to: Route::Users {}, {t!("users")} }
                    }
                }
            } else {
                ol {}
            }
            ol {
                li {
                    Link { to: Route::Home {}, "{brand_name_signal} ({hostname_signal})" }
                }
            }
            ol {
                li {
                    div { class: "languageselector",
                        {t!("language")}
                        a { onclick: change_to_german, "de" }
                        a { onclick: change_to_english, "en" }
                    }
                }
                if let Some(user) = user_signal.cloned() {
                    li {
                        div { {t!("welcome-user", username : user.username)} }
                    }
                    li {
                        a {
                            onclick: move |_| {
                                async move {
                                    info!("Logging out");
                                    let res = crate::api::auth_api::logout().await;
                                    info!("Logout result: {:?}", res);
                                    user_signal.set(None);
                                    nav.push(Route::LoginScreen {});
                                }
                            },
                            {t!("logout")}
                        }
                    }
                } else {
                    li {
                        Link { to: Route::LoginScreen {}, {t!("login")} }
                    }
                }
            }
        }
    )
}

#[component]
pub fn WebNavbar() -> Element {
    rsx! {
        div { class: "app-container",
            Navbar {}
            DemoWarning {}
            Outlet::<Route> {}
        }
    }
}

#[component]
pub fn PageNotFound(route: Vec<String>) -> Element {
    rsx! {
        div { class: "container",
            div { class: "section",
                div { class: "columns is-centered",
                    div { class: "column is-half",
                        div { class: "notification is-danger",
                            h1 { class: "title", {t!("page-not-found")} }
                            p { class: "subtitle", {t!("page-not-found-text")} }
                            p { {t!("requested-route", route : route.join("/"))} }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn DemoWarning() -> Element {
    let PslinkContext { is_demo, .. } = use_context::<PslinkContext>();
    if is_demo() {
        rsx! {
            div { class: "notification is-danger mx-6 my-3", {t!("demo-warning")} }
        }
    } else {
        rsx! {}
    }
}
