use crate::PslinkContext;
use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn Home() -> Element {
    let PslinkContext { user, .. } = use_context::<PslinkContext>();

    rsx! {
        if let Some(user) = user.cloned() {
            h1 { {t!("welcome", username : user.username)} } // Welcome message with the username
        } else {
            h1 { {t!("welcome-stranger")} } // Welcome message for strangers
        }
    }
}
