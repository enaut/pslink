//! The admin interface of pslink. It communicates with the server mostly via https and json.
pub mod i18n;
pub mod navigation;
pub mod pages;

use gloo_console::log;
use gloo_net::http::Request;
use i18n::I18n;
use pages::list_links;
use pages::list_users;
use pslink_shared::{
    apirequests::users::LoginUser,
    datatypes::{Lang, Loadable, User},
};
use seed::window;
use seed::{attrs, button, div, input, label, prelude::*, App, Url, C, IF};
// ------ ------
//     Init
// ------ ------

fn init(url: Url, orders: &mut impl Orders<Msg>) -> Model {
    orders.subscribe(Msg::UrlChanged);
    orders.send_msg(Msg::GetLoggedUser);

    let lang = I18n::new(Lang::EnUS);

    Model {
        index: 0,
        location: Location::new(url),
        page: Page::NotFound,
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
    location: Location,
    page: Page,
    i18n: I18n,
    user: Loadable<User>,
    login_form: LoginForm,
    login_data: LoginUser,
}

impl Model {
    fn set_lang(&mut self, l: Lang) {
        self.i18n.set_lang(l);
        match &mut self.page {
            Page::Home(ref mut m) => m.set_lang(l),
            Page::ListUsers(ref mut m) => m.set_lang(l),
            Page::NotFound => (),
        }
    }
}

/// The input fields of the login dialog.
#[derive(Default, Debug)]
struct LoginForm {
    username: ElRef<web_sys::HtmlInputElement>,
    password: ElRef<web_sys::HtmlInputElement>,
}

/// All information regarding the current location
#[derive(Debug)]
struct Location {
    host: String,
    base_url: Url,
    current_url: Url,
}

impl Location {
    fn new(url: Url) -> Self {
        let host = get_host();
        Self {
            host,
            base_url: Url::new().add_path_part("app"),
            current_url: url,
        }
    }
}

/// Get the url from the address bar.
#[must_use]
pub fn get_host() -> String {
    window()
        .location()
        .host()
        .expect("Failed to extract the host of the url")
}

/// The pages:
///   * `Home` for listing of links
///   * `ListUsers` for listing of users
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
enum Page {
    Home(pages::list_links::Model),
    ListUsers(pages::list_users::Model),
    NotFound,
}

impl Page {
    fn init(mut url: Url, orders: &mut impl Orders<Msg>, i18n: I18n) -> Self {
        log!(&url.to_string());
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

        orders.perform_cmd(async {
            // create request
            let request = Request::get("/admin/json/get_language/").send();
            // perform and get response
            let response = unwrap_or_return!(request.await, Msg::NoMessage);
            // validate response status
            let result = if response.ok() {
                let lang: Lang = unwrap_or_return!(response.json().await, Msg::NoMessage);

                Msg::LanguageChanged(lang)
            } else {
                Msg::NoMessage
            };
            result
        });

        log!("Page initialized");
        result
    }
}

// ------ ------
//    Update
// ------ ------

/// The messages regarding authentication and settings.
#[derive(Clone)]
pub enum Msg {
    UrlChanged(seed::app::subs::UrlChanged),
    ListLinks(list_links::Msg),
    ListUsers(list_users::Msg),
    GetLoggedUser,
    UserReceived(User),
    NoMessage,
    NotAuthenticated,
    Logout,
    Login,
    UsernameChanged(String),
    PasswordChanged(String),
    SetLanguage(Lang),
    LanguageChanged(Lang),
}

/// react to settings and authentication changes.
fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::UrlChanged(url) => {
            model.page = Page::init(url.0, orders, model.i18n.clone());
        }
        Msg::ListLinks(msg) => {
            if let Page::Home(model) = &mut model.page {
                list_links::update(msg, model, &mut orders.proxy(Msg::ListLinks));
            }
        }
        Msg::ListUsers(msg) => {
            if let Page::ListUsers(model) = &mut model.page {
                list_users::update(msg, model, &mut orders.proxy(Msg::ListUsers));
            }
        }
        Msg::NoMessage => (),
        Msg::GetLoggedUser => {
            model.user = Loadable::Loading;
            orders.perform_cmd(async {
                let request = unwrap_or_return!(
                    Request::post("/admin/json/get_logged_user/").json(&()),
                    Msg::Logout
                )
                .send();
                let response = unwrap_or_return!(request.await, Msg::Logout);
                if response.ok() {
                    let user: User = unwrap_or_return!(response.json().await, Msg::Logout);
                    Msg::UserReceived(user)
                } else {
                    Msg::Logout
                }
            });
        }
        Msg::UserReceived(user) => {
            model.set_lang(user.language);
            model.user = Loadable::Data(Some(user));
            model.page = Page::init(
                model.location.current_url.clone(),
                orders,
                model.i18n.clone(),
            );
        }
        Msg::NotAuthenticated => {
            if model.user.is_some() {
                model.user = Loadable::Data(None);
                logout(orders);
            }
            model.user = Loadable::Data(None);
        }
        Msg::Logout => {
            model.user = Loadable::Data(None);
            logout(orders);
        }
        Msg::Login => login_user(model, orders),
        Msg::UsernameChanged(s) => model.login_data.username = s,
        Msg::PasswordChanged(s) => model.login_data.password = s,
        Msg::SetLanguage(l) => {
            change_language(l, orders);
        }
        Msg::LanguageChanged(l) => {
            log!(format!("Changed Language: {:?}", &l));
            model.set_lang(l);
        }
    }
}

/// switch the language
fn change_language(l: Lang, orders: &mut impl Orders<Msg>) {
    orders.perform_cmd(async move {
        let request = unwrap_or_return!(
            Request::post("/admin/json/change_language/").json(&l),
            Msg::NoMessage
        )
        .send();
        let response = unwrap_or_return!(request.await, Msg::NoMessage);
        if response.ok() {
            let l: Lang = unwrap_or_return!(response.json().await, Msg::NoMessage);
            Msg::LanguageChanged(l)
        } else {
            Msg::NoMessage
        }
    });
}

/// logout on the server
fn logout(orders: &mut impl Orders<Msg>) {
    orders.perform_cmd(async {
        let request = Request::post("/admin/logout/").send();
        unwrap_or_return!(request.await, Msg::GetLoggedUser);
        Msg::NotAuthenticated
    });
}

/// login using username and password
fn login_user(model: &mut Model, orders: &mut impl Orders<Msg>) {
    model.user = Loadable::Loading;
    let data = model.login_data.clone();

    orders.perform_cmd(async move {
        let request = unwrap_or_return!(
            Request::post("/admin/json/login_user/").json(&data),
            Msg::NotAuthenticated
        )
        .send();
        let response = unwrap_or_return!(request.await, Msg::NotAuthenticated);
        if response.ok() {
            let user: User = unwrap_or_return!(response.json().await, Msg::NotAuthenticated);
            Msg::UserReceived(user)
        } else {
            Msg::NotAuthenticated
        }
    });
}

/// to create urls for different subpages
pub struct Urls<'a> {
    base_url: std::borrow::Cow<'a, Url>,
}

impl<'a> Urls<'a> {
    /// Create a new `Urls` instance.
    ///
    /// # Example
    ///
    /// ```ignore
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
    /// ```ignore
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
        match model.user {
            Loadable::Data(Some(ref user)) => div![
                navigation::navigation(&model.i18n, &model.location.base_url, user),
                view_content(&model.page, &model.location.base_url, user)
            ],
            Loadable::Data(None) => view_login(&model.i18n, model),
            Loadable::Loading => div![C!("lds-ellipsis"), div!(), div!(), div!(), div!()],
        }
    ]
}

/// Render the subpages.
fn view_content(page: &Page, url: &Url, user: &User) -> Node<Msg> {
    div![
        C!["container"],
        match page {
            Page::Home(model) => pages::list_links::view(model, user).map_msg(Msg::ListLinks),
            Page::ListUsers(model) => pages::list_users::view(model, user).map_msg(Msg::ListUsers),
            Page::NotFound => div![div![url.to_string()], "Page not found!"],
        }
    ]
}

/// If not logged in render the login form
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
                keyboard_ev(Ev::KeyDown, |keyboard_event| {
                    IF!(keyboard_event.key() == "Enter" => Msg::Login)
                }),
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
