use fluent::fluent_args;
use seed::{a, attrs, div, li, nav, ol, prelude::*, Url};

use crate::{i18n::I18n, Msg};
#[must_use]
pub fn navigation(i18n: &I18n, base_url: &Url) -> Node<Msg> {
    let username = fluent_args![ "username" => "enaut"];
    macro_rules! t {
        { $key:expr } => {
            {
                i18n.translate($key, None)
            }
        };
        { $key:expr, $args:expr } => {
            {
                i18n.translate($key, Some(&$args))
            }
        };
    }
    nav![
        ol![
            li![a![
                attrs! {At::Href => crate::Urls::new(base_url).list_links()},
                t!("list-links"),
            ],],
            li![a![ev(Ev::Click, |_| Msg::NoMessage), t!("add-link"),],],
            li![a![
                attrs! {At::Href => crate::Urls::new(base_url).create_user()},
                ev(Ev::Click, |_| Msg::ListUsers(
                    super::pages::list_users::Msg::Edit(
                        super::pages::list_users::UserEditMsg::CreateNewUser
                    )
                )),
                t!("invite-user"),
            ],],
            li![a![
                attrs! {At::Href => crate::Urls::new(base_url).list_users()},
                t!("list-users"),
            ],],
        ],
        ol![
            li![div![t!("welcome-user", username)]],
            li![a![
                attrs! {At::Href => "#"},
                ev(Ev::Click, |_| Msg::NoMessage),
                t!("logout"),
            ]]
        ]
    ]
}
