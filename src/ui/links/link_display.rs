use dioxus::{logger::tracing::info, prelude::*};
use dioxus_i18n::t;
use indexmap::IndexMap;

use crate::{
    domain::{
        models::{CheckStatus, Clicks, Count, FullLink, Role},
        requests::EditMode,
    },
    ui::{
        PslinkContext,
        links::{
            EditDialog, generate_svg_qr_from_url, generate_url_for_code, link_check::CheckDot,
            stats::Stats,
        },
    },
};

const TRASH_SVG: Asset = asset!("/assets/trash.svg");

#[component]
pub fn LinkDisplay(
    current_code: String,
    links: Signal<IndexMap<String, FullLink>>,
    mut link_stats: Signal<IndexMap<String, Clicks>>,
    mut link_signal: Signal<Option<EditDialog>>,
) -> Element {
    let ll = use_memo(move || links().get(&current_code).cloned());
    let mut nachricht: Signal<Option<String>> = use_signal(|| None);
    let mut copied = use_signal(|| false);
    let PslinkContext {
        user,
        hostname,
        base_url,
        ..
    } = use_context::<PslinkContext>();

    let mut timer = use_resource(move || {
        let delay = std::time::Duration::from_secs(3);
        async move {
            wasmtimer::tokio::sleep(delay).await;
            nachricht.set(None);
        }
    });

    let short_url = use_memo(move || {
        ll().map(|link_data| generate_url_for_code(&link_data.link.code, &base_url(), &hostname()))
            .unwrap_or_default()
    });

    let qr_code_svg = use_memo(move || {
        let url = short_url();
        if url.is_empty() {
            String::new()
        } else {
            generate_svg_qr_from_url(&url)
        }
    });

    let stats = use_memo(move || {
        if let Some(link_data) = ll() {
            link_stats()
                .get(&link_data.link.code)
                .cloned()
                .unwrap_or(link_data.clicks)
        } else {
            Clicks::Count(Count { number: 0 })
        }
    });

    rsx! {
        if let Some(link_data) = ll() {
            tr {
                class: if link_data.last_check.as_ref().is_some_and(|c| c.status == CheckStatus::Failed) { "link-failed" } else { "" },
                onclick: {
                    let link_data_clone = link_data.clone();
                    move |_| {
                        if let Some(user_data) = user() {
                            info!("Edit link {:?}", user_data.role);
                            if user_data.role != Role::Admin && user_data.id != link_data_clone.link.author {
                                nachricht.set(Some(t!("links-error-not-author")));
                                timer.restart();
                            } else {
                                link_signal.set(Some(EditDialog::new(
                                    Some(link_data_clone.link.id),
                                    link_data_clone.link.code.clone(),
                                    link_data_clone.link.title.clone(),
                                    link_data_clone.link.target.clone(),
                                    Some(link_data_clone.link.author),
                                    EditMode::Edit,
                                )));
                            }
                        }
                    }
                },

                td {
                    div { class: "table-cell-with-action",
                        span { "{link_data.link.code}" }
                        button {
                            class: "button is-light is-small table-action-btn",
                            r#type: "button",
                            title: t!("link-edit-copy"),
                            onclick: move |e: Event<MouseData>| {
                                e.stop_propagation();
                                let url = short_url();
                                async move {
                                    if let Some(window) = web_sys::window() {
                                        let _ = window.navigator().clipboard().write_text(&url);
                                    }
                                    copied.set(true);
                                    wasmtimer::tokio::sleep(std::time::Duration::from_secs(2)).await;
                                    copied.set(false);
                                }
                            },
                            if copied() {
                                "✓"
                            } else {
                                "📋"
                            }
                        }
                    }
                }
                td { "{link_data.link.title}" }
                td {
                    style: "max-width:70%;word-wrap:anywhere;",
                    if let Some(msg) = nachricht() {
                        div { class: "is-danger notification vanishing-message", "{msg}" }
                    }
                    div { class: "table-cell-with-action",
                        span {
                            CheckDot { check: link_data.last_check.clone() }
                            "{link_data.link.target}"
                        }
                        a {
                            class: "button is-light is-small table-action-btn",
                            href: "{link_data.link.target}",
                            target: "_blank",
                            rel: "noopener noreferrer",
                            title: t!("link-preview-open"),
                            onclick: move |e: Event<MouseData>| {
                                e.stop_propagation();
                            },
                            "↗"
                        }
                    }
                }
                td { "{link_data.user.username}" }
                td {
                    style: "cursor: pointer;",
                    onclick: {
                        let code_clone = link_data.link.code.clone();
                        let link_id = link_data.link.id;
                        move |e: Event<MouseData>| {
                            e.stop_propagation();
                            let code = code_clone.clone();
                            async move {
                                if link_stats.read().get(&code).is_some() {
                                    link_stats.write().swap_remove(&code);
                                } else if let Ok(extended) =
                                    crate::api::link_api::get_link_statistics(link_id).await
                                {
                                    link_stats.write().insert(code, extended);
                                }
                            }
                        }
                    },
                    Stats { clicks: stats() }
                }
                td { class: "table_qr", dangerous_inner_html: qr_code_svg }
                td {
                    onclick: {
                        let link_data_clone = link_data.clone();
                        move |e: Event<MouseData>| {
                            e.stop_propagation();
                            if let Some(user_data) = user()
                                && user_data.role != Role::Admin
                                && user_data.id != link_data_clone.link.author
                            {
                                nachricht.set(Some(t!("links-error-not-author")));
                                timer.restart();
                                return;
                            }
                            info!("Delete link");
                            link_signal.set(Some(EditDialog::new(
                                Some(link_data_clone.link.id),
                                link_data_clone.link.code.clone(),
                                link_data_clone.link.title.clone(),
                                link_data_clone.link.target.clone(),
                                Some(link_data_clone.link.author),
                                EditMode::Delete(false),
                            )));
                        }
                    },
                    img { src: TRASH_SVG, class: "trashicon" }
                }
            }
        }
    }
}
