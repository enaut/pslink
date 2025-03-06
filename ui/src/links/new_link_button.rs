use dioxus::prelude::*;
use pslink_shared::apirequests::general::EditMode;

use crate::{
    PslinkContext,
    links::{EditDialog, OptionEditDialog},
};
const FLOATING_BUTTON_STYLES: Asset = asset!("/assets/styling/floating_button.css");
const PLUS_ICON: Asset = asset!("/assets/plus.svg");

#[component]
pub fn NewLinkButton(edit_link: Signal<Option<EditDialog>>) -> Element {
    let PslinkContext { hostname, .. } = use_context::<PslinkContext>();
    rsx! {
        document::Stylesheet { href: FLOATING_BUTTON_STYLES }
        div { class: "fixed-button",
            button {
                class: "button is-primary is-huge is-floating",
                onclick: move |_| {
                    edit_link
                        .set_edit_dialog(
                            None,
                            String::new(),
                            String::new(),
                            String::new(),
                            None,
                            EditMode::Create,
                            &hostname(),
                        );
                },
                img { style: "max-width: 300%", src: PLUS_ICON }
            }
        }
    }
}
