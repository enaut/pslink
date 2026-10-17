pub mod api;
pub mod domain;
#[cfg(feature = "server")]
pub mod server;
pub mod ui;

use dioxus::prelude::*;
use dioxus_i18n::{
    prelude::{i18n, use_init_i18n},
    unic_langid::langid,
};

use crate::{domain::models::Lang, ui::PslinkContext};

fn main() {
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    {
        console_error_panic_hook::set_once();

        // Workaround for Dioxus bug: Dioxus fullstack expects an absolute URL,
        // but when base_path is set in Dioxus.toml, LaunchBuilder sets server_url to
        // relative "/app", causing Url::parse to panic with RelativeUrlWithoutBase.
        let base_path = dioxus::cli_config::base_path().unwrap_or_default();
        let base_path = base_path.trim_matches('/');
        let server_url = if base_path.is_empty() {
            "http://this.is.not.a.real.url:9000"
        } else {
            format!("http://this.is.not.a.real.url:9000/{}", base_path).leak()
        };
        dioxus::fullstack::set_server_url(server_url);

        dioxus::web::launch::launch_cfg(App, dioxus::web::Config::default());
    }
    #[cfg(feature = "server")]
    server::launch_pslink(App);
}

#[component]
fn App() -> Element {
    const FAVICON: Asset = asset!("/assets/favicon.ico");
    const MAIN_CSS: Asset = asset!("/assets/main.css");
    use_init_i18n(|| ui::translations::config(langid!("de-DE")));
    let logged_user = use_context_provider(PslinkContext::default);
    let mut language_selector = i18n();
    let _language_setter = use_memo(move || {
        if let Some(user) = logged_user.user.read().as_ref() {
            match user.language {
                Lang::DeDE => language_selector.set_language(langid!("de-DE")),
                Lang::EnUS => language_selector.set_language(langid!("en-US")),
            }
        } else {
            language_selector.set_language(langid!("en-US"));
        }
    });

    rsx! {
        div { id: "app-root",
            document::Link { rel: "icon", href: FAVICON }
            document::Link { rel: "stylesheet", href: MAIN_CSS }
            Router::<ui::navbar::Route> {}
        }
    }
}
