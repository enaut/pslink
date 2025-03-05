use dioxus::logger::tracing::info;
use dioxus::prelude::*;
use dioxus_i18n::t;
use indexmap::IndexMap;
use pslink_shared::apirequests::general::EditMode;

use pslink_shared::datatypes::User;

use crate::users::OptionUserEditDialog as _;
use crate::PslinkContext;

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
            .unwrap()
            .clone()
    });
    let PslinkContext { user } = use_context::<PslinkContext>();
    rsx! {
        tr {
            onclick: move |_| {
                info!("Edit user {:?}", user().unwrap().role);
                edit_dialog_signal
                    .set_edit_dialog(
                        Some(uu().id),
                        uu().username.clone(),
                        uu().email.clone(),
                        None,
                        uu().role,
                        EditMode::Edit,
                    );
            },

            td { "{uu().id}" }
            td { "{uu().username}" }
            td { "{uu().email}" }
            td {
                match uu().role {
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
