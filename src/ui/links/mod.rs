mod link_check;
mod link_display;
mod link_edit;
mod new_link_button;
mod stats;

use base64::prelude::*;
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

use crate::{
    domain::{
        models::{Clicks, FullLink},
        requests::{
            EditMode, Filter, LinkDelta, LinkOverviewColumns, LinkRequestForm, Operation, Ordering,
        },
    },
    ui::{
        PslinkContext,
        links::{link_display::LinkDisplay, link_edit::LinkEdit, new_link_button::NewLinkButton},
        navbar::Route,
        toggle_column,
    },
};

const LINKS_CSS: Asset = asset!("/assets/styling/lists.css");
const VANISHING_MESSAGE: Asset = asset!("/assets/styling/vanishing_message.css");

const SEARCH_SVG: Asset = asset!("/assets/search.svg");
const RELOAD_SVG: Asset = asset!("/assets/reload.svg");

#[derive(Clone, Debug)]
pub struct EditDialog {
    pub link_delta: LinkDelta,
    pub error: Option<String>,
    /// The target as it was when the dialog was opened (to detect changes).
    pub original_target: String,
}

impl EditDialog {
    #[must_use]
    pub fn new(
        id: Option<i64>,
        code: String,
        title: String,
        target: String,
        author: Option<i64>,
        edit: EditMode,
    ) -> Self {
        Self {
            original_target: target.clone(),
            link_delta: LinkDelta {
                edit,
                id,
                title,
                target,
                code,
                author,
            },
            error: None,
        }
    }
}

#[component]
pub fn Links() -> Element {
    let PslinkContext { user, .. } = use_context::<PslinkContext>();
    let mut code_filter = use_signal(String::new);
    let mut description_filter = use_signal(String::new);
    let mut target_filter = use_signal(String::new);
    let mut username_filter = use_signal(String::new);
    let mut order_by = use_signal(|| Option::<Operation<LinkOverviewColumns, Ordering>>::None);
    let reload_trigger = use_signal(|| 0u32);
    let edit_link = use_signal(|| Option::<EditDialog>::None);
    let mut links: Signal<IndexMap<String, FullLink>> = use_signal(IndexMap::new);
    let link_codes = use_memo(move || links().keys().cloned().collect::<Vec<String>>());
    let link_stats: Signal<IndexMap<String, Clicks>> = use_signal(IndexMap::new);

    let _update_filters = use_resource(move || async move {
        let _trigger = reload_trigger();
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
        div { class: "links-view",
            document::Stylesheet { href: LINKS_CSS }
            document::Stylesheet { href: VANISHING_MESSAGE }
            LinkEdit { edit_link, reload_trigger, links }
            if user().is_some() {
                div {
                    table { class: "table is-bordered is-striped is-hoverable is-fullwidth",
                        tbody {
                            tr {
                                th {
                                    class: "headlines",
                                    onclick: move |_| {
                                        order_by.set(toggle_column(order_by(), LinkOverviewColumns::Code));
                                    },
                                    {t!("links-table-header-code")}
                                }
                                th {
                                    class: "headlines",
                                    onclick: move |_| {
                                        order_by.set(toggle_column(order_by(), LinkOverviewColumns::Description));
                                    },
                                    {t!("links-table-header-description")}
                                }
                                th {
                                    class: "headlines",
                                    onclick: move |_| {
                                        order_by.set(toggle_column(order_by(), LinkOverviewColumns::Target));
                                    },
                                    {t!("links-table-header-target")}
                                }
                                th {
                                    class: "headlines",
                                    onclick: move |_| {
                                        order_by.set(toggle_column(order_by(), LinkOverviewColumns::Author));
                                    },
                                    {t!("links-table-header-username")}
                                }
                                th {
                                    class: "headlines",
                                    onclick: move |_| {
                                        order_by.set(toggle_column(order_by(), LinkOverviewColumns::Statistics));
                                    },
                                    {t!("links-table-header-statistics")}
                                }
                                th {}
                                th {}
                            }
                            tr {
                                td {
                                    div { class: "control has-icons-left has-icons-right is-small",
                                        input {
                                            r#type: "search",
                                            placeholder: t!("links-table-filter-placeholder"),
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
                                            placeholder: t!("links-table-filter-placeholder"),
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
                                            placeholder: t!("links-table-filter-placeholder"),
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
                                            placeholder: t!("links-table-filter-placeholder"),
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
                                        key: "{code}{link_stats().get(&code).is_some()}",
                                        current_code: code.clone(),
                                        links,
                                        link_stats,
                                        link_signal: edit_link,
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
                            if !new_links.is_empty() {
                                links.write().extend(new_links);
                            }
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
                            if !new_links.is_empty() {
                                links.write().extend(new_links);
                            }
                        },
                        img { src: RELOAD_SVG, class: "reloadicon" }
                        {t!("links-button-load-more")}
                    }
                }
            } else {
                div { class: "centered",
                    div { class: "boxed",
                        div { {t!("links-loading")} }
                        Link { to: Route::LoginScreen {}, {t!("links-login")} }
                    }
                }
            }
        }
    }
}

async fn load_links(
    offset: usize,
    amount: usize,
    code_filter: String,
    description_filter: String,
    target_filter: String,
    username_filter: String,
    order: Option<Operation<LinkOverviewColumns, Ordering>>,
) -> IndexMap<String, FullLink> {
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
    form.offset = offset;
    form.amount = amount;

    match crate::api::link_api::list_all_allowed(form).await {
        Ok(list) => list
            .into_iter()
            .map(|item| (item.link.code.clone(), item))
            .collect(),
        Err(e) => {
            info!("Failed to load links: {}", e);
            IndexMap::new()
        }
    }
}

/// Format a full short URL for `code` using the configured `base_url` (or `fallback_host`).
#[must_use]
pub fn generate_url_for_code(code: &str, base_url: &str, fallback_host: &str) -> String {
    let base = if !base_url.trim().is_empty() {
        base_url.trim().trim_end_matches('/').to_string()
    } else if !fallback_host.trim().is_empty() {
        format!("https://{}", fallback_host.trim().trim_end_matches('/'))
    } else {
        "http://localhost:8080".to_string()
    };
    format!("{}/{}", base, code)
}

/// Pre-rendered QR code preview and download payloads for the link edit modal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QrDownloads {
    pub svg_preview: String,
    pub svg_data_url: String,
    pub svg_filename: String,
    pub png_200_data_url: String,
    pub png_200_filename: String,
    pub png_1000_data_url: String,
    pub png_1000_filename: String,
}

/// Generate an SVG QR code string for a URL.
#[must_use]
pub fn generate_svg_qr_from_url(url: &str) -> String {
    let qrcode = QRBuilder::new(url).ecl(fast_qr::ECL::L).build();
    if let Ok(qrcode) = qrcode {
        SvgBuilder::default().shape(Shape::Square).to_str(&qrcode)
    } else {
        info!("Failed to create QR code");
        String::new()
    }
}

/// Generate a PNG data URL (`data:image/png;base64,...`) of the given pixel dimensions for `qrcode`.
fn generate_png_data_url(qrcode: &fast_qr::QRCode, dimension: u32) -> Option<String> {
    let png_bytes = ImageBuilder::default()
        .shape(Shape::Square)
        .fit_height(dimension)
        .fit_width(dimension)
        .to_bytes(qrcode)
        .ok()?;
    let encoded = BASE64_STANDARD.encode(&png_bytes);
    Some(format!("data:image/png;base64,{}", encoded))
}

/// Generate the SVG preview and all downloadable QR code variants (PNG 200x200, PNG 1000x1000, SVG).
#[must_use]
pub fn generate_qr_downloads(url: &str, code: &str) -> Option<QrDownloads> {
    let qrcode = QRBuilder::new(url).ecl(fast_qr::ECL::L).build().ok()?;
    let svg_preview = SvgBuilder::default().shape(Shape::Square).to_str(&qrcode);
    let svg_data_url = format!(
        "data:image/svg+xml;base64,{}",
        BASE64_STANDARD.encode(svg_preview.as_bytes())
    );
    let png_200_data_url = generate_png_data_url(&qrcode, 200)?;
    let png_1000_data_url = generate_png_data_url(&qrcode, 1000)?;

    let base_name = if code.trim().is_empty() {
        "root"
    } else {
        code.trim()
    };

    Some(QrDownloads {
        svg_preview,
        svg_data_url,
        svg_filename: format!("{base_name}.svg"),
        png_200_data_url,
        png_200_filename: format!("{base_name}_200x200.png"),
        png_1000_data_url,
        png_1000_filename: format!("{base_name}_1000x1000.png"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_qr_downloads_dimensions_and_filenames() {
        let dl = generate_qr_downloads("https://example.com/mycode", "mycode")
            .expect("QR generation should succeed");

        assert!(dl.svg_preview.contains("<svg"));
        assert!(dl.svg_data_url.starts_with("data:image/svg+xml;base64,"));
        assert_eq!(dl.svg_filename, "mycode.svg");
        assert_eq!(dl.png_200_filename, "mycode_200x200.png");
        assert_eq!(dl.png_1000_filename, "mycode_1000x1000.png");

        for (data_url, expected_dim) in [(&dl.png_200_data_url, 200u32), (&dl.png_1000_data_url, 1000u32)] {
            let b64 = data_url
                .strip_prefix("data:image/png;base64,")
                .expect("PNG data URL prefix");
            let bytes = BASE64_STANDARD.decode(b64).expect("valid base64");
            assert_eq!(&bytes[0..8], b"\x89PNG\r\n\x1a\n");
            let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
            let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
            assert_eq!(width, expected_dim);
            assert_eq!(height, expected_dim);
        }
    }

    #[test]
    fn test_generate_qr_downloads_empty_code_fallback() {
        let dl = generate_qr_downloads("https://example.com/", "")
            .expect("QR generation should succeed");
        assert_eq!(dl.svg_filename, "root.svg");
        assert_eq!(dl.png_200_filename, "root_200x200.png");
        assert_eq!(dl.png_1000_filename, "root_1000x1000.png");
    }
}
