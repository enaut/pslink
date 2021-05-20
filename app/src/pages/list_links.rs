use std::cell::RefCell;

use enum_map::EnumMap;
use fluent::fluent_args;
use seed::{
    a, attrs, button, div, h1, img, input, log, prelude::*, section, span, table, td, th, tr, Url,
    C,
};

use shared::{
    apirequests::general::Ordering,
    apirequests::{
        general::{EditMode, Message, Operation, Status},
        links::{LinkDelta, LinkOverviewColumns, LinkRequestForm},
    },
    datatypes::FullLink,
};

use crate::i18n::I18n;

/// Unwrap a result and return it's content, or return from the function with another expression.
macro_rules! unwrap_or_return {
    ( $e:expr, $result:expr) => {
        match $e {
            Ok(x) => x,
            Err(_) => return $result,
        }
    };
}

pub fn init(mut url: Url, orders: &mut impl Orders<Msg>, i18n: I18n) -> Model {
    log!(url);
    orders.send_msg(Msg::Query(QueryMsg::Fetch));
    let edit_link = match url.next_path_part() {
        Some("create_link") => Some(RefCell::new(LinkDelta::default())),
        None | Some(_) => None,
    };
    log!(edit_link);

    Model {
        links: Vec::new(),
        i18n,
        formconfig: LinkRequestForm::default(),
        inputs: EnumMap::default(),
        edit_link,
        last_message: None,
        question: None,
    }
}

#[derive(Debug)]
pub struct Model {
    links: Vec<FullLink>,
    i18n: I18n,
    formconfig: LinkRequestForm,
    inputs: EnumMap<LinkOverviewColumns, FilterInput>,
    edit_link: Option<RefCell<LinkDelta>>,
    last_message: Option<Status>,
    question: Option<EditMsg>,
}

#[derive(Default, Debug, Clone)]
struct FilterInput {
    filter_input: ElRef<web_sys::HtmlInputElement>,
}

#[derive(Clone)]
pub enum Msg {
    Query(QueryMsg),
    Edit(EditMsg),
    ClearAll,
    SetMessage(String),
}

#[derive(Clone)]
pub enum QueryMsg {
    Fetch,
    OrderBy(LinkOverviewColumns),
    Received(Vec<FullLink>),
    CodeFilterChanged(String),
    DescriptionFilterChanged(String),
    TargetFilterChanged(String),
    AuthorFilterChanged(String),
}
/// All the messages on link editing
#[derive(Clone, Debug)]
pub enum EditMsg {
    EditSelected(LinkDelta),
    CreateNewLink,
    Created(Status),
    EditCodeChanged(String),
    EditDescriptionChanged(String),
    EditTargetChanged(String),
    MayDeleteSelected(LinkDelta),
    DeleteSelected(LinkDelta),
    SaveLink,
    FailedToCreateLink,
    FailedToDeleteLink,
    DeletedLink(Status),
}

/// # Panics
/// Sould only panic on bugs.
pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::Query(msg) => process_query_messages(msg, model, orders),
        Msg::Edit(msg) => process_edit_messages(msg, model, orders),
        Msg::ClearAll => {
            model.edit_link = None;
            model.last_message = None;
            model.question = None;
        }
        Msg::SetMessage(msg) => {
            model.edit_link = None;
            model.question = None;
            model.last_message = Some(Status::Error(Message { message: msg }));
        }
    }
}

/// # Panics
/// Sould only panic on bugs.
pub fn process_query_messages(msg: QueryMsg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        QueryMsg::Fetch => {
            orders.skip(); // No need to rerender
            load_links(model, orders)
        }
        QueryMsg::OrderBy(column) => {
            model.formconfig.order = model.formconfig.order.as_ref().map_or_else(
                || {
                    Some(Operation {
                        column: column.clone(),
                        value: Ordering::Ascending,
                    })
                },
                |order| {
                    Some(Operation {
                        column: column.clone(),
                        value: if order.column == column && order.value == Ordering::Ascending {
                            Ordering::Descending
                        } else {
                            Ordering::Ascending
                        },
                    })
                },
            );
            orders.send_msg(Msg::Query(QueryMsg::Fetch));

            model.links.sort_by(match column {
                LinkOverviewColumns::Code => {
                    |o: &FullLink, t: &FullLink| o.link.code.cmp(&t.link.code)
                }
                LinkOverviewColumns::Description => {
                    |o: &FullLink, t: &FullLink| o.link.title.cmp(&t.link.title)
                }
                LinkOverviewColumns::Target => {
                    |o: &FullLink, t: &FullLink| o.link.target.cmp(&t.link.target)
                }
                LinkOverviewColumns::Author => {
                    |o: &FullLink, t: &FullLink| o.user.username.cmp(&t.user.username)
                }
                LinkOverviewColumns::Statistics => {
                    |o: &FullLink, t: &FullLink| o.clicks.number.cmp(&t.clicks.number)
                }
            })
        }
        QueryMsg::Received(response) => {
            model.links = response;
        }
        QueryMsg::CodeFilterChanged(s) => {
            log!("Filter is: ", &s);
            let sanit = s.chars().filter(|x| x.is_alphanumeric()).collect();
            model.formconfig.filter[LinkOverviewColumns::Code].sieve = sanit;
            orders.send_msg(Msg::Query(QueryMsg::Fetch));
        }
        QueryMsg::DescriptionFilterChanged(s) => {
            log!("Filter is: ", &s);
            let sanit = s.chars().filter(|x| x.is_alphanumeric()).collect();
            model.formconfig.filter[LinkOverviewColumns::Description].sieve = sanit;
            orders.send_msg(Msg::Query(QueryMsg::Fetch));
        }
        QueryMsg::TargetFilterChanged(s) => {
            log!("Filter is: ", &s);
            let sanit = s.chars().filter(|x| x.is_alphanumeric()).collect();
            model.formconfig.filter[LinkOverviewColumns::Target].sieve = sanit;
            orders.send_msg(Msg::Query(QueryMsg::Fetch));
        }
        QueryMsg::AuthorFilterChanged(s) => {
            log!("Filter is: ", &s);
            let sanit = s.chars().filter(|x| x.is_alphanumeric()).collect();
            model.formconfig.filter[LinkOverviewColumns::Author].sieve = sanit;
            orders.send_msg(Msg::Query(QueryMsg::Fetch));
        }
    }
}
fn load_links(model: &Model, orders: &mut impl Orders<Msg>) {
    let data = model.formconfig.clone(); // complicated way to move into the closure
    orders.perform_cmd(async {
        let data = data;
        let request = unwrap_or_return!(
            Request::new("/admin/json/list_links/")
                .method(Method::Post)
                .json(&data),
            Msg::SetMessage("Failed to parse data".to_string())
        );
        let response = unwrap_or_return!(
            fetch(request).await,
            Msg::SetMessage("Failed to send data".to_string())
        );

        let response = unwrap_or_return!(
            response.check_status(),
            Msg::SetMessage("Wrong response code".to_string())
        );

        let links: Vec<FullLink> = unwrap_or_return!(
            response.json().await,
            Msg::SetMessage("Invalid response".to_string())
        );

        Msg::Query(QueryMsg::Received(links))
    });
}

pub fn process_edit_messages(msg: EditMsg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        EditMsg::EditSelected(link) => {
            log!("Editing link: ", link);
            model.last_message = None;
            model.edit_link = Some(RefCell::new(link))
        }
        EditMsg::CreateNewLink => {
            log!("Create new link!");
            model.edit_link = Some(RefCell::new(LinkDelta::default()))
        }
        EditMsg::Created(success_msg) => {
            model.last_message = Some(success_msg);
            model.edit_link = None;
            orders.send_msg(Msg::Query(QueryMsg::Fetch));
        }
        EditMsg::EditCodeChanged(s) => {
            if let Some(ref le) = model.edit_link {
                le.try_borrow_mut().expect("Failed to borrow mutably").code = s;
            }
        }
        EditMsg::EditDescriptionChanged(s) => {
            if let Some(ref le) = model.edit_link {
                le.try_borrow_mut().expect("Failed to borrow mutably").title = s;
            }
        }
        EditMsg::EditTargetChanged(s) => {
            if let Some(ref le) = model.edit_link {
                le.try_borrow_mut()
                    .expect("Failed to borrow mutably")
                    .target = s;
            }
        }
        EditMsg::SaveLink => {
            save_link(model, orders);
        }
        EditMsg::FailedToCreateLink => {
            log!("Failed to create Link");
        }
        link @ EditMsg::MayDeleteSelected(..) => {
            log!("Deleting link: ", link);
            model.last_message = None;
            model.edit_link = None;
            model.question = Some(link)
        }
        EditMsg::DeleteSelected(link) => {
            orders.perform_cmd(async {
                let data = link;
                let response = unwrap_or_return!(
                    fetch(
                        Request::new("/admin/json/delete_link/")
                            .method(Method::Post)
                            .json(&data)
                            .expect("serialization failed"),
                    )
                    .await,
                    Msg::Edit(EditMsg::FailedToDeleteLink)
                );

                let response = unwrap_or_return!(
                    response.check_status(),
                    Msg::SetMessage("Wrong response code!".to_string())
                );
                let message: Status = unwrap_or_return!(
                    response.json().await,
                    Msg::SetMessage(
                        "Failed to parse the response the link might be deleted however!"
                            .to_string()
                    )
                );

                Msg::Edit(EditMsg::DeletedLink(message))
            });
        }
        EditMsg::FailedToDeleteLink => {
            log!("Failed to delete Link");
        }
        EditMsg::DeletedLink(message) => {
            log!("Deleted link", message);
        }
    }
}

fn save_link(model: &Model, orders: &mut impl Orders<Msg>) {
    let data = model
        .edit_link
        .as_ref()
        .expect("should exist!")
        .borrow()
        .clone();
    orders.perform_cmd(async {
        let data = data;
        let request = unwrap_or_return!(
            Request::new(match data.edit {
                EditMode::Create => "/admin/json/create_link/",
                EditMode::Edit => "/admin/json/edit_link/",
            })
            .method(Method::Post)
            .json(&data),
            Msg::SetMessage("Failed to encode the link!".to_string())
        );
        let response =
            unwrap_or_return!(fetch(request).await, Msg::Edit(EditMsg::FailedToCreateLink));

        log!(response);
        let response = unwrap_or_return!(
            response.check_status(),
            Msg::SetMessage("Wrong response code".to_string())
        );

        let message: Status = unwrap_or_return!(
            response.json().await,
            Msg::SetMessage("Invalid response!".to_string())
        );

        Msg::Edit(EditMsg::Created(message))
    });
}

#[must_use]
pub fn view(model: &Model) -> Node<Msg> {
    let lang = &model.i18n.clone();
    let t = move |key: &str| lang.translate(key, None);
    section![
        if let Some(message) = &model.last_message {
            div![
                C!["message", "center"],
                div![
                    C!["closebutton"],
                    a!["\u{d7}"],
                    ev(Ev::Click, |_| Msg::ClearAll)
                ],
                match message {
                    Status::Success(m) | Status::Error(m) => &m.message,
                }
            ]
        } else {
            section![]
        },
        if let Some(question) = &model.question {
            div![
                C!["message", "center"],
                div![
                    C!["closebutton"],
                    a!["\u{d7}"],
                    ev(Ev::Click, |_| Msg::ClearAll)
                ],
                if let EditMsg::MayDeleteSelected(l) = question.clone() {
                    div![
                        lang.translate(
                            "really-delete",
                            Some(&fluent_args!["code" => l.code.clone()])
                        ),
                        a![t("no"), C!["button"], ev(Ev::Click, |_| Msg::ClearAll)],
                        a![
                            t("yes"),
                            C!["button"],
                            ev(Ev::Click, move |_| Msg::Edit(EditMsg::DeleteSelected(l)))
                        ]
                    ]
                } else {
                    span!()
                }
            ]
        } else {
            section![]
        },
        table![
            // Add the headlines
            view_link_table_head(&t),
            // Add filter fields right below the headlines
            view_link_table_filter_input(model, &t),
            // Add all the content lines
            model.links.iter().map(view_link)
        ],
        button![
            ev(Ev::Click, |_| Msg::Query(QueryMsg::Fetch)),
            "Fetch links"
        ],
        if let Some(l) = &model.edit_link {
            edit_or_create_link(l, t)
        } else {
            section!()
        }
    ]
}

fn view_link_table_head<F: Fn(&str) -> String>(t: F) -> Node<Msg> {
    tr![
        th![
            ev(Ev::Click, |_| Msg::Query(QueryMsg::OrderBy(
                LinkOverviewColumns::Code
            ))),
            t("link-code")
        ],
        th![
            ev(Ev::Click, |_| Msg::Query(QueryMsg::OrderBy(
                LinkOverviewColumns::Description
            ))),
            t("link-description")
        ],
        th![
            ev(Ev::Click, |_| Msg::Query(QueryMsg::OrderBy(
                LinkOverviewColumns::Target
            ))),
            t("link-target")
        ],
        th![
            ev(Ev::Click, |_| Msg::Query(QueryMsg::OrderBy(
                LinkOverviewColumns::Author
            ))),
            t("username")
        ],
        th![
            ev(Ev::Click, |_| Msg::Query(QueryMsg::OrderBy(
                LinkOverviewColumns::Statistics
            ))),
            t("statistics")
        ],
        th![]
    ]
}

fn view_link_table_filter_input<F: Fn(&str) -> String>(model: &Model, t: F) -> Node<Msg> {
    tr![
        C!["filters"],
        td![input![
            attrs! {
                At::Value => &model.formconfig.filter[LinkOverviewColumns::Code].sieve,
                At::Type => "search",
                At::Placeholder => t("search-placeholder")
            },
            input_ev(Ev::Input, |s| Msg::Query(QueryMsg::CodeFilterChanged(s))),
            el_ref(&model.inputs[LinkOverviewColumns::Code].filter_input),
        ]],
        td![input![
            attrs! {At::Value =>
            &model
                .formconfig.filter[LinkOverviewColumns::Description].sieve,
                At::Type => "search",
                At::Placeholder => t("search-placeholder")
            },
            input_ev(Ev::Input, |s| Msg::Query(
                QueryMsg::DescriptionFilterChanged(s)
            )),
            el_ref(&model.inputs[LinkOverviewColumns::Description].filter_input),
        ]],
        td![input![
            attrs! {At::Value =>
            &model
                .formconfig.filter[LinkOverviewColumns::Target].sieve,
                At::Type => "search",
                At::Placeholder => t("search-placeholder")
            },
            input_ev(Ev::Input, |s| Msg::Query(QueryMsg::TargetFilterChanged(s))),
            el_ref(&model.inputs[LinkOverviewColumns::Target].filter_input),
        ]],
        td![input![
            attrs! {At::Value =>
            &model
                .formconfig.filter[LinkOverviewColumns::Author].sieve,
                At::Type => "search",
                At::Placeholder => t("search-placeholder")
            },
            input_ev(Ev::Input, |s| Msg::Query(QueryMsg::AuthorFilterChanged(s))),
            el_ref(&model.inputs[LinkOverviewColumns::Author].filter_input),
        ]],
        td![],
        td![],
    ]
}

fn view_link(l: &FullLink) -> Node<Msg> {
    // Ugly hack
    let link = LinkDelta::from(l.clone());
    let link2 = LinkDelta::from(l.clone());
    let link3 = LinkDelta::from(l.clone());
    let link4 = LinkDelta::from(l.clone());
    let link5 = LinkDelta::from(l.clone());
    tr![
        {
            td![
                ev(Ev::Click, |_| Msg::Edit(EditMsg::EditSelected(link))),
                &l.link.code
            ]
        },
        {
            td![
                ev(Ev::Click, |_| Msg::Edit(EditMsg::EditSelected(link2))),
                &l.link.title
            ]
        },
        td![a![attrs![At::Href => &l.link.target], &l.link.target]],
        {
            td![
                ev(Ev::Click, |_| Msg::Edit(EditMsg::EditSelected(link3))),
                &l.user.username
            ]
        },
        {
            td![
                ev(Ev::Click, |_| Msg::Edit(EditMsg::EditSelected(link4))),
                &l.clicks.number
            ]
        },
        {
            td![img![
                ev(Ev::Click, |_| Msg::Edit(EditMsg::MayDeleteSelected(link5))),
                C!["trashicon"],
                attrs!(At::Src => "/static/trash.svg")
            ]]
        },
    ]
}

fn edit_or_create_link<F: Fn(&str) -> String>(l: &RefCell<LinkDelta>, t: F) -> Node<Msg> {
    let link = l.borrow();
    div![
        C!["editdialog", "center"],
        div![
            C!["closebutton"],
            a!["\u{d7}"],
            ev(Ev::Click, |_| Msg::ClearAll)
        ],
        h1![match &link.edit {
            EditMode::Edit => t("edit-link"),
            EditMode::Create => t("create-link"),
        }],
        table![
            tr![
                th![t("link-description")],
                td![input![
                    attrs! {
                        At::Value => &link.title,
                        At::Type => "text",
                        At::Placeholder => t("link-description")
                    },
                    input_ev(Ev::Input, |s| {
                        Msg::Edit(EditMsg::EditDescriptionChanged(s))
                    }),
                ]]
            ],
            tr![
                th![t("link-target")],
                td![input![
                    attrs! {
                        At::Value => &link.target,
                        At::Type => "text",
                        At::Placeholder => t("link-target")
                    },
                    input_ev(Ev::Input, |s| { Msg::Edit(EditMsg::EditTargetChanged(s)) }),
                ]]
            ],
            tr![
                th![t("link-code")],
                td![input![
                    attrs! {
                        At::Value => &link.code,
                        At::Type => "text",
                        At::Placeholder => t("password")
                    },
                    input_ev(Ev::Input, |s| { Msg::Edit(EditMsg::EditCodeChanged(s)) }),
                ],]
            ]
        ],
        a![
            match &link.edit {
                EditMode::Edit => t("edit-link"),
                EditMode::Create => t("create-link"),
            },
            C!["button"],
            ev(Ev::Click, |_| Msg::Edit(EditMsg::SaveLink))
        ]
    ]
}
