use dioxus::logger::tracing::info;
use dioxus::prelude::*;
use indexmap::IndexMap;
use pslink_shared::apirequests::general::EditMode;
use pslink_shared::datatypes::FullLink;

use crate::links::stats::Stats;
use crate::links::EditDialog;
use crate::links::OptionEditDialog as _;
const TRASH_SVG: Asset = asset!("/assets/trash.svg");

#[component]
pub fn LinkDisplay(
    current_code: String,
    links: Resource<IndexMap<String, FullLink>>,
    link_signal: Signal<Option<EditDialog>>,
) -> Element {
    let ll = use_memo(move || {
        links
            .as_ref()
            .expect("Links loaded")
            .get(&current_code)
            .unwrap()
            .clone()
    });

    rsx! {
        tr {
            onclick: move |_| {
                info!("Edit link");
                link_signal
                    .set_edit_dialog(
                        Some(ll().link.id),
                        ll().link.code.clone(),
                        ll().link.title.clone(),
                        ll().link.target.clone(),
                        None,
                        EditMode::Edit,
                    );
            },

            td { "{ll().link.code}" }
            td { "{ll().link.title}" }
            td { "{ll().link.target}" }
            td { "{ll().user.username}" }
            td {
                Stats { clicks: ll().clicks }
            }
            td { class: "table_qr" }
            td {
                onclick: move |e| {
                    info!("Delete link");
                    link_signal
                        .set_edit_dialog(
                            Some(ll().link.id),
                            ll().link.code.clone(),
                            ll().link.title.clone(),
                            ll().link.target.clone(),
                            None,
                            EditMode::Delete(false),
                        );
                    e.stop_propagation();
                },
                img { src: TRASH_SVG, class: "trashicon" }
            }
        }
    }
}
