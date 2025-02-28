use crate::PslinkContext;
use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    let PslinkContext { user } = use_context::<PslinkContext>();

    rsx! {
        if let Some(user) = user.cloned() {
            h1 { "Welcome {user.username}" }
        } else {
            h1 { "Welcome stranger" }
        }
    }
}
