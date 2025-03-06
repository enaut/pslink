use crate::{PslinkContext, home::Home, links::Links, login::LoginScreen, users::Users};
use backend::{auth_api::get_session_info, user_api::set_user_language};
use dioxus::{logger::tracing::info, prelude::*};
use dioxus_i18n::{prelude::i18n, t, unic_langid::langid};
use pslink_shared::datatypes::Lang;

const NAVBAR_CSS: Asset = asset!("/assets/styling/navbar.css");
const BULMA_CSS: Asset = asset!("/assets/styling/bulma.css");

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
    // PageNotFound is a catch all route that will match any route and placing the matched segments in the route field
    #[route("/:..route")]
    PageNotFound { route: Vec<String> },
}

#[component]
pub fn Navbar(children: Element) -> Element {
    let mut language_selector = i18n();

    let change_to_english = move |_| async move {
        info!("Changing to English");
        language_selector.set_language(langid!("en-US"));
        set_user_language(Lang::EnUS)
            .await
            .expect("Failed to set language");
    };
    let change_to_german = move |_| async move {
        info!("Changing to German");
        language_selector.set_language(langid!("de-DE"));
        set_user_language(Lang::DeDE)
            .await
            .expect("Failed to set language");
    };
    let PslinkContext {
        user: mut user_signal,
        hostname: mut hostname_signal,
    } = use_context::<PslinkContext>();
    let nav = navigator();

    let _ = use_resource(move || async move {
        if let Ok(session) = get_session_info().await {
            if let Some(user) = session.user {
                user_signal.set(Some(user));
            } else {
                info!("No user found in session");
                nav.push(Route::LoginScreen {});
            }
            hostname_signal.set(session.hostname);
        }
    });

    rsx!(
        document::Stylesheet { href: NAVBAR_CSS }
        document::Stylesheet { href: BULMA_CSS }
        nav {
            if let Some(_user) = user_signal.cloned() {
                ol {
                    li {
                        Link { to: Route::Links {}, {t!("short_urls")} } // The menu entry for links
                    }
                    li {
                        Link { to: Route::Users {}, {t!("users")} } // The menu entry for users
                    }
                }
            } else {
                ol {}
            }
            ol {
                li {
                    Link { to: Route::Home {}, {hostname_signal} }
                }
            }
            ol {
                li {
                    div { class: "languageselector",
                        {t!("language")} // The menu entry for language selection
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
                                    let res = backend::auth_api::logout().await;
                                    info!("Logout result: {:?}", res);
                                    user_signal.set(None);
                                    nav.push(Route::LoginScreen {});
                                }
                            },
                            {t!("logout")} // The menu entry for logout
                        }
                    }
                } else {
                    li {
                        Link { to: Route::LoginScreen {}, {t!("login")} } // The menu entry for login
                    }
                }

            }
        }
    )
}

#[component]
pub fn WebNavbar() -> Element {
    rsx! {
        Navbar {}
        Outlet::<Route> {}
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
                            h1 { class: "title", {t!("page-not-found")} } // The title of the page
                            p { class: "subtitle", {t!("page-not-found-text")} } // The text of the page
                            p { {t!("requested-route", route : route.join("/"))} } // The requested route on the 404 page
                        }
                    }
                }
            }
        }
    }
}
