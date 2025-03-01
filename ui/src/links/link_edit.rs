use dioxus::{logger::tracing::info, prelude::*};
use indexmap::IndexMap;
use pslink_shared::{apirequests::general::EditMode, datatypes::FullLink};

use crate::links::{EditDialog, OptionEditDialog as _};

#[component]
pub fn LinkEdit(
    edit_link: Signal<Option<EditDialog>>,
    links: Resource<IndexMap<String, FullLink>>,
) -> Element {
    if edit_link().is_some() {
        rsx! {
            div { class: "modal is-active",
                div { class: "modal-background" }
                div { class: "modal-card",
                    header { class: "modal-card-head",
                        p { class: "modal-card-title", "Edit link" }
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
                                label { class: "label", "Description" }
                            }
                            div { class: "field-body",
                                div { class: "field",
                                    p { class: "control",
                                        input {
                                            placeholder: "Description",
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
                                label { class: "label", "Link target" }
                            }
                            div { class: "field-body",
                                div { class: "field",
                                    p { class: "control",
                                        input {
                                            placeholder: "Link target",
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
                                label { class: "label", "Link code" }
                            }
                            div { class: "field-body",
                                div { class: "field",
                                    p { class: "control",
                                        input {
                                            placeholder: "Link code",
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
                                label { class: "label", "QR Code" }
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
            div { class: "notification is-danger",
                "Einen Link zu löschen ist in der Regel nicht empfehlenswert. Es sollten nur Links gelöscht werden, die nirgends veröffentlicht wurden, oder die absichtlich ins Leere führen sollen."
            }
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
                        "Neuen Kurzlink erstellen"
                    }
                };
            }
            EditMode::Edit => {
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
                                        let _res = backend::link_api::save_link(link_delta).await;
                                        links.restart();
                                        edit_link.set(None);
                                    }
                                }
                            }
                        },
                        "Link Verändern"
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
                            "Link wirklich Löschen"
                        }
                    };
                } else {
                    return rsx! {
                        button {
                            class: "button is-danger",
                            onclick: move |_e: Event<MouseData>| {
                                edit_link.set_edit_mode(EditMode::Delete(true));
                            },
                            "Link Löschen"
                        }
                    };
                }
            }
        }
    } else {
        return rsx! {};
    }
}
