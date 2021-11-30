//! List all the links the own links editable or if an admin is logged in all links editable.
use std::ops::Deref;

use chrono::Datelike;
use enum_map::EnumMap;
use fluent::fluent_args;
use image::{DynamicImage, ImageOutputFormat, Luma};
use pslink_locales::I18n;
use qrcode::{render::svg, QrCode};
use seed::{
    a, attrs, div, h1, img, input, log, nodes, path, prelude::*, raw, section, span, svg, table,
    td, th, tr, Url, C, IF,
};
use web_sys::{IntersectionObserver, IntersectionObserverEntry, IntersectionObserverInit};

use pslink_shared::{
    apirequests::general::Ordering,
    apirequests::{
        general::{EditMode, Message, Operation, Status},
        links::{LinkDelta, LinkOverviewColumns, LinkRequestForm, StatisticsRequest},
    },
    datatypes::{Clicks, Count, FullLink, Lang, Loadable, Statistics, User, WeekCount},
};

use crate::{get_host, unwrap_or_return};

/// Setup the page
pub fn init(mut url: Url, orders: &mut impl Orders<Msg>, i18n: I18n) -> Model {
    // fetch the links to fill the list.
    orders.send_msg(Msg::Query(QueryMsg::Fetch));
    orders.perform_cmd(cmds::timeout(50, || Msg::SetupObserver));
    // if the url contains create_link set the edit_link variable.
    // This variable then opens the create link dialog.
    let dialog = match url.next_path_part() {
        Some("create_link") => Dialog::EditLink {
            link_delta: LinkDelta::default(),
            qr: Loadable::Data(None),
        },
        None | Some(_) => Dialog::None,
    };

    Model {
        links: Vec::new(),                      // will contain the links to display
        load_more: ElRef::new(), // will contain a reference to the load more button to be able to load more links after scrolling to the bottom.
        i18n,                    // to translate
        formconfig: LinkRequestForm::default(), // when requesting links the form is stored here
        inputs: EnumMap::default(), // the input fields for the searches
        dialog,
        handle_render: None,
        handle_timeout: None,
        observer: None, // load more on scroll - this is used to see if the load-more button is completely visible
        observer_callback: None,
        observer_entries: None,
        everything_loaded: false,
    }
}

#[derive(Debug)]
pub struct Model {
    links: Vec<Cached<FullLink, LinkCache>>, // will contain the links to display
    load_more: ElRef<web_sys::Element>,
    i18n: I18n,                                        // to translate
    formconfig: LinkRequestForm, // when requesting links the form is stored here
    inputs: EnumMap<LinkOverviewColumns, FilterInput>, // the input fields for the searches
    dialog: Dialog,              // User interaction - there can only ever be one dialog open.
    handle_render: Option<CmdHandle>, // Rendering qr-codes takes time... it is aborted when this handle is dropped and replaced.
    handle_timeout: Option<CmdHandle>, // Rendering qr-codes takes time... it is aborted when this handle is dropped and replaced.

    observer: Option<IntersectionObserver>,
    observer_callback: Option<Closure<dyn Fn(Vec<JsValue>)>>,
    observer_entries: Option<Vec<IntersectionObserverEntry>>,
    everything_loaded: bool,
}

impl Model {
    pub fn set_lang(&mut self, l: Lang) {
        self.i18n.set_lang(l);
    }
}

#[derive(Debug)]
pub struct LinkCache {
    qr: String,
    stats: Option<Node<Msg>>,
}

#[derive(Debug)]
pub struct Cached<T, S> {
    data: T,
    cache: S,
}

impl<T, S> Deref for Cached<T, S> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

/// There can always be only one dialog.
#[derive(Debug, Clone)]
enum Dialog {
    EditLink {
        link_delta: LinkDelta,
        qr: Loadable<QrGuard>,
    },
    Message(Status),
    Question(EditMsg),
    None,
}

/// A qr-code with `new` for creating a blob url and `Drop` for releasing the blob url.
#[derive(Debug, Clone)]
pub struct QrGuard {
    svg: String,
    url: String,
}

impl QrGuard {
    fn new(code: &str) -> Self {
        log!("Generating new QrCode");
        let svg = generate_qr_from_code(code);

        let mut properties = web_sys::BlobPropertyBag::new();
        properties.type_("image/png");
        let png_vec = generate_qr_png(code);

        let png_jsarray: JsValue = js_sys::Uint8Array::from(&png_vec[..]).into();
        // the buffer has to be an array of arrays
        let png_buffer: js_sys::Array = std::array::IntoIter::new([png_jsarray]).collect();
        let png_blob =
            web_sys::Blob::new_with_buffer_source_sequence_and_options(&png_buffer, &properties)
                .unwrap();
        let url = web_sys::Url::create_object_url_with_blob(&png_blob).unwrap();
        Self { svg, url }
    }
}

impl Drop for QrGuard {
    /// release the blob url
    fn drop(&mut self) {
        web_sys::Url::revoke_object_url(&self.url)
            .unwrap_or_else(|_| (log!("Failed to release url!")));
    }
}

/// Filter one column of the row.
#[derive(Default, Debug, Clone)]
struct FilterInput {
    filter_input: ElRef<web_sys::HtmlInputElement>,
}

/// A message can either edit or query. (or set a dialog)
#[derive(Clone, Debug)]
pub enum Msg {
    Query(QueryMsg), // Messages related to querying links
    Edit(EditMsg),   // Messages related to editing links
    ClearAll,        // Clear all messages
    SetupObserver,   // Make an observer for endless scroll
    Observed(Vec<IntersectionObserverEntry>),
    SetMessage(String), // Set a message to the user
}

/// All the messages related to requesting information from the server.
#[derive(Clone, Debug)]
pub enum QueryMsg {
    Fetch,
    FetchAdditional,
    GetStatistics(i64),
    ReceivedStatistics(Statistics),
    OrderBy(LinkOverviewColumns),
    Received(Vec<FullLink>),
    ReceivedAdditional(Vec<FullLink>),
    CodeFilterChanged(String),
    DescriptionFilterChanged(String),
    TargetFilterChanged(String),
    AuthorFilterChanged(String),
}

/// All the messages on storing information on the server.
#[derive(Debug, Clone)]
pub enum EditMsg {
    EditSelected(LinkDelta),
    GenerateQr(String),
    QrGenerated(Loadable<QrGuard>),
    CreateNewLink,
    Created(Status),
    EditCodeChanged(String),
    EditDescriptionChanged(String),
    EditTargetChanged(String),
    MayDeleteSelected(LinkDelta),
    DeleteSelected(LinkDelta),
    SaveLink,
    FailedToCreateLink,
    FailedToDeleteLink,
    DeletedLink(Status),
}

/// hide all dialogs
fn clear_all(model: &mut Model) {
    model.dialog = Dialog::None;
}

/// Split the update to Query updates and Edit updates.
pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::Query(msg) => process_query_messages(msg, model, orders),
        Msg::Edit(msg) => process_edit_messages(msg, model, orders),
        Msg::ClearAll => clear_all(model),
        Msg::Observed(entries) => {
            if let Some(entry) = entries.first() {
                log!(entry);
                if entry.is_intersecting() {
                    orders.send_msg(Msg::Query(QueryMsg::FetchAdditional));
                }
            }
        }
        Msg::SetMessage(msg) => {
            clear_all(model);
            model.dialog = Dialog::Message(Status::Error(Message { message: msg }));
        }
        Msg::SetupObserver => {
            orders.skip();

            // ---- observer callback ----
            let sender = orders.msg_sender();
            let callback = move |entries: Vec<JsValue>| {
                let entries = entries
                    .into_iter()
                    .map(IntersectionObserverEntry::from)
                    .collect();
                sender(Some(Msg::Observed(entries)));
            };
            let callback = Closure::wrap(Box::new(callback) as Box<dyn Fn(Vec<JsValue>)>);

            // ---- observer options ----
            let mut options = IntersectionObserverInit::new();
            options.threshold(&JsValue::from(1));
            // ---- observer ----
            log!("Trying to register observer");
            if let Ok(observer) =
                IntersectionObserver::new_with_options(callback.as_ref().unchecked_ref(), &options)
            {
                if let Some(element) = model.load_more.get() {
                    log!("element registered! ", element);
                    observer.observe(&element);

                    // Note: Drop `observer` is not enough. We have to call `observer.disconnect()`.
                    model.observer = Some(observer);
                    model.observer_callback = Some(callback);
                } else {
                    log!("element not yet registered! ");
                };
            } else {
                log!("Failed to get observer!");
            };
        }
    }
}

/// Process all messages for loading the information from the server.
pub fn process_query_messages(msg: QueryMsg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        QueryMsg::Fetch => {
            orders.skip(); // No need to rerender
            initial_load(model, orders);
        }
        QueryMsg::FetchAdditional => {
            orders.skip(); // No need to rerender
            consecutive_load(model, orders);
        }
        // Default to ascending ordering but if the links are already sorted according to this column toggle between ascending and descending ordering.
        QueryMsg::OrderBy(ref column) => order_columns(orders, model, column),
        QueryMsg::Received(response) => {
            model.links = response
                .into_iter()
                .map(|l| {
                    let cache = generate_qr_from_code(&l.link.code);
                    Cached {
                        data: l,
                        cache: LinkCache {
                            qr: cache,
                            stats: None,
                        },
                    }
                })
                .collect();
            request_all_statistics(orders, model);
        }
        QueryMsg::ReceivedAdditional(response) => {
            if response.len() < model.formconfig.amount {
                model.everything_loaded = true;
            };
            let mut new_links = response
                .into_iter()
                .map(|l| {
                    let cache = generate_qr_from_code(&l.link.code);
                    Cached {
                        data: l,
                        cache: LinkCache {
                            qr: cache,
                            stats: None,
                        },
                    }
                })
                .collect();
            model.links.append(&mut new_links);
            request_all_statistics(orders, model);
        }
        QueryMsg::CodeFilterChanged(s) => {
            log!("Filter is: ", &s);
            let sanit = s.chars().filter(|x| x.is_alphanumeric()).collect();
            model.formconfig.filter[LinkOverviewColumns::Code].sieve = sanit;
            orders.send_msg(Msg::Query(QueryMsg::Fetch));
        }
        QueryMsg::DescriptionFilterChanged(s) => {
            log!("Filter is: ", &s);
            let sanit = s.chars().filter(|x| x.is_alphanumeric()).collect();
            model.formconfig.filter[LinkOverviewColumns::Description].sieve = sanit;
            orders.send_msg(Msg::Query(QueryMsg::Fetch));
        }
        QueryMsg::TargetFilterChanged(s) => {
            log!("Filter is: ", &s);
            let sanit = s.chars().filter(|x| x.is_alphanumeric()).collect();
            model.formconfig.filter[LinkOverviewColumns::Target].sieve = sanit;
            orders.send_msg(Msg::Query(QueryMsg::Fetch));
        }
        QueryMsg::AuthorFilterChanged(s) => {
            log!("Filter is: ", &s);
            let sanit = s.chars().filter(|x| x.is_alphanumeric()).collect();
            model.formconfig.filter[LinkOverviewColumns::Author].sieve = sanit;
            orders.send_msg(Msg::Query(QueryMsg::Fetch));
        }
        QueryMsg::GetStatistics(link_id) => {
            orders.skip(); // No need to rerender
            request_statistics(orders, link_id);
        }
        QueryMsg::ReceivedStatistics(statistics) => {
            for i in 1..model.links.len() {
                if model.links[i].data.link.id == statistics.link_id {
                    model.links[i].data.clicks = Clicks::Extended(statistics.clone());

                    model.links[i].cache.stats = statistics
                        .values
                        .iter()
                        .max()
                        .map(|maximum| render_stats(statistics.clone(), maximum));
                }
            }
        }
    }
}

fn order_columns(orders: &mut impl Orders<Msg>, model: &mut Model, column: &LinkOverviewColumns) {
    model.formconfig.order = model.formconfig.order.as_ref().map_or_else(
        || {
            Some(Operation {
                column: column.clone(),
                value: Ordering::Ascending,
            })
        },
        |order| {
            Some(Operation {
                column: column.clone(),
                value: if &order.column == column && order.value == Ordering::Ascending {
                    Ordering::Descending
                } else {
                    Ordering::Ascending
                },
            })
        },
    );
    // After setting up the ordering fetch the links from the server again with the new filter settings.
    // If the new filters and ordering include more links the list would be incomplete otherwise.
    orders.send_msg(Msg::Query(QueryMsg::Fetch));

    // Also sort the links locally - can probably removed...
    model.links.sort_by(match column {
        LinkOverviewColumns::Code => {
            |o: &Cached<FullLink, _>, t: &Cached<FullLink, _>| o.link.code.cmp(&t.link.code)
        }
        LinkOverviewColumns::Description => {
            |o: &Cached<FullLink, _>, t: &Cached<FullLink, _>| o.link.title.cmp(&t.link.title)
        }
        LinkOverviewColumns::Target => {
            |o: &Cached<FullLink, _>, t: &Cached<FullLink, _>| o.link.target.cmp(&t.link.target)
        }
        LinkOverviewColumns::Author => {
            |o: &Cached<FullLink, _>, t: &Cached<FullLink, _>| o.user.username.cmp(&t.user.username)
        }
        LinkOverviewColumns::Statistics => {
            |o: &Cached<FullLink, _>, t: &Cached<FullLink, _>| o.clicks.cmp(&t.clicks)
        }
    });
}

fn request_all_statistics(orders: &mut impl Orders<Msg>, model: &Model) {
    for m in &model.links {
        match m.data.clicks {
            Clicks::Count(_) => {
                let id = m.link.id;
                orders.perform_cmd(cmds::timeout(500, move || {
                    Msg::Query(QueryMsg::GetStatistics(id))
                }));
            }
            Clicks::Extended(_) => (),
        }
    }
}

fn request_statistics(orders: &mut impl Orders<Msg>, link_id: i64) {
    let data = StatisticsRequest { link_id };
    orders.perform_cmd(async move {
        let data = data;
        let url = "/admin/json/get_link_statistics/";
        // create a request
        let request = unwrap_or_return!(
            Request::new(url).method(Method::Post).json(&data),
            Msg::SetMessage("Failed to parse data".to_string())
        );
        // send the request and receive a response
        let response = unwrap_or_return!(
            fetch(request).await,
            Msg::SetMessage("Failed to send data".to_string())
        );
        // check the html status to be 200
        let response = unwrap_or_return!(
            response.check_status(),
            Msg::SetMessage("Wrong response code".to_string())
        );
        // unpack the response into the `Vec<FullLink>`
        let statistics: Statistics = unwrap_or_return!(
            response.json().await,
            Msg::SetMessage("Invalid response".to_string())
        );
        // The message that is sent by perform_cmd after this async block is completed
        Msg::Query(QueryMsg::ReceivedStatistics(statistics))
    });
}

fn initial_load(model: &Model, orders: &mut impl Orders<Msg>) {
    let mut data = model.formconfig.clone();
    data.offset = 0;
    load_links(orders, data);
}
fn consecutive_load(model: &Model, orders: &mut impl Orders<Msg>) {
    let mut data = model.formconfig.clone();
    data.offset = model.links.len();
    load_links(orders, data);
}

/// Perform a request to the server to load the links from the server.
fn load_links(orders: &mut impl Orders<Msg>, data: LinkRequestForm) {
    orders.perform_cmd(async {
        let data = data;
        // create a request
        let request = unwrap_or_return!(
            Request::new("/admin/json/list_links/")
                .method(Method::Post)
                .json(&data),
            Msg::SetMessage("Failed to parse data".to_string())
        );
        // send the request and receive a response
        let response = unwrap_or_return!(
            fetch(request).await,
            Msg::SetMessage("Failed to send data".to_string())
        );
        // check the html status to be 200
        let response = unwrap_or_return!(
            response.check_status(),
            Msg::SetMessage("Wrong response code".to_string())
        );
        // unpack the response into the `Vec<FullLink>`
        let links: Vec<FullLink> = unwrap_or_return!(
            response.json().await,
            Msg::SetMessage("Invalid response".to_string())
        );
        // The message that is sent by perform_cmd after this async block is completed
        match data.offset.cmp(&0) {
            std::cmp::Ordering::Less => unreachable!(),
            std::cmp::Ordering::Equal => Msg::Query(QueryMsg::Received(links)),
            std::cmp::Ordering::Greater => Msg::Query(QueryMsg::ReceivedAdditional(links)),
        }
    });
}

/// Process all the events related to editing links.
#[allow(clippy::too_many_lines)]
pub fn process_edit_messages(msg: EditMsg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        EditMsg::EditSelected(link) => {
            let link_delta = link;
            model.dialog = Dialog::EditLink {
                link_delta: link_delta.clone(),
                qr: Loadable::Data(None),
            };
            let code = link_delta.code;
            model.handle_render = None;
            model.handle_timeout = Some(orders.perform_cmd_with_handle(cmds::timeout(300, || {
                Msg::Edit(EditMsg::GenerateQr(code))
            })));
        }
        EditMsg::GenerateQr(code) => {
            model.handle_render = Some(orders.perform_cmd_with_handle(async move {
                let qr_code = Loadable::Data(Some(QrGuard::new(&code)));
                Msg::Edit(EditMsg::QrGenerated(qr_code))
            }));
        }
        EditMsg::QrGenerated(qr_code) => {
            let new_dialog = if let Dialog::EditLink {
                ref link_delta,
                qr: _,
            } = model.dialog
            {
                Some(Dialog::EditLink {
                    link_delta: link_delta.clone(),
                    qr: qr_code,
                })
            } else {
                None
            };
            if let Some(dialog) = new_dialog {
                model.dialog = dialog;
            }
        }
        EditMsg::CreateNewLink => {
            clear_all(model);
            model.dialog = Dialog::EditLink {
                link_delta: LinkDelta::default(),
                qr: Loadable::Data(None),
            }
        }
        EditMsg::Created(success_msg) => {
            clear_all(model);
            model.dialog = Dialog::Message(success_msg);
            orders.send_msg(Msg::Query(QueryMsg::Fetch));
        }
        EditMsg::EditCodeChanged(s) => {
            if let Dialog::EditLink {
                mut link_delta,
                qr: _,
            } = model.dialog.clone()
            {
                link_delta.code = s.clone();
                model.handle_render = None;
                model.handle_timeout =
                    Some(orders.perform_cmd_with_handle(cmds::timeout(300, || {
                        Msg::Edit(EditMsg::GenerateQr(s))
                    })));
                model.dialog = Dialog::EditLink {
                    link_delta,
                    qr: Loadable::Loading,
                };
            }
        }
        EditMsg::EditDescriptionChanged(s) => {
            if let Dialog::EditLink {
                ref mut link_delta, ..
            } = model.dialog
            {
                link_delta.title = s;
            }
        }
        EditMsg::EditTargetChanged(s) => {
            if let Dialog::EditLink {
                ref mut link_delta, ..
            } = model.dialog
            {
                link_delta.target = s;
            }
        }
        EditMsg::SaveLink => {
            if let Dialog::EditLink { link_delta, .. } = model.dialog.clone() {
                save_link(link_delta, orders);
            }
        }
        EditMsg::FailedToCreateLink => {
            orders.send_msg(Msg::SetMessage("Failed to create this link!".to_string()));
            log!("Failed to create Link");
        }
        // capture including the message part
        link @ EditMsg::MayDeleteSelected(..) => {
            clear_all(model);
            model.dialog = Dialog::Question(link);
        }
        EditMsg::DeleteSelected(link) => delete_link(link, orders),
        EditMsg::FailedToDeleteLink => log!("Failed to delete Link"),

        EditMsg::DeletedLink(message) => {
            clear_all(model);
            model.dialog = Dialog::Message(message);
            orders.send_msg(Msg::Query(QueryMsg::Fetch));
        }
    }
}

/// Send a link save request to the server.
fn save_link(link_delta: LinkDelta, orders: &mut impl Orders<Msg>) {
    let data = link_delta;
    orders.perform_cmd(async {
        let data = data;
        // create the request
        let request = unwrap_or_return!(
            Request::new(match data.edit {
                EditMode::Create => "/admin/json/create_link/",
                EditMode::Edit => "/admin/json/edit_link/",
            })
            .method(Method::Post)
            .json(&data),
            Msg::SetMessage("Failed to encode the link!".to_string())
        );
        // perform the request
        let response =
            unwrap_or_return!(fetch(request).await, Msg::Edit(EditMsg::FailedToCreateLink));

        // check the response status
        let response = unwrap_or_return!(
            response.check_status(),
            Msg::SetMessage("Wrong response code".to_string())
        );
        // Parse the response
        let message: Status = unwrap_or_return!(
            response.json().await,
            Msg::SetMessage("Invalid response!".to_string())
        );

        Msg::Edit(EditMsg::Created(message))
    });
}

/// Send a link delete request to the server.
fn delete_link(link_delta: LinkDelta, orders: &mut impl Orders<Msg>) {
    orders.perform_cmd(async move {
        // create the request
        let request = unwrap_or_return!(
            Request::new("/admin/json/delete_link/")
                .method(Method::Post)
                .json(&link_delta),
            Msg::SetMessage("serialization failed".to_string())
        );
        // perform the request and receive a response
        let response =
            unwrap_or_return!(fetch(request).await, Msg::Edit(EditMsg::FailedToDeleteLink));

        // check the status of the response
        let response = unwrap_or_return!(
            response.check_status(),
            Msg::SetMessage("Wrong response code!".to_string())
        );
        // deserialize the response
        let message: Status = unwrap_or_return!(
            response.json().await,
            Msg::SetMessage(
                "Failed to parse the response! The link might or might not be deleted!".to_string()
            )
        );

        Msg::Edit(EditMsg::DeletedLink(message))
    });
}

/// view the page
///   * messages
///   * questions
///   * the table of links including sorting and searching
#[must_use]
pub fn view(model: &Model, logged_in_user: &User) -> Node<Msg> {
    let lang = &model.i18n.clone();
    // shortcut for translating
    let t = move |key: &str| lang.translate(key, None);
    section![
        // display a message if any
        match &model.dialog {
            Dialog::EditLink { link_delta, qr } => nodes![edit_or_create_link(link_delta, qr, t)],
            Dialog::Message(message) => nodes![div![
                C!["message", "center"],
                close_button(),
                match message {
                    Status::Success(m) | Status::Error(m) => &m.message,
                }
            ]],
            Dialog::Question(question) => nodes![div![
                C!["message", "center"],
                close_button(),
                if let EditMsg::MayDeleteSelected(l) = question.clone() {
                    nodes![div![
                        lang.translate(
                            "really-delete",
                            Some(&fluent_args!["code" => l.code.clone()])
                        ),
                        a![t("no"), C!["button"], ev(Ev::Click, |_| Msg::ClearAll)],
                        a![t("yes"), C!["button"], {
                            ev(Ev::Click, move |_| Msg::Edit(EditMsg::DeleteSelected(l)))
                        }]
                    ]]
                } else {
                    nodes!()
                }
            ]],
            Dialog::None => nodes![],
        },
        // display the list of links
        table![
            // Add the headlines
            view_link_table_head(&t),
            // Add filter fields right below the headlines
            view_link_table_filter_input(model, &t),
            // Add all the content lines
            model
                .links
                .iter()
                .map(|l| { view_link(l, logged_in_user, t) })
        ],
        if not(model.everything_loaded) {
            a![
                C!["loadmore", "button"],
                el_ref(&model.load_more),
                ev(Ev::Click, move |_| Msg::Query(QueryMsg::FetchAdditional)),
                img![C!["reloadicon"], attrs!(At::Src => "/static/reload.svg")],
                t("load-more-links")
            ]
        } else {
            a![C!["loadmore", "button"], t("no-more-links")]
        }
    ]
}

/// Create the headlines of the link table
fn view_link_table_head<F: Fn(&str) -> String>(t: F) -> Node<Msg> {
    tr![
        th![
            ev(Ev::Click, |_| Msg::Query(QueryMsg::OrderBy(
                LinkOverviewColumns::Code
            ))),
            t("link-code")
        ],
        th![
            ev(Ev::Click, |_| Msg::Query(QueryMsg::OrderBy(
                LinkOverviewColumns::Description
            ))),
            t("link-description")
        ],
        th![
            ev(Ev::Click, |_| Msg::Query(QueryMsg::OrderBy(
                LinkOverviewColumns::Target
            ))),
            t("link-target")
        ],
        th![
            ev(Ev::Click, |_| Msg::Query(QueryMsg::OrderBy(
                LinkOverviewColumns::Author
            ))),
            t("username")
        ],
        th![
            ev(Ev::Click, |_| Msg::Query(QueryMsg::OrderBy(
                LinkOverviewColumns::Statistics
            ))),
            t("statistics")
        ],
        th![],
        th![]
    ]
}

/// Create the filter fields in the table columns
fn view_link_table_filter_input<F: Fn(&str) -> String>(model: &Model, t: F) -> Node<Msg> {
    tr![
        C!["filters"],
        td![input![
            attrs! {
                At::Value => &model.formconfig.filter[LinkOverviewColumns::Code].sieve,
                At::Type => "search",
                At::Placeholder => t("search-placeholder")
            },
            input_ev(Ev::Input, |s| Msg::Query(QueryMsg::CodeFilterChanged(s))),
            el_ref(&model.inputs[LinkOverviewColumns::Code].filter_input),
        ]],
        td![input![
            attrs! {At::Value =>
            &model
                .formconfig.filter[LinkOverviewColumns::Description].sieve,
                At::Type => "search",
                At::Placeholder => t("search-placeholder")
            },
            input_ev(Ev::Input, |s| Msg::Query(
                QueryMsg::DescriptionFilterChanged(s)
            )),
            el_ref(&model.inputs[LinkOverviewColumns::Description].filter_input),
        ]],
        td![input![
            attrs! {At::Value =>
            &model
                .formconfig.filter[LinkOverviewColumns::Target].sieve,
                At::Type => "search",
                At::Placeholder => t("search-placeholder")
            },
            input_ev(Ev::Input, |s| Msg::Query(QueryMsg::TargetFilterChanged(s))),
            el_ref(&model.inputs[LinkOverviewColumns::Target].filter_input),
        ]],
        td![input![
            attrs! {At::Value =>
            &model
                .formconfig.filter[LinkOverviewColumns::Author].sieve,
                At::Type => "search",
                At::Placeholder => t("search-placeholder")
            },
            input_ev(Ev::Input, |s| Msg::Query(QueryMsg::AuthorFilterChanged(s))),
            el_ref(&model.inputs[LinkOverviewColumns::Author].filter_input),
        ]],
        // statistics and the delete column cannot be filtered
        td![],
        td![],
        td![],
    ]
}

/// display a single table row containing one link
fn view_link<F: Fn(&str) -> String>(
    l: &Cached<FullLink, LinkCache>,
    logged_in_user: &User,
    t: F,
) -> Node<Msg> {
    use pslink_shared::apirequests::users::Role;
    let link = LinkDelta::from(l.data.clone());
    tr![
        IF! (logged_in_user.role == Role::Admin
            || (logged_in_user.role == Role::Regular) && l.user.id == logged_in_user.id =>
            ev(Ev::Click, |_| Msg::Edit(EditMsg::EditSelected(link)))),
        td![&l.link.code],
        td![&l.link.title],
        td![&l.link.target],
        td![&l.user.username],
        match &l.clicks {
            Clicks::Count(Count { number }) => td![number],
            Clicks::Extended(statistics) =>
                if let Some(nodes) = l.cache.stats.clone() {
                    td![
                        span!(
                            C!("stats_total"),
                            t("total_clicks"),
                            match &l.data.clicks {
                                Clicks::Count(c) => c.number,
                                Clicks::Extended(e) => e.total.number,
                            }
                        ),
                        nodes
                    ]
                } else {
                    td!(statistics.total.number)
                },
        },
        {
            td![
                C!["table_qr"],
                a![
                    ev(Ev::Click, |event| event.stop_propagation()),
                    attrs![At::Href => format!("/admin/download/png/{}", &l.link.code),
                    At::Download => true.as_at_value()],
                    raw!(&l.cache.qr)
                ]
            ]
        },
        if logged_in_user.role == Role::Admin
            || (logged_in_user.role == Role::Regular) && l.user.id == logged_in_user.id
        {
            let link = LinkDelta::from(l.data.clone());
            td![
                ev(Ev::Click, |event| {
                    event.stop_propagation();
                    Msg::Edit(EditMsg::MayDeleteSelected(link))
                }),
                img![C!["trashicon"], attrs!(At::Src => "/static/trash.svg")]
            ]
        } else {
            td![]
        },
    ]
}

/// Render stats is best performed in the update rather than in the view cycle to avoid needless recalculations. Since the database only sends a list of weeks that contain clicks this function has to add the weeks that do not contain clicks. This is more easily said than done since the first and last week index are not constant, the ordering has to be right, and not every year has 52 weeks. But we ignore the last issue.
fn render_stats(q: Statistics, maximum: &WeekCount) -> Node<Msg> {
    let factor = 40.0 / f64::max(f64::from(maximum.total.number), 1.0);
    let mut full: Vec<WeekCount> = Vec::new();
    let mut week = chrono::Utc::now() - chrono::Duration::weeks(52);

    for with_clicks in q.values {
        loop {
            #[allow(clippy::cast_possible_wrap)]
            let cw = week.iso_week().week() as i32;
            if with_clicks.week == cw {
                full.push(with_clicks);
                week = week + chrono::Duration::weeks(1);
                break;
            }
            // otherwise add another empty week
            let nstat = WeekCount {
                month: week.naive_local(),
                total: Count { number: 0 },
                week: cw,
            };
            full.push(nstat);
            week = week + chrono::Duration::weeks(1);
        }
    }
    loop {
        #[allow(clippy::cast_possible_wrap)]
        let cw = week.iso_week().week() as i32;
        if week < chrono::Utc::now() {
            let nstat = WeekCount {
                month: week.naive_local(),
                total: Count { number: 0 },
                week: cw,
            };
            full.push(nstat);
            week = week + chrono::Duration::weeks(1);
        } else {
            break;
        }
    }
    #[allow(clippy::cast_possible_truncation)]
    let normalized: Vec<i64> = full
        .iter()
        .map(|v| (40.0 - f64::from(v.total.number) * factor).round() as i64)
        .collect();
    let mut points = Vec::new();
    points.push(format!("M 0 {}", &normalized[0]));
    #[allow(clippy::needless_range_loop)]
    for i in 1..normalized.len() {
        points.push(format!("L {} {}", i * 2, &normalized[i]));
    }

    svg![
        C!("statistics_graph"),
        path![attrs![At::D => points.join(" "), At::Stroke => "green", At::Fill => "transparent"]]
    ]
}

/// display a link editing dialog with save and close button
fn edit_or_create_link<F: Fn(&str) -> String>(
    link: &LinkDelta,
    qr: &Loadable<QrGuard>,
    t: F,
) -> Node<Msg> {
    div![
        // close button top right
        C!["editdialog", "center"],
        close_button(),
        h1![match &link.edit {
            EditMode::Edit => t("edit-link"),
            EditMode::Create => t("create-link"),
        }],
        table![
            tr![
                th![t("link-description")],
                td![input![
                    attrs! {
                        At::Value => &link.title,
                        At::Type => "text",
                        At::Placeholder => t("link-description")
                    },
                    input_ev(Ev::Input, |s| {
                        Msg::Edit(EditMsg::EditDescriptionChanged(s))
                    }),
                ]]
            ],
            tr![
                th![t("link-target")],
                td![input![
                    attrs! {
                        At::Value => &link.target,
                        At::Type => "text",
                        At::Placeholder => t("link-target")
                    },
                    input_ev(Ev::Input, |s| { Msg::Edit(EditMsg::EditTargetChanged(s)) }),
                ]]
            ],
            tr![
                th![t("link-code")],
                td![input![
                    attrs! {
                        At::Value => &link.code,
                        At::Type => "text",
                        At::Placeholder => t("link-code")
                    },
                    input_ev(Ev::Input, |s| { Msg::Edit(EditMsg::EditCodeChanged(s)) }),
                ],]
            ],
            tr![
                th![t("qr-code")],
                qr.as_ref().map_or_else(|| td!["Loading..."], render_qr),
            ]
        ],
        a![
            match &link.edit {
                EditMode::Edit => t("edit-link"),
                EditMode::Create => t("create-link"),
            },
            C!["button"],
            ev(Ev::Click, |_| Msg::Edit(EditMsg::SaveLink))
        ]
    ]
}

fn render_qr(qr: &QrGuard) -> Node<Msg> {
    td![a![
        span![C!["qrdownload"], "Download", raw!(&qr.svg),],
        attrs!(At::Href => qr.url, At::Download => "qr-code.png")
    ]]
}

/// generate a qr-code for a code
fn generate_qr_from_code(code: &str) -> String {
    generate_qr_from_link(&format!("https://{}/{}", get_host(), code))
}

/// generate a svg qr-code for a url
fn generate_qr_from_link(url: &str) -> String {
    if let Ok(qr) = QrCode::with_error_correction_level(&url, qrcode::EcLevel::L) {
        let svg = qr
            .render()
            .min_dimensions(100, 100)
            .dark_color(svg::Color("#000000"))
            .light_color(svg::Color("#ffffff"))
            .build();
        svg
    } else {
        // should never (only on very huge codes) happen.
        "".to_string()
    }
}

/// a close button for dialogs
fn close_button() -> Node<Msg> {
    div![
        C!["closebutton"],
        a!["\u{d7}"],
        ev(Ev::Click, |_| Msg::ClearAll)
    ]
}

/// generate a png qr-code for a url
fn generate_qr_png(code: &str) -> Vec<u8> {
    let qr = QrCode::with_error_correction_level(
        &format!("http://{}/{}", get_host(), code),
        qrcode::EcLevel::L,
    )
    .unwrap();
    let png = qr.render::<Luma<u8>>().quiet_zone(false).build();
    let mut temporary_data = std::io::Cursor::new(Vec::new());
    DynamicImage::ImageLuma8(png)
        .write_to(&mut temporary_data, ImageOutputFormat::Png)
        .unwrap();
    temporary_data.into_inner()
}
