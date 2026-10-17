use std::rc::Rc;

use dioxus::{logger::tracing::info, prelude::*};
use dioxus_i18n::t;

use crate::{
    domain::requests::EditMode,
    ui::{
        PslinkContext,
        links::{
            EditDialog, generate_png_download_from_url, generate_svg_qr_from_url,
            generate_url_for_code,
        },
    },
};

fn generate_random_client_code() -> String {
    const CHARSET: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";
    (0..6)
        .map(|_| {
            let idx = (js_sys::Math::random() * (CHARSET.len() as f64)) as usize;
            CHARSET[idx.min(CHARSET.len() - 1)] as char
        })
        .collect()
}

#[component]
pub fn LinkEdit(
    mut edit_link: Signal<Option<EditDialog>>,
    reload_trigger: Signal<u32>,
) -> Element {
    let PslinkContext {
        hostname, base_url, ..
    } = use_context::<PslinkContext>();

    let on_esc_event = move |evt: KeyboardEvent| {
        if evt.key() == Key::Named(NamedKey::Escape) {
            edit_link.set(None);
        }
    };

    let mut description_field: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    let _focus_grabber = use_resource(move || async move {
        if let Some(field) = description_field() {
            let _ = field.set_focus(true).await;
        }
    });

    let qr_preview = use_memo(move || {
        edit_link().map(|dialog| {
            let url =
                generate_url_for_code(&dialog.link_delta.code, &base_url(), &hostname());
            let svg = generate_svg_qr_from_url(&url);
            let png = generate_png_download_from_url(&url);
            (svg, png)
        })
    });

    let Some(dialog) = edit_link() else {
        return rsx! {};
    };

    let (qr_svg, png_qr_url) = qr_preview().unwrap_or_else(|| (String::new(), None));

    rsx! {
        div { class: "modal is-active", onkeydown: on_esc_event,
            div { class: "modal-background" }
            div { class: "modal-card",
                header { class: "modal-card-head",
                    p { class: "modal-card-title", {t!("link-edit-modal-title")} }
                    button {
                        "aria-label": "close",
                        class: "delete",
                        onclick: move |_| {
                            edit_link.set(None);
                        },
                    }
                }
                div { class: "modal-card-body",
                    if let Some(err) = &dialog.error {
                        div { class: "notification is-danger", "{err}" }
                    }
                    div { class: "field is-horizontal is-wider",
                        div { class: "field-label is-normal",
                            label { class: "label", {t!("link-edit-field-description")} }
                        }
                        div { class: "field-body",
                            div { class: "field",
                                p { class: "control",
                                    input {
                                        autofocus: true,
                                        onmounted: move |e| {
                                            description_field.set(Some(e.data()));
                                        },
                                        placeholder: t!("link-edit-placeholder-description"),
                                        value: "{dialog.link_delta.title}",
                                        r#type: "text",
                                        class: "input",
                                        oninput: move |e| {
                                            if let Some(d) = edit_link.write().as_mut() {
                                                d.link_delta.title = e.value();
                                                d.error = None;
                                            }
                                        },
                                    }
                                }
                            }
                        }
                    }
                    div { class: "field is-horizontal is-wider",
                        div { class: "field-label is-normal",
                            label { class: "label", {t!("link-edit-field-target")} }
                        }
                        div { class: "field-body",
                            div { class: "field",
                                p { class: "control",
                                    input {
                                        placeholder: t!("link-edit-placeholder-target"),
                                        value: "{dialog.link_delta.target}",
                                        r#type: "text",
                                        class: "input",
                                        oninput: move |e| {
                                            if let Some(d) = edit_link.write().as_mut() {
                                                d.link_delta.target = e.value();
                                                d.error = None;
                                            }
                                        },
                                    }
                                }
                            }
                        }
                    }
                    div { class: "field is-horizontal is-wider",
                        div { class: "field-label is-normal",
                            label { class: "label", {t!("link-edit-field-code")} }
                        }
                        div { class: "field-body",
                            div { class: "field has-addons",
                                p { class: "control is-expanded",
                                    input {
                                        placeholder: t!("link-edit-placeholder-code"),
                                        value: "{dialog.link_delta.code}",
                                        r#type: "text",
                                        class: "input",
                                        oninput: move |e| {
                                            if let Some(d) = edit_link.write().as_mut() {
                                                d.link_delta.code = e.value();
                                                d.error = None;
                                            }
                                        },
                                    }
                                }
                                p { class: "control",
                                    button {
                                        class: "button is-light",
                                        r#type: "button",
                                        title: "Generate random short code",
                                        onclick: move |_| {
                                            if let Some(d) = edit_link.write().as_mut() {
                                                d.link_delta.code = generate_random_client_code();
                                                d.error = None;
                                            }
                                        },
                                        "🎲"
                                    }
                                }
                            }
                        }
                    }
                    div { class: "field is-horizontal is-wider",
                        div { class: "field-label is-normal",
                            label { class: "label", {t!("link-edit-field-qrcode")} }
                        }
                        if let Some((png_url, filename)) = png_qr_url {
                            div { class: "field-body",
                                div { width: "133px",
                                    a {
                                        href: png_url,
                                        dangerous_inner_html: "{qr_svg}",
                                        download: filename,
                                    }
                                }
                            }
                        }
                    }
                    ConfirmDialog { edit_link }
                }
                EditFooter { edit_link, reload_trigger }
            }
        }
    }
}

#[component]
pub fn ConfirmDialog(edit_link: Signal<Option<EditDialog>>) -> Element {
    if let Some(dialog) = edit_link()
        && dialog.link_delta.edit == EditMode::Delete(true)
    {
        return rsx! {
            div { class: "notification is-danger", {t!("link-edit-delete-warning")} }
        };
    }
    rsx! {}
}

#[component]
fn EditFooter(
    edit_link: Signal<Option<EditDialog>>,
    reload_trigger: Signal<u32>,
) -> Element {
    rsx! {
        footer { class: "modal-card-foot is-justify-content-flex-end",
            div { class: "buttons",
                Buttons { edit_link, reload_trigger }
            }
        }
    }
}

#[component]
fn Buttons(
    mut edit_link: Signal<Option<EditDialog>>,
    mut reload_trigger: Signal<u32>,
) -> Element {
    let Some(EditDialog { link_delta, .. }) = edit_link() else {
        return rsx! {};
    };

    info!("Edit mode: {:?}", link_delta.edit);
    match link_delta.edit {
        EditMode::Create => rsx! {
            button {
                class: "button is-success",
                onclick: move |_e: Event<MouseData>| async move {
                    if let Some(dialog) = edit_link() {
                        match crate::api::link_api::create_link(dialog.link_delta).await {
                            Ok(_) => {
                                reload_trigger.set(reload_trigger().wrapping_add(1));
                                edit_link.set(None);
                            }
                            Err(e) => {
                                if let Some(d) = edit_link.write().as_mut() {
                                    d.error = Some(e.to_string());
                                }
                            }
                        }
                    }
                },
                {t!("link-edit-button-create")}
            }
        },
        EditMode::Edit => rsx! {
            button {
                class: "button is-danger",
                onclick: move |_e: Event<MouseData>| {
                    if let Some(d) = edit_link.write().as_mut() {
                        d.link_delta.edit = EditMode::Delete(true);
                    }
                },
                {t!("link-edit-button-delete")}
            }
            button {
                class: "button is-success",
                onclick: move |_e: Event<MouseData>| async move {
                    if let Some(dialog) = edit_link() {
                        match crate::api::link_api::save_link(dialog.link_delta).await {
                            Ok(_) => {
                                reload_trigger.set(reload_trigger().wrapping_add(1));
                                edit_link.set(None);
                            }
                            Err(e) => {
                                if let Some(d) = edit_link.write().as_mut() {
                                    d.error = Some(e.to_string());
                                }
                            }
                        }
                    }
                },
                {t!("link-edit-button-modify")}
            }
        },
        EditMode::Delete(ask) => {
            if ask {
                rsx! {
                    button {
                        class: "button is-danger",
                        onclick: move |_e: Event<MouseData>| async move {
                            if let Some(dialog) = edit_link()
                                && let Some(link_id) = dialog.link_delta.id
                            {
                                match crate::api::link_api::delete_link(link_id).await {
                                    Ok(_) => {
                                        reload_trigger.set(reload_trigger().wrapping_add(1));
                                        edit_link.set(None);
                                    }
                                    Err(e) => {
                                        if let Some(d) = edit_link.write().as_mut() {
                                            d.error = Some(e.to_string());
                                        }
                                    }
                                }
                            }
                        },
                        {t!("link-edit-button-confirm-delete")}
                    }
                }
            } else {
                rsx! {
                    button {
                        class: "button is-danger",
                        onclick: move |_e: Event<MouseData>| {
                            if let Some(d) = edit_link.write().as_mut() {
                                d.link_delta.edit = EditMode::Delete(true);
                            }
                        },
                        {t!("link-edit-button-delete")}
                    }
                }
            }
        }
    }
}
