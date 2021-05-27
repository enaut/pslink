pub mod i18n;
pub mod navigation;
pub mod pages;

use pages::list_links;
use pages::list_users;
use seed::attrs;
use seed::button;
use seed::input;
use seed::label;
use seed::{div, log, prelude::*, App, Url, C};
use shared::apirequests::users::LoginUser;
use shared::datatypes::Loadable;
use shared::datatypes::User;

use crate::i18n::{I18n, Lang};

// ------ ------
//     Init
// ------ ------

fn init(url: Url, orders: &mut impl Orders<Msg>) -> Model {
    orders.subscribe(Msg::UrlChanged);
    orders.send_msg(Msg::GetLoggedUser);

    log!(url);

    let lang = I18n::new(Lang::EnUS);

    Model {
        index: 0,
        base_url: Url::new().add_path_part("app"),
        page: Page::init(url, orders, lang.clone()),
        i18n: lang,
        user: Loadable::Data(None),
        login_form: LoginForm::default(),
        login_data: LoginUser::default(),
    }
}

// ------ ------
//     Model
// ------ ------

#[derive(Debug)]
struct Model {
    index: usize,
    base_url: Url,
    page: Page,
    i18n: i18n::I18n,
    user: Loadable<User>,
    login_form: LoginForm,
    login_data: LoginUser,
}

#[derive(Default, Debug)]
struct LoginForm {
    username: ElRef<web_sys::HtmlInputElement>,
    password: ElRef<web_sys::HtmlInputElement>,
}

#[derive(Debug)]
enum Page {
    Home(pages::list_links::Model),
    ListUsers(pages::list_users::Model),
    NotFound,
}

impl Page {
    fn init(mut url: Url, orders: &mut impl Orders<Msg>, i18n: I18n) -> Self {
        url.next_path_part();
        let result = match url.next_path_part() {
            None | Some("list_links") => Self::Home(pages::list_links::init(
                url,
                &mut orders.proxy(Msg::ListLinks),
                i18n,
            )),
            Some("list_users") => Self::ListUsers(pages::list_users::init(
                url,
                &mut orders.proxy(Msg::ListUsers),
                i18n,
            )),
            _other => Self::NotFound,
        };

        log!("Page initialized");
        result
    }
}

// ------ ------
//    Update
// ------ ------
#[derive(Clone)]
pub enum Msg {
    UrlChanged(subs::UrlChanged),
    ListLinks(list_links::Msg),
    ListUsers(list_users::Msg),
    GetLoggedUser,
    UserReceived(User),
    NoMessage,
    NotAuthenticated,
    Login,
    UsernameChanged(String),
    PasswordChanged(String),
}

fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::UrlChanged(url) => {
            model.page = Page::init(url.0, orders, model.i18n.clone());
        }
        Msg::ListLinks(msg) => {
            if let Page::Home(model) = &mut model.page {
                list_links::update(msg, model, &mut orders.proxy(Msg::ListLinks))
            }
        }
        Msg::ListUsers(msg) => {
            if let Page::ListUsers(model) = &mut model.page {
                list_users::update(msg, model, &mut orders.proxy(Msg::ListUsers))
            }
        }
        Msg::NoMessage => (),
        Msg::GetLoggedUser => {
            orders.skip(); // No need to rerender
            orders.perform_cmd(async {
                // create request
                let request = unwrap_or_return!(
                    Request::new("/admin/json/get_logged_user/")
                        .method(Method::Post)
                        .json(&()),
                    Msg::NotAuthenticated
                );
                // perform and get response
                let response = unwrap_or_return!(fetch(request).await, Msg::NotAuthenticated);
                // validate response status
                let response = unwrap_or_return!(response.check_status(), Msg::NotAuthenticated);
                let user: User = unwrap_or_return!(response.json().await, Msg::NotAuthenticated);

                Msg::UserReceived(user)
            });
        }
        Msg::UserReceived(user) => model.user = Loadable::Data(Some(user)),
        Msg::NotAuthenticated => {if model.user.is_some() {model.user = Loadable::Data(None); logout(orders)}},
        Msg::Login => {login_user(model, orders)}
        Msg::UsernameChanged(s) => model.login_data.username = s,
        Msg::PasswordChanged(s) => model.login_data.password = s,
    }
}

fn logout(orders: &mut impl Orders<Msg>) {
    orders.perform_cmd(async {let request = Request::new("/admin/logout/");
    unwrap_or_return!(fetch(request).await, Msg::GetLoggedUser);
    Msg::NotAuthenticated});

}

fn login_user(model: &mut Model, orders: &mut impl Orders<Msg>) {
    orders.skip(); // No need to rerender
    let data = model.login_data.clone();

    orders.perform_cmd(async {
        let data = data;
        // create request
        let request = unwrap_or_return!(
            Request::new("/admin/json/login_user/")
                .method(Method::Post)
                .json(&data),
            Msg::NotAuthenticated
        );
        // perform and get response
        let response = unwrap_or_return!(fetch(request).await, Msg::NotAuthenticated);
        // validate response status
        let response = unwrap_or_return!(response.check_status(), Msg::NotAuthenticated);
        let user: User = unwrap_or_return!(response.json().await, Msg::NotAuthenticated);

        Msg::UserReceived(user)
    });
}

pub struct Urls<'a> {
    base_url: std::borrow::Cow<'a, Url>,
}

impl<'a> Urls<'a> {
    /// Create a new `Urls` instance.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// Urls::new(base_url).home()
    /// ```
    pub fn new(base_url: impl Into<std::borrow::Cow<'a, Url>>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }

    /// Return base `Url`. If `base_url` isn't owned, it will be cloned.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// pub fn admin_urls(self) -> page::admin::Urls<'a> {
    ///     page::admin::Urls::new(self.base_url().add_path_part(ADMIN))
    /// }
    /// ```
    #[must_use]
    pub fn base_url(self) -> Url {
        self.base_url.into_owned()
    }
    #[must_use]
    pub fn home(self) -> Url {
        self.base_url()
    }
    #[must_use]
    pub fn list_links(self) -> Url {
        self.base_url().add_path_part("list_links")
    }
    #[must_use]
    pub fn create_link(self) -> Url {
        self.list_links().add_path_part("create_link")
    }
    #[must_use]
    pub fn list_users(self) -> Url {
        self.base_url().add_path_part("list_users")
    }
    #[must_use]
    pub fn create_user(self) -> Url {
        self.list_users().add_path_part("create_user")
    }
}

// ------ ------
//     View
// ------ ------

/// Render the menu and the subpages.
fn view(model: &Model) -> Node<Msg> {
    div![
        C!["page"],
        if let Some(ref user) = *model.user {
            div![
                navigation::navigation(&model.i18n, &model.base_url, user),
                view_content(&model.page, &model.base_url)
            ]
        } else {
            view_login(&model.i18n, &model)
        }
    ]
}

/// Render the subpages.
fn view_content(page: &Page, url: &Url) -> Node<Msg> {
    div![
        C!["container"],
        match page {
            Page::Home(model) => pages::list_links::view(model).map_msg(Msg::ListLinks),
            Page::ListUsers(model) => pages::list_users::view(model).map_msg(Msg::ListUsers),
            Page::NotFound => div![div![url.to_string()], "Page not found!"],
        }
    ]
}

fn view_login(lang: &I18n, model: &Model) -> Node<Msg> {
    let t = move |key: &str| lang.translate(key, None);

    div![
        C!["center", "login"],
        div![
            label![t("username")],
            input![
                input_ev(Ev::Input, |s| { Msg::UsernameChanged(s) }),
                attrs![
        At::Type => "text",
        At::Placeholder => t("username"),
        At::Name => "username",
        At::Value => model.login_data.username],
                el_ref(&model.login_form.username)
            ]
        ],
        div![
            label![t("password")],
            input![
                input_ev(Ev::Input, |s| { Msg::PasswordChanged(s) }),
                attrs![
            At::Type => "password",
            At::Placeholder => t("password"),
        At::Name => "password",
        At::Value => model.login_data.password],
                el_ref(&model.login_form.password)
            ]
        ],
        button![t("login"), ev(Ev::Click, |_| Msg::Login)]
    ]
}

// ------ ------
//     Start
// ------ ------
#[wasm_bindgen(start)]
pub fn main() {
    App::start("app", init, update, view);
}
