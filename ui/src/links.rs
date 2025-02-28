use dioxus::{logger::tracing::info, prelude::*};
use fast_qr::{
    convert::{svg::SvgBuilder, Builder as _, Shape},
    QRBuilder,
};
use indexmap::IndexMap;
use pslink_shared::{
    apirequests::{
        general::{EditMode, Filter, Operation, Ordering},
        links::{LinkDelta, LinkOverviewColumns, LinkRequestForm},
    },
    datatypes::{Clicks, FullLink},
};

use crate::{navbar::Route, PslinkContext};

const LINKS_CSS: Asset = asset!("/assets/styling/links.css");

const TRASH_SVG: Asset = asset!("/assets/trash.svg");
const SEARCH_SVG: Asset = asset!("/assets/search.svg");
const RELOAD_SVG: Asset = asset!("/assets/reload.svg");

fn toggle_column(
    ordering: Option<Operation<LinkOverviewColumns, Ordering>>,
    new_column: LinkOverviewColumns,
) -> Option<Operation<LinkOverviewColumns, Ordering>> {
    if let Some(Operation { column, value }) = ordering {
        if column == new_column {
            let new_value = match value {
                Ordering::Ascending => Ordering::Descending,
                Ordering::Descending => Ordering::Ascending,
            };
            return Some(Operation {
                column,
                value: new_value,
            });
        }
    };

    Some(Operation {
        column: new_column,
        value: Ordering::Ascending,
    })
}

#[derive(Clone)]
struct EditDialog {
    link_delta: LinkDelta,
    qr: String,
    old_code: Option<String>,
}

trait OptionEditDialog {
    fn set_edit_dialog(
        &mut self,
        id: Option<i64>,
        code: String,
        title: String,
        target: String,
        author: Option<i64>,
        old_code: Option<String>,
    );
    fn update_code(&mut self, code: String);
    fn update_title(&mut self, title: String);
    fn update_target(&mut self, target: String);
}

impl OptionEditDialog for Signal<Option<EditDialog>> {
    fn set_edit_dialog(
        &mut self,
        id: Option<i64>,
        code: String,
        title: String,
        target: String,
        author: Option<i64>,
        old_code: Option<String>,
    ) {
        let qr_string = calculate_qr_code(&code);
        if let Some(mut dialog) = self() {
            dialog.link_delta = LinkDelta {
                id,
                author: None,
                edit: EditMode::Edit,
                title,
                target,
                code,
            };
            dialog.old_code = old_code;
            dialog.qr = qr_string;
            self.set(Some(dialog));
        } else {
            self.set(Some(EditDialog {
                link_delta: LinkDelta {
                    id,
                    author,
                    edit: EditMode::Edit,
                    title,
                    target,
                    code,
                },
                old_code,
                qr: qr_string,
            }))
        }
    }
    fn update_code(&mut self, code: String) {
        info!("Updating code to: {}", code);
        if let Some(mut dialog) = self() {
            dialog.qr = calculate_qr_code(&code);
            dialog.link_delta.code = code;
            self.set(Some(dialog));
        }
    }
    fn update_title(&mut self, title: String) {
        info!("Updating title to: {}", title);
        if let Some(mut dialog) = self() {
            dialog.link_delta.title = title;
            info!("Updated dialog: {:?}", dialog.link_delta.title);
            self.set(Some(dialog));
        };
    }
    fn update_target(&mut self, target: String) {
        info!("Updating target to: {}", target);
        if let Some(mut dialog) = self() {
            dialog.link_delta.target = target;
            self.set(Some(dialog));
        };
    }
}
fn calculate_qr_code(code: &str) -> String {
    if code == "" {
        return "".to_string();
    }
    let qrcode = QRBuilder::new(format!("http://fhs.li/{}/", code))
        .ecl(fast_qr::ECL::L)
        .build();
    if let Ok(qrcode) = qrcode {
        let svg = SvgBuilder::default().shape(Shape::Square).to_str(&qrcode);
        svg
    } else {
        info!("Failed to create QR code");
        "".to_string()
    }
}

#[component]
pub fn Links() -> Element {
    let PslinkContext { user } = use_context::<PslinkContext>();
    let mut code_filter = use_signal(|| "".to_string());
    let mut description_filter = use_signal(|| "".to_string());
    let mut target_filter = use_signal(|| "".to_string());
    let mut username_filter = use_signal(|| "".to_string());
    let mut order_by = use_signal(|| Option::<Operation<LinkOverviewColumns, Ordering>>::None);
    let edit_link = use_signal(|| None);
    let links = use_resource(move || async move {
        let mut form = LinkRequestForm::default();
        form.filter[LinkOverviewColumns::Code] = Filter {
            sieve: code_filter(),
        };
        form.filter[LinkOverviewColumns::Description] = Filter {
            sieve: description_filter(),
        };
        form.filter[LinkOverviewColumns::Target] = Filter {
            sieve: target_filter(),
        };
        form.filter[LinkOverviewColumns::Author] = Filter {
            sieve: username_filter(),
        };
        form.order = order_by();
        let res: IndexMap<String, FullLink> = backend::link_api::list_all_allowed(form)
            .await
            .expect("Links")
            .list
            .into_iter()
            .map(|item| (item.link.code.clone(), item))
            .collect();
        res
    });
    let link_codes = use_memo(move || {
        links()
            .as_ref()
            .map(|links| links.keys().cloned().collect::<Vec<String>>())
    });

    rsx! {
        document::Stylesheet { href: LINKS_CSS }
        LinkEdit { edit_link, links }
        if let Some(_user) = user.as_ref() {
            div {
                table { class: "table is-bordered is-striped is-hoverable is-fullwidth",
                    tbody {
                        tr {
                            th {
                                class: "headlines",
                                onclick: move |_| {
                                    order_by.set(toggle_column(order_by(), LinkOverviewColumns::Code));
                                },
                                "Link code"
                            }
                            th {
                                class: "headlines",
                                onclick: move |_| {
                                    order_by.set(toggle_column(order_by(), LinkOverviewColumns::Description));
                                },
                                "Description"
                            }
                            th {
                                class: "headlines",
                                onclick: move |_| {
                                    order_by.set(toggle_column(order_by(), LinkOverviewColumns::Target));
                                },
                                "Link target"
                            }
                            th {
                                class: "headlines",
                                onclick: move |_| {
                                    order_by.set(toggle_column(order_by(), LinkOverviewColumns::Author));
                                },
                                "Username"
                            }
                            th {
                                class: "headlines",
                                onclick: move |_| {
                                    order_by.set(toggle_column(order_by(), LinkOverviewColumns::Statistics));
                                },
                                "Statistics"
                            }
                            th {}
                            th {}
                        }
                        tr {
                            td {
                                div { class: "control has-icons-left has-icons-right is-small",
                                    input {
                                        r#type: "search",
                                        placeholder: "Filter according to...",
                                        value: "{code_filter}",
                                        class: "input is-small",
                                        oninput: move |e| {
                                            code_filter.set(e.value());
                                        },
                                    }
                                    span { class: "icon is-small is-left",
                                        img { src: SEARCH_SVG }
                                    }
                                }
                            }
                            td {
                                div { class: "control has-icons-left has-icons-right is-small",
                                    input {
                                        value: "{description_filter}",
                                        r#type: "search",
                                        placeholder: "Filter according to...",
                                        class: "input is-small",
                                        oninput: move |e| {
                                            description_filter.set(e.value());
                                        },
                                    }
                                    span { class: "icon is-small is-left",
                                        img { src: SEARCH_SVG }
                                    }
                                }
                            }
                            td {
                                div { class: "control has-icons-left has-icons-right is-small",
                                    input {
                                        r#type: "search",
                                        value: "{target_filter}",
                                        placeholder: "Filter according to...",
                                        class: "input is-small",
                                        oninput: move |e| {
                                            target_filter.set(e.value());
                                        },
                                    }
                                    span { class: "icon is-small is-left",
                                        img { src: SEARCH_SVG }
                                    }
                                }
                            }
                            td {
                                div { class: "control has-icons-left has-icons-right is-small",
                                    input {
                                        placeholder: "Filter according to...",
                                        value: "{username_filter}",
                                        class: "input is-small",
                                        r#type: "search",
                                        oninput: move |e| {
                                            username_filter.set(e.value());
                                        },
                                    }
                                    span { class: "icon is-small is-left",
                                        img { src: SEARCH_SVG }
                                    }
                                }
                            }
                            td {}
                            td {}
                            td {}
                        }
                        if links().is_some() {
                            for code in link_codes().expect("Links not loaded") {
                                LinkDisplay {
                                    key: code.clone(),
                                    current_code: code,
                                    links,
                                    link_signal: edit_link.clone(),
                                }
                            }
                        }
                    }
                }
                a { class: "loadmore button",
                    img { src: RELOAD_SVG, class: "reloadicon" }
                    "load more links"
                }
            }
        } else {
            div { class: "centered",
                div { class: "boxed",
                    div { "Loading..." }
                    Link { to: Route::LoginScreen {}, "Login" }
                }
            }
        }
    }
}

#[component]
fn LinkDisplay(
    current_code: String,
    links: Resource<IndexMap<String, FullLink>>,
    link_signal: Signal<Option<EditDialog>>,
) -> Element {
    let cached_code = current_code.clone();
    let ll = use_memo(move || {
        links
            .as_ref()
            .expect("Links loaded")
            .get(&current_code)
            .unwrap()
            .clone()
    });

    rsx! {
        tr {
            onclick: move |_| {
                info!("Edit link");
                link_signal
                    .set_edit_dialog(
                        Some(ll().link.id),
                        ll().link.code.clone(),
                        ll().link.title.clone(),
                        ll().link.target.clone(),
                        None,
                        Some(cached_code.clone()),
                    );
            },

            td { "{ll().link.code}" }
            td { "{ll().link.title}" }
            td { "{ll().link.target}" }
            td { "{ll().user.username}" }
            td {
                Stats { clicks: ll().clicks }
            }
            td { class: "table_qr" }
            td {
                img { src: TRASH_SVG, class: "trashicon" }
            }
        }
    }
}

#[component]
fn Stats(clicks: Clicks) -> Element {
    match clicks {
        Clicks::Count(count) => rsx! {
            div { "{count.number}" }
        },
        Clicks::Extended(stats) => rsx! {
            div { "{stats.total.number}" }
        },
    }
}
#[component]
fn LinkEdit(
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
                    }
                    footer { class: "modal-card-foot is-justify-content-flex-end",
                        div { class: "buttons",
                            button {
                                class: "button is-success",
                                onclick: {
                                    move |_e: Event<MouseData>| {
                                        info!("Save edits");
                                        async move {
                                            if let Some(dialog) = edit_link() {
                                                let link_delta = dialog.link_delta;
                                                info!("Link delta: {:?}", link_delta);
                                                let res = backend::link_api::save_link(link_delta).await;
                                                links.restart();
                                                info!("Save result: {:?}", res);
                                                edit_link.set(None);
                                            } else {
                                                info!("Edit dialog is not open");
                                            }
                                        }
                                    }
                                },
                                "Save Edits"
                            }
                        }
                    }
                }
            }
        }
    } else {
        rsx! {}
    }
}
