use dioxus::logger::tracing::info;
use dioxus::prelude::*;
use dioxus_i18n::t;
use indexmap::IndexMap;
use pslink_shared::apirequests::general::EditMode;
use pslink_shared::apirequests::users::Role;
use pslink_shared::datatypes::{Clicks, Count, FullLink};

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
    let ll = use_memo(move || links().get(&current_code).cloned());
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
        if let Some(link_data) = ll() {
            let code = link_data.link.code.clone();
            let url = generate_url_for_code(&code, &hostname());
            generate_svg_qr_from_url(&url)
        } else {
            String::new()
        }
    });
    let stats = use_memo(move || {
        if let Some(link_data) = ll() {
            link_stats()
                .get(&link_data.link.code)
                .cloned()
                .unwrap_or_else(|| link_data.clicks)
        } else {
            Clicks::Count(Count { number: 0 })
        }
    });

    rsx! {
        document::Stylesheet { href: VANISHING_MESSAGE }

        if let Some(link_data) = ll() {
            tr {
                onclick: {
                    let link_data_clone = link_data.clone();
                    let hostname_clone = hostname();
                    move |_| {
                        if let Some(user_data) = user() {
                            info!("Edit link {:?}", user_data.role);
                            if user_data.role != Role::Admin && user_data.id != link_data_clone.link.author {
                                nachricht.set(Some(t!("links-error-not-author")));
                                timer.restart();
                            } else {
                                link_signal
                                    .set_edit_dialog(
                                        Some(link_data_clone.link.id),
                                        link_data_clone.link.code.clone(),
                                        link_data_clone.link.title.clone(),
                                        link_data_clone.link.target.clone(),
                                        None,
                                        EditMode::Edit,
                                        &hostname_clone,
                                    );
                            }
                        }
                    }
                },

                td { "{link_data.link.code}" }
                td { "{link_data.link.title}" }
                td {
                    style: "max-width:70%;word-wrap:anywhere;",
                    if let Some(msg) = nachricht() {
                        div { class: "is-danger notification vanishing-message", "{msg}" }
                    }
                    "{link_data.link.target}"
                }
                td { "{link_data.user.username}" }
                td {
                    Stats {
                        clicks: stats() }
                }
                td { class: "table_qr", dangerous_inner_html: qr_code_svg }
                td {
                    onclick: {
                        let link_data_clone = link_data.clone();
                        let hostname_clone = hostname();
                        move |e| {
                            info!("Delete link");
                            link_signal
                                .set_edit_dialog(
                                    Some(link_data_clone.link.id),
                                    link_data_clone.link.code.clone(),
                                    link_data_clone.link.title.clone(),
                                    link_data_clone.link.target.clone(),
                                    None,
                                    EditMode::Delete(false),
                                    &hostname_clone,
                                );
                            e.stop_propagation();
                        }
                    },
                    img { src: TRASH_SVG, class: "trashicon" }
                }
            }
        }
    }
}
