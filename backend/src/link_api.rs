#[cfg(feature = "server")]
use std::str::FromStr;

#[cfg(feature = "server")]
use crate::models::{LinkDbOperations as _, NewLink};
#[cfg(feature = "server")]
use dioxus::logger::tracing::info;
use dioxus::prelude::{ServerFnError, server, server_fn};
#[cfg(feature = "server")]
use enum_map::EnumMap;
use pslink_shared::{
    apirequests::links::{LinkDelta, LinkRequestForm},
    datatypes::{FullLink, Item, Link, ListWithOwner},
};

#[cfg(feature = "server")]
use pslink_shared::{
    apirequests::{
        general::{Filter, Operation, Ordering},
        links::LinkOverviewColumns,
        users::Role,
    },
    datatypes::{Clicks, Count, Lang, Secret, User},
};
/// Returns a List of `FullLink` meaning `Links` enriched by their author and statistics. This returns all links if the user is either Admin or Regular user.
///
/// Todo: this function only naively protects agains SQL-injections use better variants.
///
/// # Errors
/// Fails with [`ServerError`] if access to the database fails.
#[server(ListAllLinksFiltered, endpoint = "list_all_links")]
pub async fn list_all_allowed(
    parameters: LinkRequestForm,
) -> Result<ListWithOwner<FullLink>, ServerFnError> {
    let auth = crate::auth::get_session().await?;
    if auth.is_anonymous() {
        return Err(ServerFnError::new("Not authenticated".to_owned()));
    }
    let user = auth
        .current_user
        .expect("not authenticated")
        .get_user()
        .expect("User is authenticated");
    let db = crate::get_db().await;

    let mut querystring = "select
                        links.id as lid,
                        links.title as ltitle,
                        links.target as ltarget,
                        links.code as lcode,
                        links.author as lauthor,
                        links.created_at as ldate,
                        users.id as usid,
                        users.username as usern,
                        users.email as uemail,
                        users.role as urole,
                        users.language as ulang,
                        count(clicks.id) as counter
                    from
                        links
                        join users on links.author = users.id
                        left join clicks on links.id = clicks.link"
        .to_string();
    querystring.push_str(&generate_filter_sql(&parameters.filter));
    querystring.push_str("\n GROUP BY links.id");
    if let Some(order) = parameters.order {
        querystring.push_str(&generate_order_sql(&order));
    }
    querystring.push_str(&format!("\n LIMIT {}", parameters.amount));
    querystring.push_str(&format!("\n OFFSET {}", parameters.offset));

    use sqlx::Row;
    let links = sqlx::query(&querystring)
        .fetch_all(&db)
        .await?
        .into_iter()
        .map(|v| FullLink {
            link: Link {
                id: v.get("lid"),
                title: v.get("ltitle"),
                target: v.get("ltarget"),
                code: v.get("lcode"),
                author: v.get("lauthor"),
                created_at: v.get("ldate"),
            },
            user: User {
                id: v.get("usid"),
                username: v.get("usern"),
                email: v.get("uemail"),
                password: Secret::new("invalid".to_string()),
                role: Role::convert(v.get("urole")),
                language: Lang::from_str(v.get("ulang")).expect("Should parse"),
            },
            clicks: Clicks::Count(Count {
                number: v.get("counter"), /* count is never None */
            }),
        });
    // show all links
    let all_links: Vec<FullLink> = links.collect();
    Ok(ListWithOwner {
        user,
        list: all_links,
    })
}

/// Generate a filter statement for the SQL-Query according to the parameters...
///
/// Todo: this function only naively protects agains SQL-injections use better variants.
#[cfg(feature = "server")]
fn generate_filter_sql(filters: &EnumMap<LinkOverviewColumns, Filter>) -> String {
    let mut result = String::new();
    let filterstring = filters
        .iter()
        .filter_map(|(column, sieve)| {
            // avoid sql injections
            let sieve: String = sieve.chars().filter(|x| x.is_alphanumeric()).collect();
            if sieve.is_empty() {
                None
            } else {
                Some(match column {
                    LinkOverviewColumns::Code => {
                        format!("\n lcode LIKE '%{}%'", sieve)
                    }
                    LinkOverviewColumns::Description => {
                        format!("\n ltitle LIKE '%{}%'", sieve)
                    }
                    LinkOverviewColumns::Target => {
                        format!("\n ltarget LIKE '%{}%'", sieve)
                    }
                    LinkOverviewColumns::Author => {
                        format!("\n usern LIKE '%{}%'", sieve)
                    }
                    LinkOverviewColumns::Statistics => {
                        format!("\n counter LIKE '%{}%'", sieve)
                    }
                })
            }
        })
        .collect::<Vec<String>>()
        .join(" AND ");
    if filterstring.len() > 1 {
        result.push_str("\n WHERE ");
        result.push_str(&filterstring);
    }
    result
}

/// A macro to translate the Ordering Type into a sql ordering string.
#[cfg(feature = "server")]
macro_rules! ts {
    ($ordering:expr) => {
        match $ordering {
            Ordering::Ascending => "ASC",
            Ordering::Descending => "DESC",
        }
    };
}

/// Generate a order statement for the SQL-Query according to the parameters...
#[cfg(feature = "server")]
fn generate_order_sql(order: &Operation<LinkOverviewColumns, Ordering>) -> String {
    match order.column {
        LinkOverviewColumns::Code => {
            format!("\n ORDER BY lcode {}", ts!(order.value))
        }
        LinkOverviewColumns::Description => {
            format!("\n ORDER BY ltitle {}", ts!(order.value))
        }
        LinkOverviewColumns::Target => {
            format!("\n ORDER BY ltarget {}", ts!(order.value))
        }
        LinkOverviewColumns::Author => {
            format!("\n ORDER BY usern {}", ts!(order.value))
        }
        LinkOverviewColumns::Statistics => {
            format!("\n ORDER BY counter {}", ts!(order.value))
        }
    }
}

#[server(CreateLink, endpoint = "create_link")]
pub async fn create_link(data: LinkDelta) -> Result<Item<Link>, ServerFnError> {
    let auth = crate::auth::get_session().await?;
    if auth.is_anonymous() {
        return Err(ServerFnError::new("Not authenticated".to_owned()));
    }
    let user = auth
        .current_user
        .expect("not authenticated")
        .get_user()
        .expect("User is authenticated");

    let code = data.code.clone();
    info!("Creating link for: {}", &code);
    let new_link = NewLink::from_link_delta(data, user.id);
    info!("Creating link for: {:?}", &new_link);

    new_link.insert().await?;
    let new_link: Link = get_link_simple(code).await?;
    Ok(Item {
        user,
        item: new_link,
    })
}

#[server(SaveLink, endpoint = "save_link")]
pub async fn save_link(data: LinkDelta) -> Result<Item<Link>, ServerFnError> {
    let auth = crate::auth::get_session().await?;
    if auth.is_anonymous() {
        return Err(ServerFnError::new("Not authenticated".to_owned()));
    }
    let user = auth
        .current_user
        .expect("not authenticated")
        .get_user()
        .expect("User is authenticated");

    // Get existing link first
    let mut link = Link::get_link_by_id(data.id.expect("Link ID must be set")).await?;

    // Verify ownership
    if user.role != Role::Admin && link.author != user.id {
        return Err(ServerFnError::new(
            "Not authorized to edit this link".to_owned(),
        ));
    }

    // Update link fields
    link.code = data.code;
    link.title = data.title;
    link.target = data.target;

    // Use the trait method to update
    link.update_link().await?;

    Ok(Item { user, item: link })
}

#[server(GetLinkSimple, endpoint = "get_link_simple")]
pub async fn get_link_simple(link_code: String) -> Result<Link, ServerFnError> {
    info!("Getting link for {:?}", link_code);

    let link = Link::get_link_by_code(&link_code).await?;
    info!("Foun d link for {:?}", link);
    Ok(link)
}

#[server(DeleteLink, endpoint = "delete_link")]
pub async fn delete_link(link_id: i64) -> Result<(), ServerFnError> {
    let auth = crate::auth::get_session().await?;
    if auth.is_anonymous() {
        return Err(ServerFnError::new("Not authenticated".to_owned()));
    }
    let user = auth
        .current_user
        .expect("not authenticated")
        .get_user()
        .expect("User is authenticated");

    // Get existing link first
    let link = Link::get_link_by_id(link_id).await?;

    // Verify ownership
    if link.author != user.id {
        return Err(ServerFnError::new(
            "Not authorized to delete this link".to_owned(),
        ));
    }

    // Use the trait method to delete
    Link::delete_link_by_code(&link.code).await?;

    Ok(())
}
