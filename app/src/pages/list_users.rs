use enum_map::EnumMap;
use seed::{attrs, button, h1, input, log, prelude::*, section, table, td, th, tr, Url, C};
use shared::{
    apirequests::general::{Operation, Ordering},
    apirequests::users::{UserOverviewColumns, UserRequestForm},
    datatypes::User,
};

use crate::i18n::I18n;
#[must_use]
pub fn init(_: Url, orders: &mut impl Orders<Msg>, i18n: I18n) -> Model {
    orders.send_msg(Msg::Fetch);
    Model {
        users: Vec::new(),
        i18n,
        formconfig: UserRequestForm::default(),
        inputs: EnumMap::default(),
    }
}
#[derive(Debug)]
pub struct Model {
    users: Vec<User>,
    i18n: I18n,
    formconfig: UserRequestForm,
    inputs: EnumMap<UserOverviewColumns, FilterInput>,
}

#[derive(Default, Debug, Clone)]
struct FilterInput {
    filter_input: ElRef<web_sys::HtmlInputElement>,
}

#[derive(Clone)]
pub enum Msg {
    Fetch,
    OrderBy(UserOverviewColumns),
    Received(Vec<User>),
    IdFilterChanged(String),
    EmailFilterChanged(String),
    UsernameFilterChanged(String),
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
                    Request::new("/admin/json/list_users/")
                        .method(Method::Post)
                        .json(&data)
                        .expect("serialization failed"),
                )
                .await
                .expect("HTTP request failed");

                let users: Vec<User> = response
                    .check_status() // ensure we've got 2xx status
                    .expect("status check failed")
                    .json()
                    .await
                    .expect("deserialization failed");

                Msg::Received(users)
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

            model.users.sort_by(match column {
                UserOverviewColumns::Id => |o: &User, t: &User| o.id.cmp(&t.id),
                UserOverviewColumns::Username => |o: &User, t: &User| o.username.cmp(&t.username),
                UserOverviewColumns::Email => |o: &User, t: &User| o.email.cmp(&t.email),
            })
        }
        Msg::Received(response) => {
            model.users = response;
        }
        Msg::IdFilterChanged(s) => {
            log!("Filter is: ", &s);
            let sanit = s.chars().filter(|x| x.is_numeric()).collect();
            model.formconfig.filter[UserOverviewColumns::Id].sieve = sanit;
            orders.send_msg(Msg::Fetch);
        }
        Msg::UsernameFilterChanged(s) => {
            log!("Filter is: ", &s);
            let sanit = s.chars().filter(|x| x.is_alphanumeric()).collect();
            model.formconfig.filter[UserOverviewColumns::Username].sieve = sanit;
            orders.send_msg(Msg::Fetch);
        }
        Msg::EmailFilterChanged(s) => {
            log!("Filter is: ", &s);
            // FIXME: Sanitazion does not work for @
            let sanit = s.chars().filter(|x| x.is_alphanumeric()).collect();
            model.formconfig.filter[UserOverviewColumns::Email].sieve = sanit;
            orders.send_msg(Msg::Fetch);
        }
    }
}
#[must_use]
/// # Panics
/// Sould only panic on bugs.
pub fn view(model: &Model) -> Node<Msg> {
    let lang = model.i18n.clone();
    let t = move |key: &str| lang.translate(key, None);
    section![
        h1!("List Users Page from list_users"),
        table![
            // Column Headlines
            view_user_table_head(&t),
            // Add filter fields right below the headlines
            view_user_table_filter_input(model, &t),
            // Add all the users one line for each
            model.users.iter().map(view_user)
        ],
        button![ev(Ev::Click, |_| Msg::Fetch), "Refresh"]
    ]
}

fn view_user_table_head<F: Fn(&str) -> String>(t: F) -> Node<Msg> {
    tr![
        th![
            ev(Ev::Click, |_| Msg::OrderBy(UserOverviewColumns::Id)),
            t("userid")
        ],
        th![
            ev(Ev::Click, |_| Msg::OrderBy(UserOverviewColumns::Email)),
            t("email")
        ],
        th![
            ev(Ev::Click, |_| Msg::OrderBy(UserOverviewColumns::Username)),
            t("username")
        ],
    ]
}

fn view_user_table_filter_input<F: Fn(&str) -> String>(model: &Model, t: F) -> Node<Msg> {
    tr![
        C!["filters"],
        td![input![
            attrs! {
                At::Value => &model.formconfig.filter[UserOverviewColumns::Id].sieve,
                At::Type => "search",
                At::Placeholder => t("search-placeholder")
            },
            input_ev(Ev::Input, Msg::IdFilterChanged),
            el_ref(&model.inputs[UserOverviewColumns::Id].filter_input),
        ]],
        td![input![
            attrs! {At::Value =>
            &model
                .formconfig.filter[UserOverviewColumns::Email].sieve,
                At::Type => "search",
                At::Placeholder => t("search-placeholder")
            },
            input_ev(Ev::Input, Msg::EmailFilterChanged),
            el_ref(&model.inputs[UserOverviewColumns::Email].filter_input),
        ]],
        td![input![
            attrs! {At::Value =>
            &model
                .formconfig.filter[UserOverviewColumns::Username].sieve,
                At::Type => "search",
                At::Placeholder => t("search-placeholder")
            },
            input_ev(Ev::Input, Msg::UsernameFilterChanged),
            el_ref(&model.inputs[UserOverviewColumns::Username].filter_input),
        ]],
    ]
}

fn view_user(l: &User) -> Node<Msg> {
    tr![
        td![&l.id],
        td![&l.email],
        //td![a![attrs![At::Href => &l.link.target], &l.link.target]],
        td![&l.username],
    ]
}
