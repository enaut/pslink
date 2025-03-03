use dioxus::{logger::tracing::info, prelude::*};
use indexmap::IndexMap;
use pslink_shared::{
    apirequests::{general::EditMode, users::Role},
    datatypes::User,
};

use crate::users::{EditDialog, OptionUserEditDialog as _};

#[component]
pub fn UserEdit(
    edit_dialog_signal: Signal<Option<EditDialog>>,
    users: Resource<IndexMap<String, User>>,
) -> Element {
    let change_evt = move |evt: KeyboardEvent| {
        if evt.key() == Key::Escape {
            edit_dialog_signal.set(None);
        }
    };
    let mut grab_focus = use_signal(move || true);
    let _focus_grabber = use_resource(move || async move {
        if grab_focus() && edit_dialog_signal().is_some() {
            edit_dialog_signal.focus_username().await;
            grab_focus.set(false);
        }
    });
    if edit_dialog_signal().is_some() {
        rsx! {
            div { class: "modal is-active", onkeydown: change_evt,
                div { class: "modal-background" }
                div { class: "modal-card",
                    header { class: "modal-card-head",
                        p { class: "modal-card-title",
                            "Edit User {edit_dialog_signal().expect(\"a user should be loaded.\").user_delta.username}"
                        }
                        button {
                            "aria-label": "close",
                            class: "delete",
                            onclick: move |_| {
                                edit_dialog_signal.set(None);
                            },
                        }
                    }
                    div { class: "modal-card-body",
                        div { class: "field is-horizontal is-wider",
                            div { class: "field-label is-normal",
                                label { class: "label", "Username" }
                            }
                            div { class: "field-body",
                                div { class: "field",
                                    p { class: "control",
                                        input {
                                            autofocus: true,
                                            onmounted: move |e| {
                                                edit_dialog_signal.set_username_field(Some(e.data()));
                                                grab_focus.set(true);
                                            },
                                            placeholder: "Username",
                                            value: "{edit_dialog_signal().expect(\"dialog defined\").user_delta.username}",
                                            r#type: "text",
                                            class: "input",
                                            oninput: move |e| {
                                                edit_dialog_signal.update_username(e.value());
                                            },
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "field is-horizontal is-wider",
                            div { class: "field-label is-normal",
                                label { class: "label", "Email" }
                            }
                            div { class: "field-body",
                                div { class: "field",
                                    p { class: "control",
                                        input {
                                            placeholder: "E-Mail",
                                            value: "{edit_dialog_signal().expect(\"dialog defined\").user_delta.email}",
                                            r#type: "text",
                                            class: "input",
                                            oninput: move |e| {
                                                edit_dialog_signal.update_email(e.value());
                                            },
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "field is-horizontal is-wider",
                            div { class: "field-label is-normal",
                                label { class: "label", "Password" }
                            }
                            div { class: "field-body",
                                div { class: "field",
                                    p { class: "control",
                                        input {
                                            placeholder: "Password",
                                            value: if edit_dialog_signal().expect("dialog defined").user_delta.password.is_some() { "{edit_dialog_signal().expect(\"dialog defined\").user_delta.password.unwrap()}" },
                                            r#type: "text",
                                            class: "input",
                                            oninput: move |e| {
                                                let e = e.value();
                                                if e.is_empty() {
                                                    edit_dialog_signal.update_password(None);
                                                } else {
                                                    edit_dialog_signal.update_password(Some(e.trim().to_owned()));
                                                }
                                            },
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "field is-horizontal is-wider",
                            div { class: "field-label is-normal",
                                label { class: "label", "Berechtigung" }
                            }
                            div { class: "field-body",
                                p { class: "control", style: "width: 100%",
                                    div { class: "select is-fullwidth",
                                        select {
                                            oninput: move |e| {
                                                let role = e.value().parse::<i64>().expect("Role must be a number");
                                                edit_dialog_signal.update_role(Role::convert(role));
                                                info!(
                                                    "Role: {:?}", edit_dialog_signal().expect("dialog defined").user_delta.role
                                                );
                                            },
                                            option {
                                                value: Role::Regular.to_i64(),
                                                selected: edit_dialog_signal().expect("dialog defined").user_delta.role == Role::Regular,
                                                "Normalnutzer"
                                            }
                                            option {
                                                value: Role::Admin.to_i64(),
                                                selected: edit_dialog_signal().expect("dialog defined").user_delta.role == Role::Admin,
                                                "Admin"
                                            }
                                            option {
                                                value: Role::Disabled.to_i64(),
                                                selected: edit_dialog_signal().expect("dialog defined").user_delta.role == Role::Disabled
                                                    || edit_dialog_signal().expect("dialog defined").user_delta.role
                                                        == Role::NotAuthenticated,
                                                "Deaktiviert"
                                            }

                                        }
                                    }
                                }
                            }
                        }
                        ConfirmDialog { edit_dialog_signal, users }
                    }
                    EditFooter { edit_dialog_signal, users }
                }
            }
        }
    } else {
        rsx! {}
    }
}

#[component]
pub fn ConfirmDialog(
    edit_dialog_signal: Signal<Option<EditDialog>>,
    users: Resource<IndexMap<String, User>>,
) -> Element {
    if let Some(dialog) = edit_dialog_signal() {
        if dialog.user_delta.edit != EditMode::Delete(true) {
            return rsx! {};
        }
        rsx! {
            div { class: "notification is-danger",
                "Einen Nutzer zu löschen ist meist nicht sinnvoll, da die erstellten Links dann Besitzerlos sind. Besser wäre es einfach das Passwort zu ändern."
            }
        }
    } else {
        rsx! {}
    }
}

#[component]
fn EditFooter(
    edit_dialog_signal: Signal<Option<EditDialog>>,
    users: Resource<IndexMap<String, User>>,
) -> Element {
    rsx! {
        footer { class: "modal-card-foot is-justify-content-flex-end",
            div { class: "buttons",
                Buttons { edit_dialog_signal, users }
            }
        }
    }
}

#[component]
fn Buttons(
    mut edit_dialog_signal: Signal<Option<EditDialog>>,
    users: Resource<IndexMap<String, User>>,
) -> Element {
    if let Some(EditDialog { user_delta, .. }) = edit_dialog_signal() {
        info!("Edit mode: {:?}", user_delta.edit);
        match user_delta.edit {
            EditMode::Create => {
                return rsx! {
                    button {
                        class: "button is-success",
                        onclick: {
                            move |_e: Event<MouseData>| {
                                info!("Save edits");
                                async move {
                                    if let Some(dialog) = edit_dialog_signal() {
                                        let user_delta = dialog.user_delta;
                                        info!("User delta: {:?}", user_delta);
                                        let _res = backend::user_api::create_user(user_delta).await;
                                        users.restart();
                                        edit_dialog_signal.set(None);
                                    }
                                }
                            }
                        },
                        "Neuen Nutzer erstellen"
                    }
                };
            }
            EditMode::Edit => {
                return rsx! {
                    button {
                        class: "button is-danger",
                        onclick: move |_e: Event<MouseData>| {
                            edit_dialog_signal.set_edit_mode(EditMode::Delete(true));
                        },
                        "Nutzer Löschen"
                    }
                    button {
                        class: "button is-success",
                        onclick: {
                            move |_e: Event<MouseData>| {
                                info!("Save edits");
                                async move {
                                    if let Some(dialog) = edit_dialog_signal() {
                                        let user_delta = dialog.user_delta;
                                        info!("User delta: {:?}", user_delta);
                                        let _res = backend::user_api::update_user(user_delta).await;
                                        users.restart();
                                        edit_dialog_signal.set(None);
                                    }
                                }
                            }
                        },
                        "Nutzer Verändern"
                    }
                };
            }
            EditMode::Delete(ask) => {
                if ask {
                    return rsx! {
                        button {
                            class: "button is-danger",
                            onclick: {
                                move |_e: Event<MouseData>| {
                                    info!("Delete User");
                                    async move {
                                        if let Some(dialog) = edit_dialog_signal() {
                                            let user_delta = dialog.user_delta;
                                            info!("User delta: {:?}", user_delta);
                                            let _res = backend::user_api::delete_user(
                                                    user_delta.id.expect("User ID must be set"),
                                                )
                                                .await;
                                            users.restart();
                                            edit_dialog_signal.set(None);
                                        }
                                    }
                                }
                            },
                            "Nutzer wirklich Löschen"
                        }
                    };
                } else {
                    return rsx! {
                        button {
                            class: "button is-danger",
                            onclick: move |_e: Event<MouseData>| {
                                edit_dialog_signal.set_edit_mode(EditMode::Delete(true));
                            },
                            "Nutzer Löschen"
                        }
                    };
                }
            }
        }
    } else {
        return rsx! {};
    }
}
