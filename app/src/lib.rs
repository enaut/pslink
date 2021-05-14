pub mod i18n;
pub mod navigation;
pub mod pages;

use pages::list_links;
use pages::list_users;
use seed::{div, log, prelude::*, App, Url, C};

use crate::i18n::{I18n, Lang};

// ------ ------
//     Init
// ------ ------

fn init(url: Url, orders: &mut impl Orders<Msg>) -> Model {
    orders.subscribe(Msg::UrlChanged);

    log!(url);

    let lang = I18n::new(Lang::DeDE);

    Model {
        index: 0,
        base_url: Url::new().add_path_part("app"),
        page: Page::init(url, orders, lang.clone()),
        i18n: lang,
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
                &mut orders.proxy(Msg::ListLinksMsg),
                i18n,
            )),
            Some("list_users") => Self::ListUsers(pages::list_users::init(
                url,
                &mut orders.proxy(Msg::ListUsersMsg),
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
#[allow(renamed_and_removed_lints, pub_enum_variant_names)]
#[derive(Clone)]
pub enum Msg {
    UrlChanged(subs::UrlChanged),
    ListLinksMsg(list_links::Msg),
    ListUsersMsg(list_users::Msg),
    NoMessage,
}

fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::UrlChanged(url) => {
            log!("Url changed!");

            model.page = Page::init(url.0, orders, model.i18n.clone());
        }
        Msg::ListLinksMsg(msg) => {
            if let Page::Home(model) = &mut model.page {
                list_links::update(msg, model, &mut orders.proxy(Msg::ListLinksMsg))
            }
        }
        Msg::ListUsersMsg(msg) => {
            if let Page::ListUsers(model) = &mut model.page {
                list_users::update(msg, model, &mut orders.proxy(Msg::ListUsersMsg))
            }
        }
        Msg::NoMessage => (),
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
        navigation::navigation(&model.i18n, &model.base_url,),
        view_content(&model.page, &model.base_url),
    ]
}
fn view_content(page: &Page, url: &Url) -> Node<Msg> {
    div![
        C!["container"],
        match page {
            Page::Home(model) => pages::list_links::view(model).map_msg(Msg::ListLinksMsg),
            Page::ListUsers(model) => pages::list_users::view(model).map_msg(Msg::ListUsersMsg),
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
