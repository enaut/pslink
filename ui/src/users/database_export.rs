//! Database export component for admin users

use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn DatabaseExportButton() -> Element {
    let mut secret_input = use_signal(|| String::new());
    let mut is_loading = use_signal(|| false);
    let mut error_message = use_signal(|| Option::<String>::None);
    let mut success_message = use_signal(|| Option::<String>::None);
    let mut download_data = use_signal(|| Option::<Vec<u8>>::None);

    let create_download_url = move |data: &[u8]| -> String {
        use base64::prelude::*;
        let encoded = BASE64_STANDARD.encode(data);
        format!("data:application/octet-stream;base64,{}", encoded)
    };

    let export_database = move |_| async move {
        is_loading.set(true);
        error_message.set(None);
        success_message.set(None);
        download_data.set(None);
        
        let secret = secret_input();
        if secret.trim().is_empty() {
            error_message.set(Some(t!("database-export-error-empty-secret")));
            is_loading.set(false);
            return;
        }
        
        match backend::export_api::export_database(secret).await {
            Ok(data) => {
                download_data.set(Some(data));
                success_message.set(Some(t!("database-export-success-message")));
                secret_input.set(String::new()); // Clear the secret input
            }
            Err(e) => {
                error_message.set(Some(format!("{}: {}", t!("database-export-error-failed"), e)));
            }
        }
        
        is_loading.set(false);
    };

    rsx! {
        div { class: "database-export-section",
            h3 { class: "title is-5", {t!("database-export-title")} }
            p { class: "subtitle is-6", {t!("database-export-subtitle")} }
            
            if let Some(error) = error_message() {
                div { class: "notification is-danger", "{error}" }
            }
            
            if let Some(success) = success_message() {
                div { class: "notification is-success", "{success}" }
            }
            
            if let Some(data) = download_data() {
                div { class: "notification is-success",
                    p { {t!("database-export-ready")} }
                    p { class: "is-size-7", {t!("database-export-size", size: data.len())} }
                    div { class: "buttons",
                        a { 
                            class: "button is-primary",
                            href: create_download_url(&data),
                            download: "pslink_database.sqlite",
                            {t!("database-export-button-download")}
                        }
                        button { 
                            class: "button is-light is-small",
                            onclick: move |_| {
                                download_data.set(None);
                                success_message.set(None);
                            },
                            {t!("database-export-button-clear")}
                        }
                    }
                }
            }
            
            div { class: "field",
                label { class: "label", {t!("database-export-label-secret")} }
                div { class: "control",
                    input { 
                        class: "input",
                        r#type: "password",
                        placeholder: "{t!(\"database-export-placeholder-secret\")}",
                        value: "{secret_input}",
                        oninput: move |e| secret_input.set(e.value()),
                        disabled: is_loading()
                    }
                }
                p { class: "help", {t!("database-export-help-secret")} }
            }
            
            div { class: "field",
                div { class: "control",
                    if is_loading() {
                        button { class: "button is-primary is-loading", disabled: true, {t!("database-export-button-exporting")} }
                    } else {
                        button { 
                            class: "button is-primary",
                            onclick: export_database,
                            disabled: secret_input().trim().is_empty(),
                            {t!("database-export-button-export")}
                        }
                    }
                }
            }
        }
    }
}