use std::cell::RefCell;

use enum_map::EnumMap;
use seed::{
    a, attrs, button, div, h1, input, log, p, prelude::*, section, table, td, th, tr, Url, C, IF,
};
use shared::{
    apirequests::general::{Operation, Ordering},
    apirequests::{
        general::{EditMode, SuccessMessage},
        users::{UserDelta, UserOverviewColumns, UserRequestForm},
    },
    datatypes::User,
};

use crate::i18n::I18n;
#[must_use]
pub fn init(mut url: Url, orders: &mut impl Orders<Msg>, i18n: I18n) -> Model {
    orders.send_msg(Msg::Query(UserQueryMsg::Fetch));
    let user_edit = match url.next_path_part() {
        Some("create_user") => Some(RefCell::new(UserDelta::default())),
        None | Some(_) => None,
    };
    Model {
        users: Vec::new(),
        i18n,
        formconfig: UserRequestForm::default(),
        inputs: EnumMap::default(),
        user_edit,
        last_message: None,
    }
}
#[derive(Debug)]
pub struct Model {
    users: Vec<User>,
    i18n: I18n,
    formconfig: UserRequestForm,
    inputs: EnumMap<UserOverviewColumns, FilterInput>,
    user_edit: Option<RefCell<UserDelta>>,
    last_message: Option<SuccessMessage>,
}

#[derive(Default, Debug, Clone)]
struct FilterInput {
    filter_input: ElRef<web_sys::HtmlInputElement>,
}

#[derive(Clone)]
pub enum Msg {
    Query(UserQueryMsg),
    Edit(UserEditMsg),
}

/// All the messages on user Querying
#[derive(Clone)]
pub enum UserQueryMsg {
    Fetch,
    FailedToFetchUsers,
    OrderBy(UserOverviewColumns),
    Received(Vec<User>),
    IdFilterChanged(String),
    EmailFilterChanged(String),
    UsernameFilterChanged(String),
}
/// All the messages on user editing
#[derive(Clone)]
pub enum UserEditMsg {
    EditUserSelected(UserDelta),
    CreateNewUser,
    UserCreated(SuccessMessage),
    EditUsernameChanged(String),
    EditEmailChanged(String),
    EditPasswordChanged(String),
    SaveUser,
    FailedToCreateUser,
}

/// # Panics
/// Sould only panic on bugs.
pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::Query(msg) => process_query_messages(msg, model, orders),
        Msg::Edit(msg) => process_user_edit_messages(msg, model, orders),
    }
}

pub fn process_query_messages(msg: UserQueryMsg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        UserQueryMsg::Fetch => {
            orders.skip(); // No need to rerender
            let data = model.formconfig.clone(); // complicated way to move into the closure
            orders.perform_cmd(async {
                let data = data;
                let response = match fetch(
                    Request::new("/admin/json/list_users/")
                        .method(Method::Post)
                        .json(&data)
                        .expect("serialization failed"),
                )
                .await
                {
                    Ok(response) => response,
                    Err(_) => return Msg::Query(UserQueryMsg::FailedToFetchUsers),
                };

                let users: Vec<User> = response
                    .check_status() // ensure we've got 2xx status
                    .expect("status check failed")
                    .json()
                    .await
                    .expect("deserialization failed");

                Msg::Query(UserQueryMsg::Received(users))
            });
        }
        UserQueryMsg::OrderBy(column) => {
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
            orders.send_msg(Msg::Query(UserQueryMsg::Fetch));

            model.users.sort_by(match column {
                UserOverviewColumns::Id => |o: &User, t: &User| o.id.cmp(&t.id),
                UserOverviewColumns::Username => |o: &User, t: &User| o.username.cmp(&t.username),
                UserOverviewColumns::Email => |o: &User, t: &User| o.email.cmp(&t.email),
            })
        }
        UserQueryMsg::Received(response) => {
            model.users = response;
        }
        UserQueryMsg::IdFilterChanged(s) => {
            log!("Filter is: ", &s);
            let sanit = s.chars().filter(|x| x.is_numeric()).collect();
            model.formconfig.filter[UserOverviewColumns::Id].sieve = sanit;
            orders.send_msg(Msg::Query(UserQueryMsg::Fetch));
        }
        UserQueryMsg::UsernameFilterChanged(s) => {
            log!("Filter is: ", &s);
            let sanit = s.chars().filter(|x| x.is_alphanumeric()).collect();
            model.formconfig.filter[UserOverviewColumns::Username].sieve = sanit;
            orders.send_msg(Msg::Query(UserQueryMsg::Fetch));
        }
        UserQueryMsg::EmailFilterChanged(s) => {
            log!("Filter is: ", &s);
            // FIXME: Sanitazion does not work for @
            let sanit = s.chars().filter(|x| x.is_alphanumeric()).collect();
            model.formconfig.filter[UserOverviewColumns::Email].sieve = sanit;
            orders.send_msg(Msg::Query(UserQueryMsg::Fetch));
        }

        UserQueryMsg::FailedToFetchUsers => {
            log!("Failed to fetch users");
        }
    }
}
pub fn process_user_edit_messages(
    msg: UserEditMsg,
    model: &mut Model,
    orders: &mut impl Orders<Msg>,
) {
    match msg {
        UserEditMsg::EditUserSelected(user) => {
            log!("Editing user: ", user);
            model.user_edit = Some(RefCell::new(user))
        }
        UserEditMsg::CreateNewUser => {
            log!("Creating new user");
            model.user_edit = Some(RefCell::new(UserDelta::default()))
        }
        UserEditMsg::EditUsernameChanged(s) => {
            log!("New Username is: ", &s);
            if let Some(ref ue) = model.user_edit {
                ue.try_borrow_mut()
                    .expect("Failed to borrow mutably")
                    .username = s;
            };
        }
        UserEditMsg::EditEmailChanged(s) => {
            log!("New Email is: ", &s);
            if let Some(ref ue) = model.user_edit {
                ue.try_borrow_mut().expect("Failed to borrow mutably").email = s;
            };
        }
        UserEditMsg::EditPasswordChanged(s) => {
            log!("New Password is: ", &s);
            if let Some(ref ue) = model.user_edit {
                ue.try_borrow_mut()
                    .expect("Failed to borrow mutably")
                    .password = Some(s);
            };
        }
        UserEditMsg::SaveUser => {
            let data = model
                .user_edit
                .as_ref()
                .expect("Somehow a user should exist!")
                .borrow()
                .clone(); // complicated way to move into the closure
            log!("Saving User: ", &data.username);

            orders.perform_cmd(async {
                let data = data;
                let response = match fetch(
                    Request::new("/admin/json/create_user/")
                        .method(Method::Post)
                        .json(&data)
                        .expect("serialization failed"),
                )
                .await
                {
                    Ok(response) => response,
                    Err(_) => return Msg::Edit(UserEditMsg::FailedToCreateUser),
                };

                let message: SuccessMessage = response
                    .check_status() // ensure we've got 2xx status
                    .expect("status check failed")
                    .json()
                    .await
                    .expect("deserialization failed");

                Msg::Edit(UserEditMsg::UserCreated(message))
            });
        }
        UserEditMsg::FailedToCreateUser => {
            log!("Failed to create user");
        }
        UserEditMsg::UserCreated(u) => {
            log!(u, "created user");
            model.last_message = Some(u);
            model.user_edit = None;
            orders.send_msg(Msg::Query(UserQueryMsg::Fetch));
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
        if let Some(message) = &model.last_message {
            div![C!("Message"), &message.message]
        } else {
            section!()
        },
        table![
            // Column Headlines
            view_user_table_head(&t),
            // Add filter fields right below the headlines
            view_user_table_filter_input(model, &t),
            // Add all the users one line for each
            model.users.iter().map(view_user)
        ],
        button![
            ev(Ev::Click, |_| Msg::Query(UserQueryMsg::Fetch)),
            "Refresh"
        ],
        if let Some(l) = &model.user_edit {
            edit_or_create_user(l, t)
        } else {
            section!()
        },
    ]
}

fn view_user_table_head<F: Fn(&str) -> String>(t: F) -> Node<Msg> {
    tr![
        th![
            ev(Ev::Click, |_| Msg::Query(UserQueryMsg::OrderBy(
                UserOverviewColumns::Id
            ))),
            t("userid")
        ],
        th![
            ev(Ev::Click, |_| Msg::Query(UserQueryMsg::OrderBy(
                UserOverviewColumns::Email
            ))),
            t("email")
        ],
        th![
            ev(Ev::Click, |_| Msg::Query(UserQueryMsg::OrderBy(
                UserOverviewColumns::Username
            ))),
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
            input_ev(Ev::Input, |s| {
                Msg::Query(UserQueryMsg::IdFilterChanged(s))
            }),
            el_ref(&model.inputs[UserOverviewColumns::Id].filter_input),
        ]],
        td![input![
            attrs! {At::Value =>
            &model
                .formconfig.filter[UserOverviewColumns::Email].sieve,
                At::Type => "search",
                At::Placeholder => t("search-placeholder")
            },
            input_ev(Ev::Input, |s| {
                Msg::Query(UserQueryMsg::EmailFilterChanged(s))
            }),
            el_ref(&model.inputs[UserOverviewColumns::Email].filter_input),
        ]],
        td![input![
            attrs! {At::Value =>
            &model
                .formconfig.filter[UserOverviewColumns::Username].sieve,
                At::Type => "search",
                At::Placeholder => t("search-placeholder")
            },
            input_ev(Ev::Input, |s| {
                Msg::Query(UserQueryMsg::UsernameFilterChanged(s))
            }),
            el_ref(&model.inputs[UserOverviewColumns::Username].filter_input),
        ]],
    ]
}

fn view_user(l: &User) -> Node<Msg> {
    let user = UserDelta::from(l.clone());
    tr![
        ev(Ev::Click, |_| Msg::Edit(UserEditMsg::EditUserSelected(
            user
        ))),
        td![&l.id],
        td![&l.email],
        //td![a![attrs![At::Href => &l.link.target], &l.link.target]],
        td![&l.username],
    ]
}

fn edit_or_create_user<F: Fn(&str) -> String>(l: &RefCell<UserDelta>, t: F) -> Node<Msg> {
    let user = l.borrow();
    div![
        C!["editdialog", "center"],
        h1![match &user.edit {
            EditMode::Edit => t("edit-user"),
            EditMode::Create => t("new-user"),
        }],
        table![
            tr![
                th![
                    ev(Ev::Click, |_| Msg::Query(UserQueryMsg::OrderBy(
                        UserOverviewColumns::Username
                    ))),
                    t("username")
                ],
                td![input![
                    attrs! {
                        At::Value => &user.username,
                        At::Type => "text",
                        At::Placeholder => t("username")
                    },
                    input_ev(Ev::Input, |s| {
                        Msg::Edit(UserEditMsg::EditUsernameChanged(s))
                    }),
                ]]
            ],
            tr![
                th![
                    ev(Ev::Click, |_| Msg::Query(UserQueryMsg::OrderBy(
                        UserOverviewColumns::Email
                    ))),
                    t("email")
                ],
                td![input![
                    attrs! {
                        At::Value => &user.email,
                        At::Type => "email",
                        At::Placeholder => t("email")
                    },
                    input_ev(Ev::Input, |s| {
                        Msg::Edit(UserEditMsg::EditEmailChanged(s))
                    }),
                ]]
            ],
            tr![
                th![
                    ev(Ev::Click, |_| Msg::Query(UserQueryMsg::OrderBy(
                        UserOverviewColumns::Email
                    ))),
                    t("password")
                ],
                td![
                    input![
                        attrs! {
                            At::Type => "password",
                            At::Placeholder => t("password")
                        },
                        input_ev(Ev::Input, |s| {
                            Msg::Edit(UserEditMsg::EditPasswordChanged(s))
                        }),
                    ],
                    IF!(user.edit == EditMode::Edit => p![t("leave-password-empty-hint")])
                ]
            ]
        ],
        a![
            match &user.edit {
                EditMode::Edit => t("edit-user"),
                EditMode::Create => t("create-user"),
            },
            C!["button"],
            ev(Ev::Click, |_| Msg::Edit(UserEditMsg::SaveUser))
        ]
    ]
}
