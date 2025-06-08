use dioxus::logger::tracing::info;
use dioxus::prelude::*;
use dioxus_i18n::t;
use indexmap::IndexMap;
use pslink_shared::apirequests::general::EditMode;

use pslink_shared::datatypes::User;

use crate::PslinkContext;
use crate::users::OptionUserEditDialog as _;

use super::EditDialog;

#[component]
pub fn UserDisplay(
    current_username: String,
    users: Resource<IndexMap<String, User>>,
    edit_dialog_signal: Signal<Option<EditDialog>>,
) -> Element {
    let uu = use_memo(move || {
        users
            .as_ref()
            .expect("Users loaded")
            .get(&current_username)
            .cloned()
    });
    let PslinkContext { user, .. } = use_context::<PslinkContext>();
    rsx! {
        if let Some(user_data) = uu() {
            tr {
                onclick: move |_| {
                    if let Some(current_user) = user() {
                        info!("Edit user {:?}", current_user.role);
                        edit_dialog_signal
                            .set_edit_dialog(
                                Some(user_data.id),
                                user_data.username.clone(),
                                user_data.email.clone(),
                                None,
                                user_data.role,
                                EditMode::Edit,
                            );
                    }
                },

                td { "{user_data.id}" }
                td { "{user_data.username}" }
                td { "{user_data.email}" }
                td {
                    match user_data.role {
                        pslink_shared::apirequests::users::Role::NotAuthenticated => {
                            t!("users-role-anonymous")
                        }
                        pslink_shared::apirequests::users::Role::Disabled => t!("users-role-disabled"),
                        pslink_shared::apirequests::users::Role::Regular => t!("users-role-regular"),
                        pslink_shared::apirequests::users::Role::Admin => t!("users-role-admin"),
                    }
                }
            }
        }
    }
}
