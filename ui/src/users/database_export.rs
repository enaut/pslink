//! Database export component for admin users

use dioxus::prelude::*;

#[component]
pub fn DatabaseExportButton() -> Element {
    let mut export_url = use_signal(|| Option::<String>::None);
    let mut is_loading = use_signal(|| false);
    let mut error_message = use_signal(|| Option::<String>::None);

    let get_export_url = move |_| async move {
        is_loading.set(true);
        error_message.set(None);
        
        match backend::export_api::get_export_url().await {
            Ok(url) => {
                export_url.set(Some(url));
            }
            Err(e) => {
                error_message.set(Some(format!("Error: {}", e)));
            }
        }
        
        is_loading.set(false);
    };

    rsx! {
        div { class: "database-export-section",
            h3 { class: "title is-5", "Database Export" }
            p { class: "subtitle is-6", "Export the SQLite database for backup purposes" }
            
            if let Some(error) = error_message() {
                div { class: "notification is-danger", "{error}" }
            }
            
            if is_loading() {
                button { class: "button is-primary is-loading", disabled: true, "Generating Export URL..." }
            } else if let Some(url) = export_url() {
                div { class: "field is-grouped",
                    div { class: "control",
                        a { 
                            href: "{url}",
                            class: "button is-success",
                            download: "pslink_backup.db",
                            "Download Database"
                        }
                    }
                    div { class: "control",
                        button { 
                            class: "button is-light",
                            onclick: move |_| {
                                export_url.set(None);
                            },
                            "Generate New URL"
                        }
                    }
                }
                div { class: "content is-small",
                    p { class: "has-text-grey", 
                        "⚠️ This URL contains a secret token. Do not share it publicly."
                    }
                }
            } else {
                button { 
                    class: "button is-primary",
                    onclick: get_export_url,
                    "Generate Export URL"
                }
            }
        }
    }
}