use dioxus::{logger::tracing::info, prelude::*};
use dioxus_i18n::t;
use indexmap::IndexMap;

use crate::{
    domain::{
        models::{Role, User},
        requests::EditMode,
    },
    ui::{PslinkContext, users::EditDialog},
};

#[component]
pub fn UserDisplay(
    current_username: String,
    users: Signal<IndexMap<String, User>>,
    mut edit_dialog_signal: Signal<Option<EditDialog>>,
) -> Element {
    let uu = use_memo(move || users().get(&current_username).cloned());
    let PslinkContext { user, .. } = use_context::<PslinkContext>();
    rsx! {
        if let Some(user_data) = uu() {
            tr {
                onclick: move |_| {
                    if let Some(current_user) = user() {
                        info!("Edit user {:?}", current_user.role);
                        edit_dialog_signal.set(Some(EditDialog::new(
                            Some(user_data.id),
                            user_data.username.clone(),
                            user_data.email.clone(),
                            None,
                            user_data.role,
                            EditMode::Edit,
                        )));
                    }
                },

                td { "{user_data.id}" }
                td { "{user_data.username}" }
                td { "{user_data.email}" }
                td {
                    match user_data.role {
                        Role::NotAuthenticated => t!("users-role-anonymous"),
                        Role::Disabled => t!("users-role-disabled"),
                        Role::Regular => t!("users-role-regular"),
                        Role::Admin => t!("users-role-admin"),
                    }
                }
            }
        }
    }
}
