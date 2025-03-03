mod new_user_button;
mod user_display;
mod user_edit;

use dioxus::{logger::tracing::info, prelude::*};

use indexmap::IndexMap;
use new_user_button::NewUserButton;
use pslink_shared::{
    apirequests::{
        general::{EditMode, Filter, Operation, Ordering},
        users::{Role, UserDelta, UserOverviewColumns, UserRequestForm},
    },
    datatypes::User,
};
use user_display::UserDisplay;
use user_edit::UserEdit;

use crate::{navbar::Route, PslinkContext};

const LISTS_CSS: Asset = asset!("/assets/styling/lists.css");

const SEARCH_SVG: Asset = asset!("/assets/search.svg");
const RELOAD_SVG: Asset = asset!("/assets/reload.svg");

fn toggle_column(
    ordering: Option<Operation<UserOverviewColumns, Ordering>>,
    new_column: UserOverviewColumns,
) -> Option<Operation<UserOverviewColumns, Ordering>> {
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
    user_delta: UserDelta,
}

trait OptionUserEditDialog {
    fn set_edit_dialog(
        &mut self,
        id: Option<i64>,
        username: String,
        email: String,
        password: Option<String>,
        role: Role,
        edit_mode: EditMode,
    );
    fn update_username(&mut self, username: String);
    fn update_email(&mut self, email: String);
    fn update_password(&mut self, password: Option<String>);
    fn update_role(&mut self, role: Role);
    fn set_edit_mode(&mut self, edit_mode: EditMode);
}

impl OptionUserEditDialog for Signal<Option<EditDialog>> {
    fn set_edit_dialog(
        &mut self,
        id: Option<i64>,
        username: String,
        email: String,
        password: Option<String>,
        role: Role,
        edit_mode: EditMode,
    ) {
        if let Some(mut dialog) = self() {
            dialog.user_delta = UserDelta {
                edit: edit_mode,
                id,
                username,
                email,
                password,
                role,
            };
            self.set(Some(dialog));
        } else {
            self.set(Some(EditDialog {
                user_delta: UserDelta {
                    edit: edit_mode,
                    id,
                    username,
                    email,
                    password,
                    role,
                },
            }))
        }
    }

    fn update_username(&mut self, username: String) {
        info!("Updating username to: {}", username);
        if let Some(mut dialog) = self() {
            dialog.user_delta.username = username;
            self.set(Some(dialog));
        }
    }

    fn update_email(&mut self, email: String) {
        info!("Updating email to: {}", email);
        if let Some(mut dialog) = self() {
            dialog.user_delta.email = email;
            self.set(Some(dialog));
        }
    }

    fn update_password(&mut self, password: Option<String>) {
        info!("Updating password");
        if let Some(mut dialog) = self() {
            dialog.user_delta.password = password;
            self.set(Some(dialog));
        }
    }

    fn update_role(&mut self, role: Role) {
        info!("Updating role to: {:?}", &role);
        if let Some(mut dialog) = self() {
            dialog.user_delta.role = role;
            self.set(Some(dialog));
        }
    }

    fn set_edit_mode(&mut self, edit_mode: EditMode) {
        info!("Updating edit to: {:?}", &edit_mode);
        if let Some(mut dialog) = self() {
            dialog.user_delta.edit = edit_mode;
            self.set(Some(dialog));
        }
    }
}

#[component]
pub fn Users() -> Element {
    let PslinkContext { user } = use_context::<PslinkContext>();
    let mut id_filter = use_signal(|| "".to_string());
    let mut email_filter = use_signal(|| "".to_string());
    let mut username_filter = use_signal(|| "".to_string());
    let mut order_by = use_signal(|| Option::<Operation<UserOverviewColumns, Ordering>>::None);
    let edit_dialog_signal = use_signal(|| None);
    let users = use_resource(move || async move {
        let mut form = UserRequestForm::default();
        form.filter[UserOverviewColumns::Id] = Filter { sieve: id_filter() };

        form.filter[UserOverviewColumns::Email] = Filter {
            sieve: email_filter(),
        };
        form.filter[UserOverviewColumns::Username] = Filter {
            sieve: username_filter(),
        };
        form.order = order_by();
        let res: IndexMap<String, User> = backend::user_api::list_users(form)
            .await
            .expect("No users loaded")
            .list
            .into_iter()
            .map(|item| (item.username.clone(), item))
            .collect();
        res
    });
    let user_codes = use_memo(move || {
        users()
            .as_ref()
            .map(|users| users.keys().cloned().collect::<Vec<String>>())
    });

    rsx! {
        document::Stylesheet { href: LISTS_CSS }
        if let Some(_user) = user.as_ref() {
            UserEdit { edit_dialog_signal, users }
            div {
                table { class: "table is-bordered is-striped is-hoverable is-fullwidth",
                    tbody {
                        tr {
                            th {
                                class: "headlines",
                                onclick: move |_| {
                                    order_by.set(toggle_column(order_by(), UserOverviewColumns::Id));
                                },
                                "User ID"
                            }
                            th {
                                class: "headlines",
                                onclick: move |_| {
                                    order_by.set(toggle_column(order_by(), UserOverviewColumns::Username));
                                },
                                "Username"
                            }
                            th {
                                class: "headlines",
                                onclick: move |_| {
                                    order_by.set(toggle_column(order_by(), UserOverviewColumns::Email));
                                },
                                "E-Mail"
                            }
                            th { class: "headlines", "Role" }
                        }
                        tr {
                            td {
                                div { class: "control has-icons-left has-icons-right is-small",
                                    input {
                                        r#type: "search",
                                        placeholder: "Filter according to...",
                                        value: "{id_filter}",
                                        class: "input is-small",
                                        oninput: move |e| {
                                            id_filter.set(e.value());
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
                                        value: "{username_filter}",
                                        r#type: "search",
                                        placeholder: "Filter according to...",
                                        class: "input is-small",
                                        oninput: move |e| {
                                            username_filter.set(e.value());
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
                                        value: "{email_filter}",
                                        placeholder: "Filter according to...",
                                        class: "input is-small",
                                        oninput: move |e| {
                                            email_filter.set(e.value());
                                        },
                                    }
                                    span { class: "icon is-small is-left",
                                        img { src: SEARCH_SVG }
                                    }
                                }
                            }
                            td {}
                        }
                        if users().is_some() {
                            for code in user_codes().expect("users not loaded") {
                                UserDisplay {
                                    key: code.clone(),
                                    current_username: code.clone(),
                                    users,
                                    edit_dialog_signal: edit_dialog_signal.clone(),
                                }
                            }
                        }
                    }
                }
                if user.as_ref().unwrap().role == Role::Admin {
                    NewUserButton { edit_dialog_signal }
                    a { class: "loadmore button",
                        img { src: RELOAD_SVG, class: "reloadicon" }
                        "load more users"
                    }
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
