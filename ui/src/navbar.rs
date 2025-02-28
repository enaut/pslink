use crate::{home::Home, links::Links, login::LoginScreen, PslinkContext};
use backend::auth_api::get_session_info;
use dioxus::{logger::tracing::info, prelude::*};

const NAVBAR_CSS: Asset = asset!("/assets/styling/navbar.css");
const BULMA_CSS: Asset = asset!("/assets/styling/bulma.css");

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(WebNavbar)]
    #[route("/login")]
    LoginScreen {},
    #[route("/app/links")]
    Links {},
    #[route("/app")]
    Home {},
    // PageNotFound is a catch all route that will match any route and placing the matched segments in the route field
    #[route("/:..route")]
    PageNotFound { route: Vec<String> },
}

#[component]
pub fn Navbar(children: Element) -> Element {
    let PslinkContext {
        user: mut user_signal,
    } = use_context::<PslinkContext>();
    let nav = navigator();

    let _ = use_resource(move || async move {
        if let Ok(session) = get_session_info().await {
            if let Some(user) = session.user {
                info!("Setting user to: {:?}", &user);
                user_signal.set(Some(user));
            } else {
                info!("No user found in session");
                nav.push(Route::LoginScreen {});
            }
        }
    });

    rsx!(
        if let Some(user) = user_signal.cloned() {
            document::Stylesheet { href: NAVBAR_CSS }
            document::Stylesheet { href: BULMA_CSS }
            nav {
                ol {
                    li {
                        Link { to: Route::Links {}, "List of existing links" }
                    }
                    li {
                        a { href: "/app/list_links/create_link", "Add a new link" }
                    }
                    li {
                        a { href: "/app/list_users/create_user", "Invite a new user" }
                    }
                    li {
                        a { href: "/app/list_users", "List of existing users" }
                    }
                }
                ol {
                    li {
                        div { class: "languageselector",
                            "Language:"
                            a { "de" }
                            a { "en" }
                        }
                    }
                    li {
                        div { "Welcome {user.username}" }
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
                            "Logout"
                        }
                    }
                }
            }
        } else {
            document::Stylesheet { href: NAVBAR_CSS }
            nav {
                ol {
                    li { "Loading..." }
                }
                ol {
                    li {
                        a { href: "/login/", "Login" }
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
                            h1 { class: "title", "404 - Page not found" }
                            p { class: "subtitle", "The page you requested could not be found." }
                            p { "Requested route: {route.join(\"/\")}" }
                        }
                    }
                }
            }
        }
    }
}
