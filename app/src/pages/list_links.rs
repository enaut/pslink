use enum_map::EnumMap;
use seed::{a, attrs, button, h1, input, log, prelude::*, section, table, td, th, tr, Url, C};

use shared::{
    apirequests::general::Ordering,
    apirequests::{
        general::Operation,
        links::{LinkOverviewColumns, LinkRequestForm},
    },
    datatypes::FullLink,
};

use crate::i18n::I18n;

pub fn init(_: Url, orders: &mut impl Orders<Msg>, i18n: I18n) -> Model {
    orders.send_msg(Msg::Fetch);

    Model {
        links: Vec::new(),
        i18n,
        formconfig: LinkRequestForm::default(),
        inputs: EnumMap::default(),
    }
}

#[derive(Debug)]
pub struct Model {
    links: Vec<FullLink>,
    i18n: I18n,
    formconfig: LinkRequestForm,
    inputs: EnumMap<LinkOverviewColumns, FilterInput>,
}

#[derive(Default, Debug, Clone)]
struct FilterInput {
    filter_input: ElRef<web_sys::HtmlInputElement>,
}

#[derive(Clone)]
pub enum Msg {
    Fetch,
    OrderBy(LinkOverviewColumns),
    Received(Vec<FullLink>),
    CodeFilterChanged(String),
    DescriptionFilterChanged(String),
    TargetFilterChanged(String),
    AuthorFilterChanged(String),
}

/// # Panics
/// Sould only panic on bugs.
pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::Fetch => {
            orders.skip(); // No need to rerender
            let data = model.formconfig.clone(); // complicated way to move into the closure
            orders.perform_cmd(async {
                let data = data;
                let response = fetch(
                    Request::new("/admin/json/list_links/")
                        .method(Method::Post)
                        .json(&data)
                        .expect("serialization failed"),
                )
                .await
                .expect("HTTP request failed");

                let user: Vec<FullLink> = response
                    .check_status() // ensure we've got 2xx status
                    .expect("status check failed")
                    .json()
                    .await
                    .expect("deserialization failed");

                Msg::Received(user)
            });
        }
        Msg::OrderBy(column) => {
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
            orders.send_msg(Msg::Fetch);

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
        Msg::Received(response) => {
            model.links = response;
        }
        Msg::CodeFilterChanged(s) => {
            log!("Filter is: ", &s);
            let sanit = s.chars().filter(|x| x.is_alphanumeric()).collect();
            model.formconfig.filter[LinkOverviewColumns::Code].sieve = sanit;
            orders.send_msg(Msg::Fetch);
        }
        Msg::DescriptionFilterChanged(s) => {
            log!("Filter is: ", &s);
            let sanit = s.chars().filter(|x| x.is_alphanumeric()).collect();
            model.formconfig.filter[LinkOverviewColumns::Description].sieve = sanit;
            orders.send_msg(Msg::Fetch);
        }
        Msg::TargetFilterChanged(s) => {
            log!("Filter is: ", &s);
            let sanit = s.chars().filter(|x| x.is_alphanumeric()).collect();
            model.formconfig.filter[LinkOverviewColumns::Target].sieve = sanit;
            orders.send_msg(Msg::Fetch);
        }
        Msg::AuthorFilterChanged(s) => {
            log!("Filter is: ", &s);
            let sanit = s.chars().filter(|x| x.is_alphanumeric()).collect();
            model.formconfig.filter[LinkOverviewColumns::Author].sieve = sanit;
            orders.send_msg(Msg::Fetch);
        }
    }
}

#[must_use]
/// # Panics
/// Sould only panic on bugs.
pub fn view(model: &Model) -> Node<Msg> {
    macro_rules! t {
        { $key:expr } => {
            {
                model.i18n.translate($key, None)
            }
        };
        { $key:expr, $args:expr } => {
            {
                model.i18n.translate($key, Some(&$args))
            }
        };
    }
    section![
        h1!("List Links Page from list_links"),
        table![
            tr![
                th![
                    ev(Ev::Click, |_| Msg::OrderBy(LinkOverviewColumns::Code)),
                    t!("link-code")
                ],
                th![
                    ev(Ev::Click, |_| Msg::OrderBy(
                        LinkOverviewColumns::Description
                    )),
                    t!("link-description")
                ],
                th![
                    ev(Ev::Click, |_| Msg::OrderBy(LinkOverviewColumns::Target)),
                    t!("link-target")
                ],
                th![
                    ev(Ev::Click, |_| Msg::OrderBy(LinkOverviewColumns::Author)),
                    t!("username")
                ],
                th![
                    ev(Ev::Click, |_| Msg::OrderBy(LinkOverviewColumns::Statistics)),
                    t!("statistics")
                ]
            ],
            tr![
                C!["filters"],
                td![input![
                    attrs! {
                        At::Value => &model.formconfig.filter[LinkOverviewColumns::Code].sieve,
                        At::Type => "search",
                        At::Placeholder => t!("search-placeholder")
                    },
                    input_ev(Ev::Input, Msg::CodeFilterChanged),
                    el_ref(&model.inputs[LinkOverviewColumns::Code].filter_input),
                ]],
                td![input![
                    attrs! {At::Value =>
                    &model
                        .formconfig.filter[LinkOverviewColumns::Description].sieve,
                        At::Type => "search",
                        At::Placeholder => t!("search-placeholder")
                    },
                    input_ev(Ev::Input, Msg::DescriptionFilterChanged),
                    el_ref(&model.inputs[LinkOverviewColumns::Description].filter_input),
                ]],
                td![input![
                    attrs! {At::Value =>
                    &model
                        .formconfig.filter[LinkOverviewColumns::Target].sieve,
                        At::Type => "search",
                        At::Placeholder => t!("search-placeholder")
                    },
                    input_ev(Ev::Input, Msg::TargetFilterChanged),
                    el_ref(&model.inputs[LinkOverviewColumns::Target].filter_input),
                ]],
                td![input![
                    attrs! {At::Value =>
                    &model
                        .formconfig.filter[LinkOverviewColumns::Author].sieve,
                        At::Type => "search",
                        At::Placeholder => t!("search-placeholder")
                    },
                    input_ev(Ev::Input, Msg::AuthorFilterChanged),
                    el_ref(&model.inputs[LinkOverviewColumns::Author].filter_input),
                ]],
                td![]
            ],
            model.links.iter().map(view_link)
        ],
        button![ev(Ev::Click, |_| Msg::Fetch), "Fetch links"]
    ]
}

fn view_link(l: &FullLink) -> Node<Msg> {
    tr![
        td![&l.link.code],
        td![&l.link.title],
        td![a![attrs![At::Href => &l.link.target], &l.link.target]],
        td![&l.user.username],
        td![&l.clicks.number]
    ]
}
