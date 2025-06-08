use std::rc::Rc;

use dioxus::{logger::tracing::info, prelude::*};
use dioxus_i18n::t;
use indexmap::IndexMap;
use pslink_shared::{
    apirequests::{general::EditMode, users::Role},
    datatypes::User,
};

use crate::{
    PslinkContext,
    users::{EditDialog, OptionUserEditDialog as _},
};

#[component]
pub fn UserEdit(
    edit_dialog_signal: Signal<Option<EditDialog>>,
    users: Resource<IndexMap<String, User>>,
) -> Element {
    let PslinkContext { user, .. } = use_context::<PslinkContext>();
    let on_esc_event = move |evt: KeyboardEvent| {
        if evt.key() == Key::Escape {
            edit_dialog_signal.set(None);
        }
    };
    let mut username_field: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    let _focus_grabber = use_resource(move || async move {
        if username_field().is_some() {
            username_field()
                .expect("username field visible")
                .set_focus(true)
                .await
                .expect("failed to set focus");
        }
    });
    if edit_dialog_signal().is_some() {
        rsx! {
            div { class: "modal is-active", onkeydown: on_esc_event,
                div { class: "modal-background" }
                div { class: "modal-card",
                    CardHeader { edit_dialog_signal }
                    div { class: "modal-card-body",
                        div { class: "field is-horizontal is-wider",
                            div { class: "field-label is-normal",
                                label { class: "label", {t!("user-edit-label-username")} } // Label for username field in edit form
                            }
                            div { class: "field-body",
                                div { class: "field",
                                    p { class: "control",
                                        input {
                                            autofocus: true,
                                            onmounted: move |e| {
                                                username_field.set(Some(e.data()));
                                            },
                                            placeholder: t!("user-edit-placeholder-username"), // Placeholder text for username input field
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
                                label { class: "label", {t!("user-edit-label-email")} } // Label for email field in edit form
                            }
                            div { class: "field-body",
                                div { class: "field",
                                    p { class: "control",
                                        input {
                                            placeholder: t!("user-edit-placeholder-email"), // Placeholder text for email input field
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
                                label { class: "label", {t!("user-edit-label-password")} } // Label for password field in edit form
                            }
                            div { class: "field-body",
                                div { class: "field",
                                    p { class: "control",
                                        input {
                                            placeholder: t!("user-edit-placeholder-password"), // Placeholder text for password input field
                                            value: if edit_dialog_signal().expect("dialog defined").user_delta.password.is_some() { "" },
                                            r#type: "password",
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
                                label { class: "label", {t!("user-edit-label-role")} } // Label for role selection dropdown in edit form
                            }
                            div { class: "field-body",
                                p { class: "control", style: "width: 100%",
                                    div { class: "select is-fullwidth",
                                        select {
                                            disabled: user().expect("logged in").role != Role::Admin,
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
                                                {t!("user-edit-role-regular")} // Option for regular user role in dropdown
                                            }
                                            option {
                                                value: Role::Admin.to_i64(),
                                                selected: edit_dialog_signal().expect("dialog defined").user_delta.role == Role::Admin,
                                                {t!("user-edit-role-admin")} // Option for admin role in dropdown
                                            }
                                            option {
                                                value: Role::Disabled.to_i64(),
                                                selected: edit_dialog_signal().expect("dialog defined").user_delta.role == Role::Disabled
                                                    || edit_dialog_signal().expect("dialog defined").user_delta.role
                                                        == Role::NotAuthenticated,
                                                {t!("user-edit-role-disabled")} // Option for disabled role in dropdown
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
            div { class: "notification is-danger", {t!("user-edit-delete-warning")} } // Warning message displayed when attempting to delete a user
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
    let PslinkContext { user, .. } = use_context::<PslinkContext>();
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
                        {t!("user-edit-button-create")} // Button text for creating a new user
                    }
                };
            }
            EditMode::Edit => {
                return rsx! {
                    if user().expect("logged in").role == Role::Admin {
                        button {
                            class: "button is-danger",
                            onclick: move |_e: Event<MouseData>| {
                                edit_dialog_signal.set_edit_mode(EditMode::Delete(true));
                            },
                            {t!("user-edit-button-delete")} // Button text for deleting a user
                        }
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
                        {t!("user-edit-button-update")} // Button text for updating an existing user
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
                            {t!("user-edit-button-confirm-delete")} // Button text for confirming user deletion
                        }
                    };
                } else {
                    return rsx! {
                        button {
                            class: "button is-danger",
                            onclick: move |_e: Event<MouseData>| {
                                edit_dialog_signal.set_edit_mode(EditMode::Delete(true));
                            },
                            {t!("user-edit-button-delete")} // Button text for deleting a user
                        }
                    };
                }
            }
        }
    } else {
        return rsx! {};
    }
}

#[component]
fn CardHeader(mut edit_dialog_signal: Signal<Option<EditDialog>>) -> Element {
    if let Some(EditDialog { ref user_delta, .. }) = edit_dialog_signal() {
        rsx! {
            header { class: "modal-card-head",
                p { class: "modal-card-title",
                    {t!("user-edit-title", username : & user_delta.username)} // Title of the username in the edit dialog
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
