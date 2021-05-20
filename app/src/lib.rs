pub mod i18n;
pub mod navigation;
pub mod pages;

use pages::list_links;
use pages::list_users;
use seed::{div, log, prelude::*, App, Url, C};
use shared::datatypes::User;

use crate::i18n::{I18n, Lang};

// ------ ------
//     Init
// ------ ------

fn init(url: Url, orders: &mut impl Orders<Msg>) -> Model {
    orders.subscribe(Msg::UrlChanged);
    orders.send_msg(Msg::GetLoggedUser);

    log!(url);

    let lang = I18n::new(Lang::DeDE);

    Model {
        index: 0,
        base_url: Url::new().add_path_part("app"),
        page: Page::init(url, orders, lang.clone()),
        i18n: lang,
        user: None,
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
    user: Option<User>,
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
}

fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::UrlChanged(url) => {
            log!("Url changed!");

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
            orders.skip(); // No need to rerender/ complicated way to move into the closure
            orders.perform_cmd(async {
                let response = fetch(
                    Request::new("/admin/json/get_logged_user/")
                        .method(Method::Post)
                        .json(&())
                        .expect("serialization failed"),
                )
                .await
                .expect("HTTP request failed");

                let user: User = response
                    .check_status() // ensure we've got 2xx status
                    .expect("status check failed")
                    .json()
                    .await
                    .expect("deserialization failed");

                Msg::UserReceived(user)
            });
        }
        Msg::UserReceived(user) => model.user = Some(user),
    }
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

fn view(model: &Model) -> Node<Msg> {
    div![
        C!["page"],
        navigation::navigation(&model.i18n, &model.base_url, &model.user),
        view_content(&model.page, &model.base_url),
    ]
}
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

// ------ ------
//     Start
// ------ ------
#[wasm_bindgen(start)]
pub fn main() {
    App::start("app", init, update, view);
}
