use std::rc::Rc;

use dioxus::{logger::tracing::info, prelude::*};
use dioxus_i18n::t;
use indexmap::IndexMap;
use pslink_shared::{apirequests::general::EditMode, datatypes::FullLink};

use crate::links::{EditDialog, OptionEditDialog as _};

#[component]
pub fn LinkEdit(
    edit_link: Signal<Option<EditDialog>>,
    links: Resource<IndexMap<String, FullLink>>,
) -> Element {
    let on_esc_event = move |evt: KeyboardEvent| {
        if evt.key() == Key::Escape {
            edit_link.set(None);
        }
    };
    let mut description_field: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    let _focus_grabber = use_resource(move || async move {
        if description_field().is_some() {
            description_field()
                .expect("username field visible")
                .set_focus(true)
                .await
                .expect("failed to set focus");
        }
    });
    if edit_link().is_some() {
        rsx! {
            div { class: "modal is-active", onkeydown: on_esc_event,
                div { class: "modal-background" }
                div { class: "modal-card",
                    header { class: "modal-card-head",
                        p { class: "modal-card-title", {t!("link-edit-modal-title")} } // Title for the link editing modal
                        button {
                            "aria-label": "close",
                            class: "delete",
                            onclick: move |_| {
                                edit_link.set(None);
                            },
                        }
                    }
                    div { class: "modal-card-body",
                        div { class: "field is-horizontal",
                            div { class: "field-label is-normal",
                                label { class: "label", {t!("link-edit-field-description")} } // Label for description field
                            }
                            div { class: "field-body",
                                div { class: "field",
                                    p { class: "control",
                                        input {
                                            autofocus: true,
                                            onmounted: move |e| {
                                                description_field.set(Some(e.data()));
                                            },
                                            placeholder: t!("link-edit-placeholder-description"), // Placeholder for description input
                                            value: "{edit_link().expect(\"dialog defined\").link_delta.title}",
                                            r#type: "text",
                                            class: "input",
                                            oninput: move |e| {
                                                edit_link.update_title(e.value());
                                            },
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "field is-horizontal",
                            div { class: "field-label is-normal",
                                label { class: "label", {t!("link-edit-field-target")} } // Label for link target field
                            }
                            div { class: "field-body",
                                div { class: "field",
                                    p { class: "control",
                                        input {
                                            placeholder: t!("link-edit-placeholder-target"), // Placeholder for target input
                                            value: "{edit_link().expect(\"dialog defined\").link_delta.target}",
                                            r#type: "text",
                                            class: "input",
                                            oninput: move |e| {
                                                edit_link.update_target(e.value());
                                            },
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "field is-horizontal",
                            div { class: "field-label is-normal",
                                label { class: "label", {t!("link-edit-field-code")} } // Label for link code field
                            }
                            div { class: "field-body",
                                div { class: "field",
                                    p { class: "control",
                                        input {
                                            placeholder: t!("link-edit-placeholder-code"), // Placeholder for code input
                                            value: "{edit_link().expect(\"dialog defined\").link_delta.code}",
                                            r#type: "text",
                                            class: "input",
                                            oninput: move |e| {
                                                edit_link.update_code(e.value());
                                            },
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "field is-horizontal",
                            div { class: "field-label is-normal",
                                label { class: "label", {t!("link-edit-field-qrcode")} } // Label for QR code field
                            }
                            div { class: "field-body",
                                div {
                                    width: "133px",
                                    dangerous_inner_html: "{edit_link().expect(\"dialog defined\").qr}",
                                }
                            }
                        }
                        ConfirmDialog { edit_link, links }
                    }
                    EditFooter { edit_link, links }
                }
            }
        }
    } else {
        rsx! {}
    }
}

#[component]
pub fn ConfirmDialog(
    edit_link: Signal<Option<EditDialog>>,
    links: Resource<IndexMap<String, FullLink>>,
) -> Element {
    if let Some(dialog) = edit_link() {
        if dialog.link_delta.edit != EditMode::Delete(true) {
            return rsx! {};
        }
        rsx! {
            div { class: "notification is-danger", {t!("link-edit-delete-warning")} } // Warning message about deleting links
        }
    } else {
        rsx! {}
    }
}

#[component]
fn EditFooter(
    edit_link: Signal<Option<EditDialog>>,
    links: Resource<IndexMap<String, FullLink>>,
) -> Element {
    rsx! {
        footer { class: "modal-card-foot is-justify-content-flex-end",
            div { class: "buttons",
                Buttons { edit_link, links }
            }
        }
    }
}

#[component]
fn Buttons(
    mut edit_link: Signal<Option<EditDialog>>,
    links: Resource<IndexMap<String, FullLink>>,
) -> Element {
    if let Some(EditDialog { link_delta, .. }) = edit_link() {
        info!("Edit mode: {:?}", link_delta.edit);
        match link_delta.edit {
            EditMode::Create => {
                return rsx! {
                    button {
                        class: "button is-success",
                        onclick: {
                            move |_e: Event<MouseData>| {
                                info!("Save edits");
                                async move {
                                    if let Some(dialog) = edit_link() {
                                        let link_delta = dialog.link_delta;
                                        info!("Link delta: {:?}", link_delta);
                                        let _res = backend::link_api::create_link(link_delta).await;
                                        links.restart();
                                        edit_link.set(None);
                                    }
                                }
                            }
                        },
                        {t!("link-edit-button-create")} // Button text for creating a new link
                    }
                };
            }
            EditMode::Edit => {
                return rsx! {
                    button {
                        class: "button is-danger",
                        onclick: move |_e: Event<MouseData>| {
                            edit_link.set_edit_mode(EditMode::Delete(true));
                        },
                        {t!("link-edit-button-delete")} // Button text for deleting a link
                    }
                    button {
                        class: "button is-success",
                        onclick: {
                            move |_e: Event<MouseData>| {
                                info!("Save edits");
                                async move {
                                    if let Some(dialog) = edit_link() {
                                        let link_delta = dialog.link_delta;
                                        info!("Link delta: {:?}", link_delta);
                                        let _res = backend::link_api::save_link(link_delta).await;
                                        links.restart();
                                        edit_link.set(None);
                                    }
                                }
                            }
                        },
                        {t!("link-edit-button-modify")} // Button text for modifying a link
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
                                    info!("Delete link");
                                    async move {
                                        if let Some(dialog) = edit_link() {
                                            let link_delta = dialog.link_delta;
                                            info!("Link delta: {:?}", link_delta);
                                            let _res = backend::link_api::delete_link(
                                                    link_delta.id.expect("Link ID must be set"),
                                                )
                                                .await;
                                            links.restart();
                                            edit_link.set(None);
                                        }
                                    }
                                }
                            },
                            {t!("link-edit-button-confirm-delete")} // Button text for confirming link deletion
                        }
                    };
                } else {
                    return rsx! {
                        button {
                            class: "button is-danger",
                            onclick: move |_e: Event<MouseData>| {
                                edit_link.set_edit_mode(EditMode::Delete(true));
                            },
                            {t!("link-edit-button-delete")} // Button text for deleting a link
                        }
                    };
                }
            }
        }
    } else {
        return rsx! {};
    }
}
