use dioxus::prelude::*;

use crate::{
    domain::{models::Role, requests::EditMode},
    ui::users::EditDialog,
};

const FLOATING_BUTTON_STYLES: Asset = asset!("/assets/styling/floating_button.css");
const PLUS_ICON: Asset = asset!("/assets/plus.svg");

#[component]
pub fn NewUserButton(mut edit_dialog_signal: Signal<Option<EditDialog>>) -> Element {
    rsx! {
        div { class: "fixed-button",
            document::Stylesheet { href: FLOATING_BUTTON_STYLES }
            button {
                class: "button is-primary is-floating",
                onclick: move |_| {
                    edit_dialog_signal.set(Some(EditDialog::new(
                        None,
                        String::new(),
                        String::new(),
                        None,
                        Role::Regular,
                        EditMode::Create,
                    )));
                },
                img { style: "max-width: 300%", src: PLUS_ICON }
            }
        }
    }
}
