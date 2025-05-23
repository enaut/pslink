use dioxus::logger::tracing::info;
use dioxus::prelude::*;
use dioxus_i18n::t;
use indexmap::IndexMap;
use pslink_shared::apirequests::general::EditMode;
use pslink_shared::apirequests::users::Role;
use pslink_shared::datatypes::Clicks;
use pslink_shared::datatypes::FullLink;

use crate::PslinkContext;
use crate::links::EditDialog;
use crate::links::OptionEditDialog as _;
use crate::links::generate_svg_qr_from_url;
use crate::links::generate_url_for_code;
use crate::links::stats::Stats;
const TRASH_SVG: Asset = asset!("/assets/trash.svg");
const VANISHING_MESSAGE: Asset = asset!("/assets/styling/vanishing_message.css");

#[component]
pub fn LinkDisplay(
    current_code: String,
    links: Signal<IndexMap<String, FullLink>>,
    link_stats: Signal<IndexMap<String, Clicks>>,
    link_signal: Signal<Option<EditDialog>>,
) -> Element {
    let ll = use_memo(move || links().get(&current_code).unwrap().clone());
    let mut nachricht: Signal<Option<String>> = use_signal(move || None);
    let PslinkContext { user, hostname } = use_context::<PslinkContext>();
    let mut timer = use_resource(move || {
        let delay = std::time::Duration::from_secs(3);
        let mut nachricht = nachricht.clone();
        async move {
            wasmtimer::tokio::sleep(delay).await;
            nachricht.set(None);
        }
    });
    let qr_code_svg = use_memo(move || {
        let code = ll().link.code.clone();
        let url = generate_url_for_code(&code, &hostname());
        generate_svg_qr_from_url(&url)
    });
    let stats = use_memo(move || {
        link_stats()
            .get(&ll().link.code)
            .cloned()
            .unwrap_or_else(|| ll().clicks)
    });

    rsx! {
        document::Stylesheet { href: VANISHING_MESSAGE }

        tr {


            onclick: move |_| {
                info!("Edit link {:?}", user().unwrap().role);
                if user().unwrap().role != Role::Admin && user().unwrap().id != ll().link.author
                {
                    nachricht.set(Some(t!("links-error-not-author")));
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
                            &hostname(),
                        );
                }
            },

            td { "{ll().link.code}" }
            td { "{ll().link.title}" }
            td {
                style: "max-width:70%;word-wrap:anywhere;",
                if nachricht().is_some() {
                    div { class: "is-danger notification vanishing-message", "{nachricht().unwrap()}" }
                }
                "{ll().link.target}"
            }
            td { "{ll().user.username}" }
            td {
                Stats {
                    clicks: stats() }
            }
            td { class: "table_qr", dangerous_inner_html: qr_code_svg }
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
                            &hostname(),
                        );
                    e.stop_propagation();
                },
                img { src: TRASH_SVG, class: "trashicon" }
            }
        }
    }
}
