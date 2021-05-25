use fluent::fluent_args;
use seed::{a, attrs, div, li, nav, ol, prelude::*, Url};
use shared::datatypes::User;

use crate::{i18n::I18n, Msg};

/// Generate the top navigation menu of all pages.
/// 
/// The menu options are translated using the i18n module.
#[must_use]
pub fn navigation(i18n: &I18n, base_url: &Url, user: &Option<User>) -> Node<Msg> {
    // A shortcut for translating strings.
    let t = move |key: &str| i18n.translate(key, None);
    // Translate the wellcome message
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
            // A button for the homepage, the list of URLs
            li![a![
                attrs! {At::Href => crate::Urls::new(base_url).list_links()},
                t("list-links"),
            ],],
            // A button to create a new shortened URL
            li![a![
                attrs! {At::Href => crate::Urls::new(base_url).create_link()},
                ev(Ev::Click, |_| Msg::ListLinks(
                    super::pages::list_links::Msg::Edit(
                        super::pages::list_links::EditMsg::CreateNewLink
                    )
                )),
                t("add-link"),
            ],],
            // A button to create a new user
            li![a![
                attrs! {At::Href => crate::Urls::new(base_url).create_user()},
                ev(Ev::Click, |_| Msg::ListUsers(
                    super::pages::list_users::Msg::Edit(
                        super::pages::list_users::UserEditMsg::CreateNewUser
                    )
                )),
                t("invite-user"),
            ],],
            // A button to list all users
            li![a![
                attrs! {At::Href => crate::Urls::new(base_url).list_users()},
                t("list-users"),
            ],],
        ],
        ol![
            // The Welcome message
            li![div![welcome]],
            // The logout button
            li![a![
                attrs! {At::Href => "/admin/logout/"},
                t("logout"),
            ]]
        ]
    ]
}
