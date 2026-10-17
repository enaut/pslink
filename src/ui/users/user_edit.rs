use std::rc::Rc;

use dioxus::{logger::tracing::info, prelude::*};
use dioxus_i18n::t;

use crate::{
    domain::{models::Role, requests::EditMode},
    ui::{PslinkContext, users::EditDialog},
};

#[component]
pub fn UserEdit(
    mut edit_dialog_signal: Signal<Option<EditDialog>>,
    reload_trigger: Signal<u32>,
) -> Element {
    let PslinkContext { user, .. } = use_context::<PslinkContext>();
    let on_esc_event = move |evt: KeyboardEvent| {
        if evt.key() == Key::Named(NamedKey::Escape) {
            edit_dialog_signal.set(None);
        }
    };
    let mut username_field: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    let _focus_grabber = use_resource(move || async move {
        if let Some(field) = username_field() {
            let _ = field.set_focus(true).await;
        }
    });

    let Some(dialog) = edit_dialog_signal() else {
        return rsx! {};
    };

    let is_admin = user().map(|u| u.role == Role::Admin).unwrap_or(false);

    rsx! {
        div { class: "modal is-active", onkeydown: on_esc_event,
            div { class: "modal-background" }
            div { class: "modal-card",
                CardHeader { edit_dialog_signal }
                div { class: "modal-card-body",
                    if let Some(err) = &dialog.error {
                        div { class: "notification is-danger", "{err}" }
                    }
                    div { class: "field is-horizontal is-wider",
                        div { class: "field-label is-normal",
                            label { class: "label", {t!("user-edit-label-username")} }
                        }
                        div { class: "field-body",
                            div { class: "field",
                                p { class: "control",
                                    input {
                                        autofocus: true,
                                        onmounted: move |e| {
                                            username_field.set(Some(e.data()));
                                        },
                                        placeholder: t!("user-edit-placeholder-username"),
                                        value: "{dialog.user_delta.username}",
                                        r#type: "text",
                                        class: "input",
                                        oninput: move |e| {
                                            if let Some(d) = edit_dialog_signal.write().as_mut() {
                                                d.user_delta.username = e.value();
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
                            label { class: "label", {t!("user-edit-label-email")} }
                        }
                        div { class: "field-body",
                            div { class: "field",
                                p { class: "control",
                                    input {
                                        placeholder: t!("user-edit-placeholder-email"),
                                        value: "{dialog.user_delta.email}",
                                        r#type: "text",
                                        class: "input",
                                        oninput: move |e| {
                                            if let Some(d) = edit_dialog_signal.write().as_mut() {
                                                d.user_delta.email = e.value();
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
                            label { class: "label", {t!("user-edit-label-password")} }
                        }
                        div { class: "field-body",
                            div { class: "field",
                                p { class: "control",
                                    input {
                                        placeholder: t!("user-edit-placeholder-password"),
                                        value: dialog.user_delta.password.as_deref().unwrap_or(""),
                                        r#type: "password",
                                        class: "input",
                                        oninput: move |e| {
                                            let val = e.value();
                                            if let Some(d) = edit_dialog_signal.write().as_mut() {
                                                if val.is_empty() {
                                                    d.user_delta.password = None;
                                                } else {
                                                    d.user_delta.password = Some(val);
                                                }
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
                            label { class: "label", {t!("user-edit-label-role")} }
                        }
                        div { class: "field-body",
                            p { class: "control", style: "width: 100%",
                                div { class: "select is-fullwidth",
                                    select {
                                        disabled: !is_admin,
                                        oninput: move |e| {
                                            if let Ok(role_num) = e.value().parse::<i64>()
                                                && let Some(d) = edit_dialog_signal.write().as_mut()
                                            {
                                                d.user_delta.role = Role::convert(role_num);
                                                d.error = None;
                                            }
                                        },
                                        option {
                                            value: Role::Regular.to_i64(),
                                            selected: dialog.user_delta.role == Role::Regular,
                                            {t!("user-edit-role-regular")}
                                        }
                                        option {
                                            value: Role::Admin.to_i64(),
                                            selected: dialog.user_delta.role == Role::Admin,
                                            {t!("user-edit-role-admin")}
                                        }
                                        option {
                                            value: Role::Disabled.to_i64(),
                                            selected: dialog.user_delta.role == Role::Disabled
                                                || dialog.user_delta.role == Role::NotAuthenticated,
                                            {t!("user-edit-role-disabled")}
                                        }
                                    }
                                }
                            }
                        }
                    }

                    ConfirmDialog { edit_dialog_signal }
                }
                EditFooter { edit_dialog_signal, reload_trigger }
            }
        }
    }
}

#[component]
pub fn ConfirmDialog(edit_dialog_signal: Signal<Option<EditDialog>>) -> Element {
    if let Some(dialog) = edit_dialog_signal()
        && dialog.user_delta.edit == EditMode::Delete(true)
    {
        return rsx! {
            div { class: "notification is-danger", {t!("user-edit-delete-warning")} }
        };
    }
    rsx! {}
}

#[component]
fn EditFooter(
    edit_dialog_signal: Signal<Option<EditDialog>>,
    reload_trigger: Signal<u32>,
) -> Element {
    rsx! {
        footer { class: "modal-card-foot is-justify-content-flex-end",
            div { class: "buttons",
                Buttons { edit_dialog_signal, reload_trigger }
            }
        }
    }
}

#[component]
fn Buttons(
    mut edit_dialog_signal: Signal<Option<EditDialog>>,
    mut reload_trigger: Signal<u32>,
) -> Element {
    let PslinkContext { mut user, .. } = use_context::<PslinkContext>();
    let Some(EditDialog { user_delta, .. }) = edit_dialog_signal() else {
        return rsx! {};
    };
    let is_admin = user().map(|u| u.role == Role::Admin).unwrap_or(false);

    info!("Edit mode: {:?}", user_delta.edit);
    match user_delta.edit {
        EditMode::Create => rsx! {
            button {
                class: "button is-success",
                onclick: move |_e: Event<MouseData>| async move {
                    if let Some(dialog) = edit_dialog_signal() {
                        match crate::api::user_api::create_user(dialog.user_delta).await {
                            Ok(_) => {
                                reload_trigger.set(reload_trigger().wrapping_add(1));
                                edit_dialog_signal.set(None);
                            }
                            Err(e) => {
                                if let Some(d) = edit_dialog_signal.write().as_mut() {
                                    d.error = Some(e.to_string());
                                }
                            }
                        }
                    }
                },
                {t!("user-edit-button-create")}
            }
        },
        EditMode::Edit => rsx! {
            if is_admin {
                button {
                    class: "button is-danger",
                    onclick: move |_e: Event<MouseData>| {
                        if let Some(d) = edit_dialog_signal.write().as_mut() {
                            d.user_delta.edit = EditMode::Delete(true);
                        }
                    },
                    {t!("user-edit-button-delete")}
                }
            }
            button {
                class: "button is-success",
                onclick: move |_e: Event<MouseData>| async move {
                    if let Some(dialog) = edit_dialog_signal() {
                        match crate::api::user_api::update_user(dialog.user_delta).await {
                            Ok(updated) => {
                                if let Some(curr) = user()
                                    && curr.id == updated.id
                                {
                                    user.set(Some(updated));
                                }
                                reload_trigger.set(reload_trigger().wrapping_add(1));
                                edit_dialog_signal.set(None);
                            }
                            Err(e) => {
                                if let Some(d) = edit_dialog_signal.write().as_mut() {
                                    d.error = Some(e.to_string());
                                }
                            }
                        }
                    }
                },
                {t!("user-edit-button-update")}
            }
        },
        EditMode::Delete(ask) => {
            if ask {
                rsx! {
                    button {
                        class: "button is-danger",
                        onclick: move |_e: Event<MouseData>| async move {
                            if let Some(dialog) = edit_dialog_signal()
                                && let Some(uid) = dialog.user_delta.id
                            {
                                match crate::api::user_api::delete_user(uid).await {
                                    Ok(_) => {
                                        reload_trigger.set(reload_trigger().wrapping_add(1));
                                        edit_dialog_signal.set(None);
                                    }
                                    Err(e) => {
                                        if let Some(d) = edit_dialog_signal.write().as_mut() {
                                            d.error = Some(e.to_string());
                                        }
                                    }
                                }
                            }
                        },
                        {t!("user-edit-button-confirm-delete")}
                    }
                }
            } else {
                rsx! {
                    button {
                        class: "button is-danger",
                        onclick: move |_e: Event<MouseData>| {
                            if let Some(d) = edit_dialog_signal.write().as_mut() {
                                d.user_delta.edit = EditMode::Delete(true);
                            }
                        },
                        {t!("user-edit-button-delete")}
                    }
                }
            }
        }
    }
}

#[component]
fn CardHeader(mut edit_dialog_signal: Signal<Option<EditDialog>>) -> Element {
    if let Some(EditDialog { ref user_delta, .. }) = edit_dialog_signal() {
        rsx! {
            header { class: "modal-card-head",
                p { class: "modal-card-title",
                    {t!("user-edit-title", username : &user_delta.username)}
                }
                button {
                    "aria-label": "close",
                    class: "delete",
                    onclick: move |_| {
                        edit_dialog_signal.set(None);
                    },
                }
            }
        }
    } else {
        rsx! {}
    }
}
