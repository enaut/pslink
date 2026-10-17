use std::{rc::Rc, time::Duration};

use dioxus::{logger::tracing::info, prelude::*};
use dioxus_i18n::t;
use indexmap::IndexMap;

use crate::{
    domain::{
        models::{FullLink, LinkCheck},
        requests::EditMode,
        validation::validate_target_url,
    },
    ui::{
        PslinkContext,
        links::{
            EditDialog, generate_png_download_from_url, generate_svg_qr_from_url,
            generate_url_for_code,
            link_check::{CheckDot, CheckLine},
        },
    },
};

/// How long to wait after the last keystroke before the typed target is checked.
const TARGET_CHECK_DEBOUNCE: Duration = Duration::from_millis(600);

/// The state of the live check of the target that is currently typed into the dialog.
#[derive(Clone, PartialEq)]
enum TargetCheck {
    /// Nothing was changed (or the input is not a valid url yet).
    Idle,
    Checking,
    Done(LinkCheck),
    Failed(String),
}

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
    mut links: Signal<IndexMap<String, FullLink>>,
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
            let url = generate_url_for_code(&dialog.link_delta.code, &base_url(), &hostname());
            let svg = generate_svg_qr_from_url(&url);
            let png = generate_png_download_from_url(&url);
            (svg, png)
        })
    });

    // The check history of the link that is edited (`None` while it is loading).
    let mut history: Signal<Option<Vec<LinkCheck>>> = use_signal(|| None);
    let mut target_check = use_signal(|| TargetCheck::Idle);
    let mut show_preview = use_signal(|| false);

    // The id of the stored link that is edited. It only changes when a dialog is opened or closed.
    let open_id = use_memo(move || {
        edit_link().and_then(|d| {
            if d.link_delta.edit == EditMode::Edit {
                d.link_delta.id
            } else {
                None
            }
        })
    });

    // Test the stored target again whenever the dialog of an existing link is opened.
    let _check_on_open = use_resource(move || async move {
        history.set(None);
        let Some(link_id) = open_id() else {
            return;
        };
        match crate::api::link_api::check_link_now(link_id).await {
            Ok(checks) => {
                if let Some(latest) = checks.first() {
                    // Keep the list in the background up to date.
                    for link in links.write().values_mut() {
                        if link.link.id == link_id {
                            link.last_check = Some(latest.clone());
                        }
                    }
                }
                history.set(Some(checks));
            }
            Err(e) => {
                info!("Failed to check link {}: {}", link_id, e);
                history.set(Some(Vec::new()));
            }
        }
    });

    // Test the typed target (debounced) whenever it differs from the stored one.
    let typed_target = use_memo(move || {
        edit_link().map(|d| {
            (
                d.link_delta.target.trim().to_string(),
                d.original_target.trim().to_string(),
            )
        })
    });
    let _check_typed_target = use_resource(move || async move {
        let Some((target, original)) = typed_target() else {
            target_check.set(TargetCheck::Idle);
            show_preview.set(false);
            return;
        };
        if target == original || validate_target_url(&target).is_err() {
            target_check.set(TargetCheck::Idle);
            return;
        }
        target_check.set(TargetCheck::Checking);
        // If the user keeps typing this future is dropped while sleeping.
        wasmtimer::tokio::sleep(TARGET_CHECK_DEBOUNCE).await;
        match crate::api::link_api::check_target(target).await {
            Ok(check) => target_check.set(TargetCheck::Done(check)),
            Err(e) => target_check.set(TargetCheck::Failed(e.to_string())),
        }
    });

    let Some(dialog) = edit_link() else {
        return rsx! {};
    };

    let (qr_svg, png_qr_url) = qr_preview().unwrap_or_else(|| (String::new(), None));

    let is_edit = dialog.link_delta.edit == EditMode::Edit;
    let target_unchanged = dialog.link_delta.target.trim() == dialog.original_target.trim();
    // What is shown in the status row: the result for the typed target or the latest stored check.
    let mut checking = false;
    let mut shown_check: Option<LinkCheck> = None;
    let mut check_error: Option<String> = None;
    match target_check() {
        TargetCheck::Checking => checking = true,
        TargetCheck::Done(check) => shown_check = Some(check),
        TargetCheck::Failed(e) => check_error = Some(e),
        TargetCheck::Idle => {
            if is_edit && target_unchanged {
                match history() {
                    None => checking = true,
                    Some(checks) => shown_check = checks.first().cloned(),
                }
            }
        }
    }
    let history_strip = if is_edit && target_unchanged {
        history().filter(|checks| !checks.is_empty())
    } else {
        None
    };
    let show_status_row =
        checking || shown_check.is_some() || check_error.is_some() || history_strip.is_some();

    let preview_url = {
        let target = dialog.link_delta.target.trim();
        (validate_target_url(target).is_ok()
            && !matches!(dialog.link_delta.edit, EditMode::Delete(_)))
        .then(|| target.to_string())
    };
    let embeddable = shown_check.as_ref().and_then(|check| check.embeddable);

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
                    if show_status_row {
                        div { class: "field is-horizontal is-wider",
                            div { class: "field-label is-normal",
                                label { class: "label", {t!("link-check-label")} }
                            }
                            div { class: "field-body",
                                div { class: "field",
                                    if checking {
                                        p { class: "check-line",
                                            span { class: "check-dot check-never" }
                                            {t!("link-check-checking")}
                                        }
                                    }
                                    if let Some(check) = shown_check.clone() {
                                        CheckLine { check }
                                    }
                                    if let Some(error) = check_error.clone() {
                                        p { class: "check-line check-failed", "{error}" }
                                    }
                                    if let Some(checks) = history_strip.clone() {
                                        div { class: "check-history",
                                            span { class: "check-history-label", {t!("link-check-history")} }
                                            for (index , check) in checks.into_iter().rev().enumerate() {
                                                CheckDot { key: "{index}", check: Some(check) }
                                            }
                                        }
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
                    if let Some(url) = preview_url {
                        div { class: "field is-horizontal is-wider",
                            div { class: "field-label is-normal",
                                label { class: "label", {t!("link-preview-label")} }
                            }
                            div { class: "field-body",
                                div { class: "field",
                                    button {
                                        class: "button is-light is-small",
                                        r#type: "button",
                                        onclick: move |_| show_preview.set(!show_preview()),
                                        if show_preview() {
                                            {t!("link-preview-hide")}
                                        } else {
                                            {t!("link-preview-show")}
                                        }
                                    }
                                    if show_preview() {
                                        if embeddable == Some(false) {
                                            div { class: "notification is-warning is-light mt-2",
                                                {t!("link-preview-blocked")}
                                            }
                                        } else {
                                            // Sandboxed without any permission: no scripts, forms or cookies.
                                            iframe {
                                                class: "link-preview mt-2",
                                                src: "{url}",
                                                "sandbox": "",
                                                referrerpolicy: "no-referrer",
                                            }
                                        }
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
fn EditFooter(edit_link: Signal<Option<EditDialog>>, reload_trigger: Signal<u32>) -> Element {
    rsx! {
        footer { class: "modal-card-foot is-justify-content-flex-end",
            div { class: "buttons",
                Buttons { edit_link, reload_trigger }
            }
        }
    }
}

#[component]
fn Buttons(mut edit_link: Signal<Option<EditDialog>>, mut reload_trigger: Signal<u32>) -> Element {
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
