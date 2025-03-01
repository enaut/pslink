use dioxus::prelude::*;
use pslink_shared::apirequests::general::EditMode;

use crate::links::{EditDialog, OptionEditDialog};
const FLOATING_BUTTON_STYLES: Asset = asset!("/assets/styling/floating_button.css");

#[component]
pub fn NewLinkButton(edit_link: Signal<Option<EditDialog>>) -> Element {
    rsx! {
        document::Stylesheet { href: FLOATING_BUTTON_STYLES }
        div { class: "fixed-button",
            button {
                class: "button is-primary is-floating",
                onclick: move |_| {
                    edit_link
                        .set_edit_dialog(
                            None,
                            String::new(),
                            String::new(),
                            String::new(),
                            None,
                            EditMode::Create,
                        );
                },
                "+"
            }
        }
    }
}
