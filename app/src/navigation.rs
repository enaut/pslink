//! Create the top menu of the app
use fluent::fluent_args;
use pslink_locales::I18n;
use pslink_shared::{
    apirequests::users::Role,
    datatypes::{Lang, User},
};
use seed::{a, attrs, div, li, nav, nodes, ol, prelude::*, Url, C};

use crate::Msg;

/// Generate the top navigation menu of all pages.
///
/// The menu options are translated using the i18n module.
#[must_use]
pub fn navigation(i18n: &I18n, base_url: &Url, user: &User) -> Node<Msg> {
    // A shortcut for translating strings.
    let t = move |key: &str| i18n.translate(key, None);
    // Translate the wellcome message
    let welcome = i18n.translate(
        "welcome-user",
        Some(&fluent_args![ "username" => user.username.clone()]),
    );
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
            if user.role == Role::Admin {
                nodes![
                    // A button to create a new user
                    li![a![
                        attrs! {At::Href => crate::Urls::new(base_url).create_user()},
                        ev(Ev::Click, |_| Msg::ListUsers(
                            super::pages::list_users::Msg::Edit(
                                super::pages::list_users::UserEditMsg::CreateNewUser
                            )
                        )),
                        t("invite-user"),
                    ],]
                ]
            } else {
                nodes!()
            },
            // A button to list all users
            li![a![
                attrs! {At::Href => crate::Urls::new(base_url).list_users()},
                t("list-users"),
            ],],
        ],
        ol![
            li![div![
                C!("languageselector"),
                t("language"),
                a![ev(Ev::Click, |_| Msg::SetLanguage(Lang::DeDE)), "de"],
                a![ev(Ev::Click, |_| Msg::SetLanguage(Lang::EnUS)), "en"]
            ]],
            // The Welcome message
            li![div![welcome]],
            // The logout button
            li![a![ev(Ev::Click, |_| Msg::NotAuthenticated), t("logout"),]]
        ]
    ]
}
