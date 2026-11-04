//! Links export component for admin users.

use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn LinksExport() -> Element {
    let mut delimiter = use_signal(|| ",".to_string());
    let mut is_loading = use_signal(|| false);
    let mut error_message = use_signal(|| Option::<String>::None);
    let mut success_message = use_signal(|| Option::<String>::None);
    let mut download_data = use_signal(|| Option::<String>::None);

    let create_download_url = move |data: &str| -> String {
        use base64::prelude::*;
        let encoded = BASE64_STANDARD.encode(data.as_bytes());
        format!("data:text/csv;charset=utf-8;base64,{}", encoded)
    };

    let export_links = move |_| async move {
        is_loading.set(true);
        error_message.set(None);
        success_message.set(None);
        download_data.set(None);

        match crate::api::export_api::export_links_csv(delimiter()).await {
            Ok(csv_data) => {
                download_data.set(Some(csv_data.clone()));
                success_message.set(Some(t!("links-export-success-message")));

                // Auto-trigger browser download
                let filename = "pslink_links.csv";
                trigger_download(filename, &csv_data);
            }
            Err(e) => {
                error_message.set(Some(format!("{}: {}", t!("links-export-error-failed"), e)));
            }
        }

        is_loading.set(false);
    };

    rsx! {
        div { class: "links-export-section container p-4",
            h3 { class: "title is-5", {t!("links-export-title")} }
            p { class: "subtitle is-6", {t!("links-export-subtitle")} }

            if let Some(error) = error_message() {
                div { class: "notification is-danger", "{error}" }
            }

            if let Some(success) = success_message() {
                div { class: "notification is-success", "{success}" }
            }

            if let Some(data) = download_data() {
                div { class: "notification is-info is-light",
                    p { {t!("links-export-ready")} }
                    div { class: "buttons mt-2",
                        a {
                            class: "button is-primary is-small",
                            href: create_download_url(&data),
                            download: "pslink_links.csv",
                            {t!("links-export-button-download")}
                        }
                        button {
                            class: "button is-light is-small",
                            onclick: move |_| {
                                download_data.set(None);
                                success_message.set(None);
                            },
                            {t!("links-export-button-clear")}
                        }
                    }
                }
            }

            div { class: "field",
                label { class: "label", {t!("links-export-label-delimiter")} }
                div { class: "control",
                    div { class: "select",
                        select {
                            value: "{delimiter}",
                            onchange: move |e| delimiter.set(e.value()),
                            disabled: is_loading(),
                            option { value: ",", {t!("links-export-delimiter-comma")} }
                            option { value: ";", {t!("links-export-delimiter-semicolon")} }
                            option { value: "|", {t!("links-export-delimiter-pipe")} }
                        }
                    }
                }
            }

            div { class: "field",
                div { class: "control",
                    if is_loading() {
                        button {
                            class: "button is-primary is-loading",
                            disabled: true,
                            {t!("links-export-button-exporting")}
                        }
                    } else {
                        button {
                            class: "button is-primary",
                            onclick: export_links,
                            {t!("links-export-button-export")}
                        }
                    }
                }
            }
        }
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn trigger_download(filename: &str, content: &str) {
    use wasm_bindgen::JsCast;
    use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            let array = js_sys::Array::new();
            array.push(&wasm_bindgen::JsValue::from_str(content));

            let bag = BlobPropertyBag::new();
            bag.set_type("text/csv;charset=utf-8;");

            if let Ok(blob) = Blob::new_with_str_sequence_and_options(&array, &bag) {
                if let Ok(url) = Url::create_object_url_with_blob(&blob) {
                    if let Ok(elem) = document.create_element("a") {
                        if let Ok(a) = elem.dyn_into::<HtmlAnchorElement>() {
                            a.set_href(&url);
                            a.set_download(filename);
                            a.style().set_property("display", "none").ok();

                            if let Some(body) = document.body() {
                                let _ = body.append_child(&a);
                                a.click();
                                let _ = body.remove_child(&a);
                            } else {
                                a.click();
                            }
                            let _ = Url::revoke_object_url(&url);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(not(all(feature = "web", target_arch = "wasm32")))]
fn trigger_download(_filename: &str, _content: &str) {}
