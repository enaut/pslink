use dioxus::prelude::*;

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
    // Build cool things ✌️
    let _logged_user = use_context_provider(|| PslinkContext::default());

    rsx! {
        // Global app resources
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        Router::<ui::navbar::Route> {}
    }
}
