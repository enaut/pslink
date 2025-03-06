use dioxus::prelude::*;

use dioxus_i18n::{
    prelude::{I18nConfig, i18n, use_init_i18n},
    unic_langid::langid,
};
use pslink_shared::datatypes::Lang;
use ui::PslinkContext;

mod views;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    #[cfg(feature = "web")]
    dioxus::launch(App);
    #[cfg(feature = "server")]
    backend::launch_server(App);
}

#[component]
fn App() -> Element {
    use_init_i18n(|| ui::translations::config(langid!("de-DE")));
    let logged_user = use_context_provider(|| PslinkContext::default());
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
        // Global app resources
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        Router::<ui::navbar::Route> {}
    }
}
