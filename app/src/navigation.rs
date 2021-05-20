use fluent::fluent_args;
use seed::{a, attrs, div, li, nav, ol, prelude::*, Url};
use shared::datatypes::User;

use crate::{i18n::I18n, Msg};
#[must_use]
pub fn navigation(i18n: &I18n, base_url: &Url, user: &Option<User>) -> Node<Msg> {
    let t = move |key: &str| i18n.translate(key, None);
    let welcome = if let Some(user) = user {
        i18n.translate(
            "welcome-user",
            Some(&fluent_args![ "username" => user.username.clone()]),
        )
    } else {
        t("welcome")
    };
    nav![
        ol![
            li![a![
                attrs! {At::Href => crate::Urls::new(base_url).list_links()},
                t("list-links"),
            ],],
            li![a![
                attrs! {At::Href => crate::Urls::new(base_url).create_link()},
                ev(Ev::Click, |_| Msg::ListLinks(
                    super::pages::list_links::Msg::Edit(
                        super::pages::list_links::EditMsg::CreateNewLink
                    )
                )),
                t("add-link"),
            ],],
            li![a![
                attrs! {At::Href => crate::Urls::new(base_url).create_user()},
                ev(Ev::Click, |_| Msg::ListUsers(
                    super::pages::list_users::Msg::Edit(
                        super::pages::list_users::UserEditMsg::CreateNewUser
                    )
                )),
                t("invite-user"),
            ],],
            li![a![
                attrs! {At::Href => crate::Urls::new(base_url).list_users()},
                t("list-users"),
            ],],
        ],
        ol![
            li![div![welcome]],
            li![a![
                attrs! {At::Href => "#"},
                ev(Ev::Click, |_| Msg::NoMessage),
                t("logout"),
            ]]
        ]
    ]
}
