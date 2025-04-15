mod link_display;
mod link_edit;
mod new_link_button;
mod stats;

use dioxus::{
    logger::tracing::{info, trace},
    prelude::*,
};
use dioxus_i18n::t;
use fast_qr::{
    QRBuilder,
    convert::{Builder as _, Shape, image::ImageBuilder, svg::SvgBuilder},
};
use indexmap::IndexMap;
use pslink_shared::{
    apirequests::{
        general::{EditMode, Filter, Operation, Ordering},
        links::{LinkDelta, LinkOverviewColumns, LinkRequestForm},
    },
    datatypes::FullLink,
};
use web_sys::wasm_bindgen::JsValue;

use crate::links::link_display::LinkDisplay;
use crate::links::link_edit::LinkEdit;
use crate::links::new_link_button::NewLinkButton;
use crate::{PslinkContext, navbar::Route};

const LINKS_CSS: Asset = asset!("/assets/styling/lists.css");

const SEARCH_SVG: Asset = asset!("/assets/search.svg");
const RELOAD_SVG: Asset = asset!("/assets/reload.svg");

fn toggle_column(
    ordering: Option<Operation<LinkOverviewColumns, Ordering>>,
    new_column: LinkOverviewColumns,
) -> Option<Operation<LinkOverviewColumns, Ordering>> {
    if let Some(Operation { column, value }) = ordering {
        if column == new_column {
            let new_value = match value {
                Ordering::Ascending => Ordering::Descending,
                Ordering::Descending => Ordering::Ascending,
            };
            return Some(Operation {
                column,
                value: new_value,
            });
        }
    };

    Some(Operation {
        column: new_column,
        value: Ordering::Ascending,
    })
}

#[derive(Clone)]
struct EditDialog {
    link_delta: LinkDelta,
    qr: String,
    png_qr_url: Option<String>,
}

trait OptionEditDialog {
    fn set_edit_dialog(
        &mut self,
        id: Option<i64>,
        code: String,
        title: String,
        target: String,
        author: Option<i64>,
        edit_mode: EditMode,
        host: &str,
    );
    fn update_code(&mut self, code: String, host: &str);
    fn update_title(&mut self, title: String);
    fn update_target(&mut self, target: String);
    fn set_edit_mode(&mut self, edit_mode: EditMode);
}

impl OptionEditDialog for Signal<Option<EditDialog>> {
    fn set_edit_dialog(
        &mut self,
        id: Option<i64>,
        code: String,
        title: String,
        target: String,
        author: Option<i64>,
        edit_mode: EditMode,
        host: &str,
    ) {
        let url = generate_url_for_code(&code, host);
        let qr_string = generate_svg_qr_from_url(&url);
        if let Some(mut dialog) = self() {
            dialog.link_delta = LinkDelta {
                id,
                author: None,
                edit: edit_mode,
                title,
                target,
                code,
            };
            dialog.qr = qr_string;
            self.set(Some(dialog));
        } else {
            self.set(Some(EditDialog {
                link_delta: LinkDelta {
                    id,
                    author,
                    edit: edit_mode,
                    title,
                    target,
                    code,
                },
                qr: qr_string,
                png_qr_url: Some(generate_blob_url_from_png(generate_png_qr_from_url(&url))),
            }))
        }
    }
    fn update_code(&mut self, code: String, host: &str) {
        trace!("Updating code to: {}", code);
        if let Some(mut dialog) = self() {
            let url = generate_url_for_code(&code, host);
            dialog.qr = generate_svg_qr_from_url(&url);
            dialog.link_delta.code = code.to_string();
            self.set(Some(dialog));
        }
    }
    fn update_title(&mut self, title: String) {
        trace!("Updating title to: {}", title);
        if let Some(mut dialog) = self() {
            dialog.link_delta.title = title;
            info!("Updated dialog: {:?}", dialog.link_delta.title);
            self.set(Some(dialog));
        };
    }
    fn update_target(&mut self, target: String) {
        trace!("Updating target to: {}", target);
        if let Some(mut dialog) = self() {
            dialog.link_delta.target = target;
            self.set(Some(dialog));
        };
    }

    fn set_edit_mode(&mut self, edit_mode: EditMode) {
        trace!("Updating edit to: {:?}", &edit_mode);
        if let Some(mut dialog) = self() {
            dialog.link_delta.edit = edit_mode;
            self.set(Some(dialog));
        };
    }
}

#[component]
pub fn Links() -> Element {
    let PslinkContext { user, .. } = use_context::<PslinkContext>();
    let mut code_filter = use_signal(|| "".to_string());
    let mut description_filter = use_signal(|| "".to_string());
    let mut target_filter = use_signal(|| "".to_string());
    let mut username_filter = use_signal(|| "".to_string());
    let mut order_by = use_signal(|| Option::<Operation<LinkOverviewColumns, Ordering>>::None);
    let edit_link = use_signal(|| None);
    let mut links: Signal<IndexMap<String, FullLink>> = use_signal(move || IndexMap::new());
    let link_codes = use_memo(move || links().keys().cloned().collect::<Vec<String>>());
    let mut link_stats = use_signal(|| IndexMap::new());
    let links_without_stats = use_resource(move || async move {
        let codes_to_update: Vec<_> = links()
            .iter()
            .filter_map(|(code, link)| {
                if link_stats.peek().get(code).is_none() {
                    Some((code.clone(), link.link.id))
                } else {
                    None
                }
            })
            .collect();

        for (code, link_id) in codes_to_update {
            if let Ok(statistics) = backend::link_api::get_link_statistics(link_id).await {
                link_stats.write().insert(code, statistics);
            }
        }
    });

    let _update_filters = use_resource(move || async move {
        let code_filter = code_filter();
        let description_filter = description_filter();
        let target_filter = target_filter();
        let username_filter = username_filter();
        let order_by = order_by();

        trace!(
            "Filters: {} {} {} {}",
            code_filter, description_filter, target_filter, username_filter
        );

        let loaded_links = load_links(
            0,
            50,
            code_filter,
            description_filter,
            target_filter,
            username_filter,
            order_by,
        )
        .await;
        links.set(loaded_links);
    });

    rsx! {
        document::Stylesheet { href: LINKS_CSS }
        LinkEdit { edit_link, links }
        if let Some(_user) = user.as_ref() {
            div {
                table { class: "table is-bordered is-striped is-hoverable is-fullwidth",
                    tbody {
                        tr {
                            th {
                                class: "headlines",
                                onclick: move |_| {
                                    order_by.set(toggle_column(order_by(), LinkOverviewColumns::Code));
                                },
                                {t!("links-table-header-code")} // Column header for link code
                            }
                            th {
                                class: "headlines",
                                onclick: move |_| {
                                    order_by.set(toggle_column(order_by(), LinkOverviewColumns::Description));
                                },
                                {t!("links-table-header-description")} // Column header for description
                            }
                            th {
                                class: "headlines",
                                onclick: move |_| {
                                    order_by.set(toggle_column(order_by(), LinkOverviewColumns::Target));
                                },
                                {t!("links-table-header-target")} // Column header for link target
                            }
                            th {
                                class: "headlines",
                                onclick: move |_| {
                                    order_by.set(toggle_column(order_by(), LinkOverviewColumns::Author));
                                },
                                {t!("links-table-header-username")} // Column header for username
                            }
                            th {
                                class: "headlines",
                                onclick: move |_| {
                                    order_by.set(toggle_column(order_by(), LinkOverviewColumns::Statistics));
                                },
                                {t!("links-table-header-statistics")}
                                {format!("{}", links_without_stats().iter().len())}
                            }
                            th {}
                            th {}
                        }
                        tr {
                            td {
                                div { class: "control has-icons-left has-icons-right is-small",
                                    input {
                                        r#type: "search",
                                        placeholder: t!("links-table-filter-placeholder"), // Placeholder text for filter input field
                                        value: "{code_filter}",
                                        class: "input is-small",
                                        oninput: move |e| {
                                            code_filter.set(e.value());
                                        },
                                    }
                                    span { class: "icon is-small is-left",
                                        img { src: SEARCH_SVG }
                                    }
                                }
                            }
                            td {
                                div { class: "control has-icons-left has-icons-right is-small",
                                    input {
                                        value: "{description_filter}",
                                        r#type: "search",
                                        placeholder: t!("links-table-filter-placeholder"), // Placeholder text for filter input field
                                        class: "input is-small",
                                        oninput: move |e| {
                                            description_filter.set(e.value());
                                        },
                                    }
                                    span { class: "icon is-small is-left",
                                        img { src: SEARCH_SVG }
                                    }
                                }
                            }
                            td {
                                div { class: "control has-icons-left has-icons-right is-small",
                                    input {
                                        r#type: "search",
                                        value: "{target_filter}",
                                        placeholder: t!("links-table-filter-placeholder"), // Placeholder text for filter input field
                                        class: "input is-small",
                                        oninput: move |e| {
                                            target_filter.set(e.value());
                                        },
                                    }
                                    span { class: "icon is-small is-left",
                                        img { src: SEARCH_SVG }
                                    }
                                }
                            }
                            td {
                                div { class: "control has-icons-left has-icons-right is-small",
                                    input {
                                        placeholder: t!("links-table-filter-placeholder"), // Placeholder text for filter input field
                                        value: "{username_filter}",
                                        class: "input is-small",
                                        r#type: "search",
                                        oninput: move |e| {
                                            username_filter.set(e.value());
                                        },
                                    }
                                    span { class: "icon is-small is-left",
                                        img { src: SEARCH_SVG }
                                    }
                                }
                            }
                            td {}
                            td {}
                            td {}
                        }
                        if !links().is_empty() {
                            for code in link_codes() {
                                LinkDisplay {
                                    key: format!("{}{}", &code, link_stats().get(&code).is_some()),
                                    current_code: code.clone(),
                                    links,
                                    link_stats,
                                    link_signal: edit_link.clone(),
                                }
                            }
                        }
                    }
                }
                div {
                    NewLinkButton { edit_link }
                }
                a {
                    class: "loadmore button",
                    onvisible: move |_| async move {
                        let new_links = load_links(
                                links().len(),
                                50,
                                code_filter(),
                                description_filter(),
                                target_filter(),
                                username_filter(),
                                order_by(),
                            )
                            .await;
                        let mut old_links = links();
                        old_links.extend(new_links);
                        links.set(old_links);
                    },
                    onclick: move |_| async move {
                        let new_links = load_links(
                                links().len(),
                                50,
                                code_filter(),
                                description_filter(),
                                target_filter(),
                                username_filter(),
                                order_by(),
                            )
                            .await;
                        let mut old_links = links();
                        old_links.extend(new_links);
                        links.set(old_links);
                    },
                    img { src: RELOAD_SVG, class: "reloadicon" }
                    {t!("links-button-load-more")} // Button text to load more links
                }
            }
        } else {
            div { class: "centered",
                div { class: "boxed",
                    div { {t!("links-loading")} } // Text displayed while loading links data
                    Link { to: Route::LoginScreen {}, {t!("links-login")} } // Text for login link
                }
            }
        }
    }
}

async fn load_links(
    offsett: usize,
    amount: usize,
    code_filter: String,
    description_filter: String,
    target_filter: String,
    username_filter: String,
    order: Option<Operation<LinkOverviewColumns, Ordering>>,
) -> IndexMap<String, FullLink> {
    {
        let mut form = LinkRequestForm::default();
        form.filter[LinkOverviewColumns::Code] = Filter { sieve: code_filter };
        form.filter[LinkOverviewColumns::Description] = Filter {
            sieve: description_filter,
        };
        form.filter[LinkOverviewColumns::Target] = Filter {
            sieve: target_filter,
        };
        form.filter[LinkOverviewColumns::Author] = Filter {
            sieve: username_filter,
        };
        form.order = order;
        form.offset = offsett;
        form.amount = amount;
        let res: IndexMap<String, FullLink> = backend::link_api::list_all_allowed(form)
            .await
            .expect("Links")
            .list
            .into_iter()
            .map(|item| (item.link.code.clone(), item))
            .collect();
        res
    }
}

/// generate a qr-code for a code
pub fn generate_url_for_code(code: &str, host: &str) -> String {
    format!("https://{}/{}", host, code)
}

/// generate a svg qr-code for a url
fn generate_svg_qr_from_url(url: &str) -> String {
    let qrcode = QRBuilder::new(url).ecl(fast_qr::ECL::L).build();
    if let Ok(qrcode) = qrcode {
        let svg = SvgBuilder::default().shape(Shape::Square).to_str(&qrcode);
        svg
    } else {
        info!("Failed to create QR code");
        "".to_string()
    }
}

// generate a png qr-code for a url
fn generate_png_qr_from_url(url: &str) -> Vec<u8> {
    let qrcode = QRBuilder::new(url).ecl(fast_qr::ECL::L).build();

    if let Ok(qrcode) = qrcode {
        let png = ImageBuilder::default()
            .shape(Shape::Square)
            .fit_height((qrcode.size * 8).try_into().unwrap())
            .fit_width((qrcode.size * 8).try_into().unwrap())
            .to_bytes(&qrcode)
            .expect("Failed to create png");
        png
    } else {
        info!("Failed to create QR code");
        vec![]
    }
}

fn generate_blob_url_from_png(png: Vec<u8>) -> String {
    let properties = web_sys::BlobPropertyBag::new();
    properties.set_type("image/png");

    let png_jsarray: JsValue = web_sys::js_sys::Uint8Array::from(&png[..]).into();
    // the buffer has to be an array of arrays
    let png_buffer: web_sys::js_sys::Array = IntoIterator::into_iter([png_jsarray]).collect();
    let png_blob =
        web_sys::Blob::new_with_buffer_source_sequence_and_options(&png_buffer, &properties)
            .unwrap();
    let url = web_sys::Url::create_object_url_with_blob(&png_blob).unwrap();
    url
}
