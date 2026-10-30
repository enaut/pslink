use dioxus::prelude::*;

use crate::{domain::requests::EditMode, ui::links::EditDialog};

const FLOATING_BUTTON_STYLES: Asset = asset!("/assets/styling/floating_button.css");
const PLUS_ICON: Asset = asset!("/assets/plus.svg");

#[component]
pub fn NewLinkButton(mut edit_link: Signal<Option<EditDialog>>) -> Element {
    rsx! {
        div { class: "fixed-button",
            button {
                class: "button is-primary is-huge is-floating",
                onclick: move |_| {
                    edit_link.set(Some(EditDialog::new(
                        None,
                        String::new(),
                        String::new(),
                        String::new(),
                        None,
                        EditMode::Create,
                    )));
                },
                img { style: "max-width: 300%", src: PLUS_ICON }
            }
            document::Stylesheet { href: FLOATING_BUTTON_STYLES }
        }
    }
}
