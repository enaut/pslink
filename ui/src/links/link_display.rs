use dioxus::logger::tracing::info;
use dioxus::prelude::*;
use indexmap::IndexMap;
use pslink_shared::apirequests::general::EditMode;
use pslink_shared::apirequests::users::Role;
use pslink_shared::datatypes::FullLink;

use crate::links::stats::Stats;
use crate::links::EditDialog;
use crate::links::OptionEditDialog as _;
use crate::PslinkContext;
const TRASH_SVG: Asset = asset!("/assets/trash.svg");
const VANISHING_MESSAGE: Asset = asset!("/assets/styling/vanishing_message.css");

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
    let mut nachricht = use_signal(move || None);
    let PslinkContext { user } = use_context::<PslinkContext>();
    let mut timer = use_resource(move || {
        let delay = std::time::Duration::from_secs(3);
        let mut nachricht = nachricht.clone();
        async move {
            wasmtimer::tokio::sleep(delay).await;
            nachricht.set(None);
        }
    });
    rsx! {
        document::Stylesheet { href: VANISHING_MESSAGE }

        tr {


            onclick: move |_| {
                info!("Edit link {:?}", user().unwrap().role);
                if user().unwrap().role != Role::Admin && user().unwrap().id != ll().link.author
                {
                    nachricht.set(Some("You are not the author of this link".to_string()));
                    timer.restart();
                } else {
                    link_signal
                        .set_edit_dialog(
                            Some(ll().link.id),
                            ll().link.code.clone(),
                            ll().link.title.clone(),
                            ll().link.target.clone(),
                            None,
                            EditMode::Edit,
                        );
                }
            },

            td { "{ll().link.code}" }
            td { "{ll().link.title}" }
            td {
                if nachricht().is_some() {
                    div { class: "is-danger notification vanishing-message", "{nachricht().unwrap()}" }
                }
                "{ll().link.target}"
            }
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
