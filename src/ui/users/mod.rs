mod database_export;
mod new_user_button;
mod user_display;
mod user_edit;

use dioxus::{logger::tracing::info, prelude::*};
use dioxus_i18n::t;
use indexmap::IndexMap;

use database_export::DatabaseExport;
use new_user_button::NewUserButton;
use user_display::UserDisplay;
use user_edit::UserEdit;

use crate::{
    domain::{
        models::{Role, User},
        requests::{
            EditMode, Filter, Operation, Ordering, UserDelta, UserOverviewColumns, UserRequestForm,
        },
    },
    ui::{PslinkContext, navbar::Route, toggle_column},
};

const LISTS_CSS: Asset = asset!("/assets/styling/lists.css");

const SEARCH_SVG: Asset = asset!("/assets/search.svg");
const RELOAD_SVG: Asset = asset!("/assets/reload.svg");

#[derive(Clone, Debug)]
pub struct EditDialog {
    pub user_delta: UserDelta,
    pub error: Option<String>,
}

impl EditDialog {
    #[must_use]
    pub fn new(
        id: Option<i64>,
        username: String,
        email: String,
        password: Option<String>,
        role: Role,
        edit: EditMode,
    ) -> Self {
        Self {
            user_delta: UserDelta {
                edit,
                id,
                username,
                email,
                password,
                role,
            },
            error: None,
        }
    }
}

#[component]
pub fn Users() -> Element {
    let PslinkContext { user, .. } = use_context::<PslinkContext>();
    let mut id_filter = use_signal(String::new);
    let mut email_filter = use_signal(String::new);
    let mut username_filter = use_signal(String::new);
    let mut order_by = use_signal(|| Option::<Operation<UserOverviewColumns, Ordering>>::None);
    let reload_trigger = use_signal(|| 0u32);
    let edit_dialog_signal = use_signal(|| Option::<EditDialog>::None);
    let mut users: Signal<IndexMap<String, User>> = use_signal(IndexMap::new);

    let _update_users = use_resource(move || async move {
        let _trigger = reload_trigger();
        let loaded = load_users(
            0,
            20,
            id_filter(),
            email_filter(),
            username_filter(),
            order_by(),
        )
        .await;
        users.set(loaded);
    });

    let user_codes = use_memo(move || users().keys().cloned().collect::<Vec<String>>());

    rsx! {
        div { class: "users-view",
            document::Stylesheet { href: LISTS_CSS }
            if user().is_some() {
                UserEdit { edit_dialog_signal, reload_trigger }
                div {
                    input {
                        name: "prevent_autofill",
                        r#type: "text",
                        style: "display:none;",
                        value: "",
                        id: "prevent_autofill",
                    }
                    table { class: "table is-bordered is-striped is-hoverable is-fullwidth",
                        tbody {
                            tr {
                                th {
                                    class: "headlines",
                                    onclick: move |_| {
                                        order_by.set(toggle_column(order_by(), UserOverviewColumns::Id));
                                    },
                                    {t!("users-table-header-user-id")}
                                }
                                th {
                                    class: "headlines",
                                    onclick: move |_| {
                                        order_by.set(toggle_column(order_by(), UserOverviewColumns::Username));
                                    },
                                    {t!("users-table-header-username")}
                                }
                                th {
                                    class: "headlines",
                                    onclick: move |_| {
                                        order_by.set(toggle_column(order_by(), UserOverviewColumns::Email));
                                    },
                                    {t!("users-table-header-email")}
                                }
                                th { class: "headlines", {t!("users-table-header-role")} }
                            }
                            tr {
                                td {
                                    input {
                                        r#type: "username",
                                        value: "",
                                        style: "display:none;",
                                        name: "username_fake",
                                        id: "username_fake",
                                    }
                                    div { class: "control has-icons-left has-icons-right is-small",
                                        input {
                                            r#type: "search",
                                            placeholder: t!("users-table-filter-placeholder"),
                                            value: "{id_filter}",
                                            class: "input is-small",
                                            autocomplete: "off",
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
                                    input {
                                        r#type: "password",
                                        value: "",
                                        style: "display:none;",
                                        name: "password_fake",
                                        id: "password_fake",
                                    }
                                    div { class: "control has-icons-left has-icons-right is-small",
                                        input {
                                            value: "{username_filter}",
                                            r#type: "search",
                                            placeholder: t!("users-table-filter-placeholder"),
                                            class: "input is-small",
                                            autocomplete: "off",
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
                                    input {
                                        r#type: "email",
                                        value: "",
                                        style: "display:none;",
                                        name: "email_fake",
                                        id: "email_fake",
                                    }
                                    div { class: "control has-icons-left has-icons-right is-small",
                                        input {
                                            r#type: "search",
                                            value: "{email_filter}",
                                            placeholder: t!("users-table-filter-placeholder"),
                                            class: "input is-small",
                                            autocomplete: "off",
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
                            if !users().is_empty() {
                                for code in user_codes() {
                                    UserDisplay {
                                        key: "{code}",
                                        current_username: code.clone(),
                                        users,
                                        edit_dialog_signal,
                                    }
                                }
                            }
                        }
                    }
                    if let Some(current_user) = user() {
                        if current_user.role == Role::Admin {
                            NewUserButton { edit_dialog_signal }
                            a {
                                class: "loadmore button",
                                onclick: move |_| async move {
                                    let more = load_users(
                                        users().len(),
                                        20,
                                        id_filter(),
                                        email_filter(),
                                        username_filter(),
                                        order_by(),
                                    )
                                    .await;
                                    if !more.is_empty() {
                                        users.write().extend(more);
                                    }
                                },
                                img { src: RELOAD_SVG, class: "reloadicon" }
                                {t!("users-button-load-more")}
                            }
                            hr {}
                            DatabaseExport {}
                        }
                    }
                }
            } else {
                div { class: "centered",
                    div { class: "boxed",
                        div { {t!("users-loading")} }
                        Link { to: Route::LoginScreen {}, {t!("users-login")} }
                    }
                }
            }
        }
    }
}

async fn load_users(
    offset: usize,
    amount: usize,
    id_filter: String,
    email_filter: String,
    username_filter: String,
    order: Option<Operation<UserOverviewColumns, Ordering>>,
) -> IndexMap<String, User> {
    let mut form = UserRequestForm::default();
    form.filter[UserOverviewColumns::Id] = Filter { sieve: id_filter };
    form.filter[UserOverviewColumns::Email] = Filter {
        sieve: email_filter,
    };
    form.filter[UserOverviewColumns::Username] = Filter {
        sieve: username_filter,
    };
    form.order = order;
    form.offset = offset;
    form.amount = amount;

    match crate::api::user_api::list_users(form).await {
        Ok(list) => list
            .into_iter()
            .map(|item| (item.username.clone(), item))
            .collect(),
        Err(e) => {
            info!("Failed to load users: {}", e);
            IndexMap::new()
        }
    }
}
