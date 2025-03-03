use dioxus::prelude::*;
use pslink_shared::apirequests::general::EditMode;
use pslink_shared::apirequests::users::Role;

use super::OptionUserEditDialog as _;

use super::EditDialog;

const FLOATING_BUTTON_STYLES: Asset = asset!("/assets/styling/floating_button.css");
const PLUS_ICON: Asset = asset!("/assets/plus.svg");

#[component]
pub fn NewUserButton(edit_dialog_signal: Signal<Option<EditDialog>>) -> Element {
    rsx! {
        document::Stylesheet { href: FLOATING_BUTTON_STYLES }
        div { class: "fixed-button",
            button {
                class: "button is-primary is-floating",
                onclick: move |_| {
                    edit_dialog_signal
                        .set_edit_dialog(
                            None,
                            String::new(),
                            String::new(),
                            None,
                            Role::Regular,
                            EditMode::Create,
                        );
                },
                img { style: "max-width: 300%", src: PLUS_ICON }
            }
        }
    }
}
