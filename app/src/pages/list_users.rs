//! List all users in case an admin views it, list the "self" user otherwise.

use crate::{unwrap_or_return, I18n};
use enum_map::EnumMap;
use gloo_console::log;
use gloo_net::http::Request;
use pslink_shared::{
    apirequests::{
        general::{EditMode, Message, Operation, Ordering, Status},
        users::{Role, UserDelta, UserOverviewColumns, UserRequestForm},
    },
    datatypes::{Lang, User},
};
use seed::{a, attrs, div, h1, input, p, prelude::*, section, table, td, th, tr, Url, C, IF};
/*
 * init
 */
#[must_use]
pub fn init(mut url: Url, orders: &mut impl Orders<Msg>, i18n: I18n) -> Model {
    orders.send_msg(Msg::Query(UserQueryMsg::Fetch));
    let user_edit = match url.next_path_part() {
        Some("create_user") => Some(UserDelta::default()),
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
    user_edit: Option<UserDelta>,
    last_message: Option<Status<Message>>,
}

impl Model {
    /// set the language of this page (part)
    pub fn set_lang(&mut self, l: Lang) {
        self.i18n.set_lang(l);
    }
}

impl Model {
    /// removing all open dialogs (often to open another afterwards).
    fn clean_dialogs(&mut self) {
        self.last_message = None;
        self.user_edit = None;
    }
}

/// A type containing one input field for later use.
#[derive(Default, Debug, Clone)]
struct FilterInput {
    filter_input: ElRef<web_sys::HtmlInputElement>,
}

/// The message splits the contained message into messages related to querying and messages related to editing.
#[derive(Clone)]
pub enum Msg {
    Query(UserQueryMsg),
    Edit(UserEditMsg),
    ClearAll,
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
    UserCreated(Status<Message>),
    EditUsernameChanged(String),
    EditEmailChanged(String),
    EditPasswordChanged(String),
    MakeAdmin(UserDelta),
    MakeRegular(UserDelta),
    SaveUser,
    FailedToCreateUser,
}
/*
 * update
 */

/// Split the update to Query updates and Edit updates
pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::Query(msg) => process_query_messages(msg, model, orders),
        Msg::Edit(msg) => process_user_edit_messages(msg, model, orders),
        Msg::ClearAll => {
            model.clean_dialogs();
        }
    }
}

/// Process all updates related to getting data from the server.
pub fn process_query_messages(msg: UserQueryMsg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        UserQueryMsg::Fetch => {
            orders.skip(); // No need to rerender only after the data is fetched the page has to be rerendered.
            load_users(model.formconfig.clone(), orders);
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
            });
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
            // FIXME: Sanitation does not work for @
            let sanit = s.chars().filter(|x| x.is_alphanumeric()).collect();
            model.formconfig.filter[UserOverviewColumns::Email].sieve = sanit;
            orders.send_msg(Msg::Query(UserQueryMsg::Fetch));
        }

        UserQueryMsg::FailedToFetchUsers => {
            log!("Failed to fetch users");
        }
    }
}

/// Load the list of users from the server.
fn load_users(data: UserRequestForm, orders: &mut impl Orders<Msg>) {
    orders.perform_cmd(async {
        let data = data;
        // create the request
        let request = unwrap_or_return!(
            Request::post("/admin/json/list_users/").json(&data),
            Msg::Query(UserQueryMsg::FailedToFetchUsers)
        )
        .send();
        // request and get response
        let response =
            unwrap_or_return!(request.await, Msg::Query(UserQueryMsg::FailedToFetchUsers));
        // check the response status
        if !response.ok() {
            Msg::Query(UserQueryMsg::FailedToFetchUsers)
        } else {
            // deserialize the users list
            let users: Vec<User> = unwrap_or_return!(
                response.json().await,
                Msg::Query(UserQueryMsg::FailedToFetchUsers)
            );

            Msg::Query(UserQueryMsg::Received(users))
        }
    });
}

/// Process all the messages related to editing users.
pub fn process_user_edit_messages(
    msg: UserEditMsg,
    model: &mut Model,
    orders: &mut impl Orders<Msg>,
) {
    match msg {
        UserEditMsg::EditUserSelected(user) => {
            model.clean_dialogs();
            model.user_edit = Some(user);
        }
        UserEditMsg::CreateNewUser => {
            model.clean_dialogs();
            model.user_edit = Some(UserDelta::default());
        }
        UserEditMsg::EditUsernameChanged(s) => {
            if let Some(ref mut ue) = model.user_edit {
                ue.username = s;
            };
        }
        UserEditMsg::EditEmailChanged(s) => {
            if let Some(ref mut ue) = model.user_edit {
                ue.email = s;
            };
        }
        UserEditMsg::EditPasswordChanged(s) => {
            if let Some(ref mut ue) = model.user_edit {
                ue.password = Some(s);
            };
        }
        UserEditMsg::SaveUser => {
            let data = model
                .user_edit
                .take()
                .expect("A user should always be there on save");
            log!("Saving User: ", &data.username);
            save_user(data, orders);
        }
        UserEditMsg::FailedToCreateUser => {
            log!("Failed to create user");
        }
        UserEditMsg::UserCreated(u) => {
            log!(format!("created user {:?}", u));
            model.last_message = Some(u);
            model.user_edit = None;
            orders.send_msg(Msg::Query(UserQueryMsg::Fetch));
        }
        UserEditMsg::MakeAdmin(user) | UserEditMsg::MakeRegular(user) => {
            update_privileges(user, orders);
        }
    }
}

/// Update the role of a user - this toggles between admin and regular.
fn update_privileges(user: UserDelta, orders: &mut impl Orders<Msg>) {
    orders.perform_cmd(async {
        let data = user;
        // create the request
        let request = unwrap_or_return!(
            Request::post("/admin/json/update_privileges/").json(&data),
            Msg::Edit(UserEditMsg::FailedToCreateUser)
        );
        // perform the request and get the response
        let response = unwrap_or_return!(
            request.send().await,
            Msg::Edit(UserEditMsg::FailedToCreateUser)
        );
        // check for the status
        if !response.ok() {
            Msg::Edit(UserEditMsg::FailedToCreateUser)
        } else {
            // deserialize the response
            let message: Status<Message> = unwrap_or_return!(
                response.json().await,
                Msg::Edit(UserEditMsg::FailedToCreateUser)
            );

            Msg::Edit(UserEditMsg::UserCreated(message))
        }
    });
}

/// Save a new user or edit an existing user
fn save_user(user: UserDelta, orders: &mut impl Orders<Msg>) {
    orders.perform_cmd(async {
        let data = user;
        let request = unwrap_or_return!(
            Request::post(match data.edit {
                EditMode::Create => "/admin/json/create_user/",
                EditMode::Edit => "/admin/json/update_user/",
            })
            .json(&data),
            Msg::Edit(UserEditMsg::FailedToCreateUser)
        )
        .send();
        let response = unwrap_or_return!(request.await, Msg::Edit(UserEditMsg::FailedToCreateUser));
        // check for the status
        if !response.ok() {
            Msg::Edit(UserEditMsg::FailedToCreateUser)
        } else {
            // deserialize the response
            let message: Status<Message> = unwrap_or_return!(
                response.json().await,
                Msg::Edit(UserEditMsg::FailedToCreateUser)
            );

            Msg::Edit(UserEditMsg::UserCreated(message))
        }
    });
}

#[must_use]
/// View the users page.
pub fn view(model: &Model, logged_in_user: &User) -> Node<Msg> {
    let lang = model.i18n.clone();
    // shortcut for easier translations
    let t = move |key: &str| lang.translate(key, None);
    section![
        // Clear all dialogs on press of the ESC button.
        keyboard_ev(Ev::KeyDown, |keyboard_event| {
            IF!(keyboard_event.key() == "Escape" => Msg::ClearAll)
        }),
        // display the messages to the user
        if let Some(message) = &model.last_message {
            div![
                C!["message", "center"],
                close_button(),
                match message {
                    Status::Success(m) | Status::Error(m) => {
                        &m.message
                    }
                }
            ]
        } else {
            section![]
        },
        // display the table with users
        table![
            // Column Headlines
            view_user_table_head(&t),
            // Add filter fields right below the headlines
            view_user_table_filter_input(model, &t),
            // Add all the users one line for each
            model
                .users
                .iter()
                .map(|u| { view_user(u, logged_in_user, &t) })
        ],
        // Display the user edit dialog if available
        if let Some(l) = &model.user_edit {
            edit_or_create_user(l.clone(), t)
        } else {
            section!()
        },
    ]
}

/// View the headlines of the table
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
        th![t("role")],
    ]
}

/// Display the filterboxes below the headlines
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
        td![],
    ]
}

/// Display one user-line of the table
fn view_user<F: Fn(&str) -> String>(l: &User, logged_in_user: &User, t: F) -> Node<Msg> {
    let user = UserDelta::from(l.clone());
    tr![
        {
            let user = user.clone();
            ev(Ev::Click, |_| {
                Msg::Edit(UserEditMsg::EditUserSelected(user))
            })
        },
        match l.role {
            Role::NotAuthenticated | Role::Disabled => C!("inactive"),
            Role::Regular => C!("regular"),
            Role::Admin => C!("admin"),
        },
        td![&l.id],
        td![&l.email],
        td![&l.username],
        match logged_in_user.role {
            Role::Admin => {
                match l.role {
                    Role::NotAuthenticated | Role::Disabled | Role::Regular => td![
                        ev(Ev::Click, |event| {
                            event.stop_propagation();
                            Msg::Edit(UserEditMsg::MakeAdmin(user))
                        }),
                        t("make-user-admin")
                    ],
                    Role::Admin => td![
                        ev(Ev::Click, |event| {
                            event.stop_propagation();
                            Msg::Edit(UserEditMsg::MakeRegular(user))
                        }),
                        t("make-user-regular"),
                    ],
                }
            }
            Role::Regular => match l.role {
                Role::NotAuthenticated | Role::Disabled | Role::Regular => td![t("user")],
                Role::Admin => td![t("admin")],
            },
            Role::NotAuthenticated | Role::Disabled => td![],
        }
    ]
}

/// display the edit and create dialog
fn edit_or_create_user<F: Fn(&str) -> String>(l: UserDelta, t: F) -> Node<Msg> {
    let user = l;
    let headline: Node<Msg> = match &user.role {
        Role::NotAuthenticated | Role::Disabled | Role::Regular => {
            h1![match &user.edit {
                EditMode::Edit => t("edit-user"),
                EditMode::Create => t("new-user"),
            }]
        }
        Role::Admin => {
            h1![match &user.edit {
                EditMode::Edit => t("edit-admin"),
                EditMode::Create => t("new-admin"),
            }]
        }
    };
    div![
        C!["editdialog", "center"],
        close_button(),
        headline,
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

/// a close button for dialogs
fn close_button() -> Node<Msg> {
    div![
        C!["closebutton"],
        a!["\u{d7}"],
        ev(Ev::Click, |_| Msg::ClearAll)
    ]
}
