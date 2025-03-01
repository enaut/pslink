mod link_display;
mod link_edit;
mod new_link_button;
mod stats;

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
    datatypes::FullLink,
};

use crate::links::link_display::LinkDisplay;
use crate::links::link_edit::LinkEdit;
use crate::links::new_link_button::NewLinkButton;
use crate::{navbar::Route, PslinkContext};

const LINKS_CSS: Asset = asset!("/assets/styling/links.css");

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
}

trait OptionEditDialog {
    fn set_edit_dialog(
        &mut self,
        id: Option<i64>,
        code: String,
        title: String,
        target: String,
        author: Option<i64>,
        edit_mode: EditMode,
    );
    fn update_code(&mut self, code: String);
    fn update_title(&mut self, title: String);
    fn update_target(&mut self, target: String);
    fn set_edit_mode(&mut self, edit_mode: EditMode);
}

impl OptionEditDialog for Signal<Option<EditDialog>> {
    fn set_edit_dialog(
        &mut self,
        id: Option<i64>,
        code: String,
        title: String,
        target: String,
        author: Option<i64>,
        edit_mode: EditMode,
    ) {
        let qr_string = calculate_qr_code(&code);
        if let Some(mut dialog) = self() {
            dialog.link_delta = LinkDelta {
                id,
                author: None,
                edit: edit_mode,
                title,
                target,
                code,
            };
            dialog.qr = qr_string;
            self.set(Some(dialog));
        } else {
            self.set(Some(EditDialog {
                link_delta: LinkDelta {
                    id,
                    author,
                    edit: edit_mode,
                    title,
                    target,
                    code,
                },
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

    fn set_edit_mode(&mut self, edit_mode: EditMode) {
        info!("Updating edit to: {:?}", &edit_mode);
        if let Some(mut dialog) = self() {
            dialog.link_delta.edit = edit_mode;
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
                div {
                    NewLinkButton { edit_link }
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
