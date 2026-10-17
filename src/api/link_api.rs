//! Link CRUD and statistics server functions.

#[cfg(feature = "server")]
use dioxus::logger::tracing::info;
use dioxus::prelude::{ServerFnError, server};

use crate::domain::{
    models::{Clicks, FullLink, Link, LinkCheck},
    requests::{LinkDelta, LinkRequestForm},
};

/// Returns a filtered and ordered list of [`FullLink`]s for authenticated users.
///
/// # Errors
/// Fails with [`ServerFnError`] if the user is not authenticated or database access fails.
#[server(endpoint = "list_all_links")]
pub async fn list_all_allowed(parameters: LinkRequestForm) -> Result<Vec<FullLink>, ServerFnError> {
    let _user = crate::server::auth::require_user().await?;
    let state = crate::server::state::get_state().await?;
    crate::server::db::list_links_filtered(&state.db, &parameters).await
}

#[server(endpoint = "create_link")]
pub async fn create_link(mut data: LinkDelta) -> Result<Link, ServerFnError> {
    use crate::{
        domain::validation::{validate_link_code, validate_target_url},
        server::{
            auth::require_user,
            db::{self, NewLink},
            state::get_state,
        },
    };

    let user = require_user().await?;
    let state = get_state().await?;

    data.target = data.target.trim().to_string();
    data.title = data.title.trim().to_string();
    data.code = data.code.trim().to_string();

    validate_target_url(&data.target).map_err(ServerFnError::new)?;

    if data.code.is_empty() {
        data.code = db::generate_unique_code(&state.db).await?;
    } else {
        validate_link_code(&data.code, false).map_err(ServerFnError::new)?;
    }

    let code = data.code.clone();
    info!("Creating link for: {}", &code);
    let new_link = NewLink::from_link_delta(data, user.id);
    new_link.insert(&state.db).await?;

    let link = db::get_link_by_code(&state.db, &code).await?;
    crate::server::checker::spawn_check(state, link.id, link.target.clone());
    Ok(link)
}

#[server(endpoint = "save_link")]
pub async fn save_link(data: LinkDelta) -> Result<Link, ServerFnError> {
    use crate::{
        domain::{
            models::Role,
            validation::{validate_link_code, validate_target_url},
        },
        server::{auth::require_user, db, state::get_state},
    };

    let user = require_user().await?;
    let state = get_state().await?;

    let link_id = data
        .id
        .ok_or_else(|| ServerFnError::new("Link ID must be set"))?;
    let mut link = db::get_link_by_id(&state.db, link_id).await?;

    if user.role != Role::Admin && link.author != user.id {
        return Err(ServerFnError::new(
            "Not authorized to edit this link".to_owned(),
        ));
    }

    let target = data.target.trim().to_string();
    let code = data.code.trim().to_string();
    let title = data.title.trim().to_string();

    validate_target_url(&target).map_err(ServerFnError::new)?;
    // Allow empty code if editing the special "" root redirect link
    validate_link_code(&code, link.code.is_empty()).map_err(ServerFnError::new)?;

    let target_changed = link.target != target;
    link.code = code;
    link.title = title;
    link.target = target;

    db::update_link(&state.db, &link).await?;

    if target_changed {
        // The old history describes a different url.
        db::clear_link_checks(&state.db, link.id).await?;
        crate::server::checker::spawn_check(state, link.id, link.target.clone());
    }

    Ok(link)
}

#[server(endpoint = "delete_link")]
pub async fn delete_link(link_id: i64) -> Result<(), ServerFnError> {
    use crate::{
        domain::models::Role,
        server::{auth::require_user, db, state::get_state},
    };

    let user = require_user().await?;
    let state = get_state().await?;

    let link = db::get_link_by_id(&state.db, link_id).await?;

    if user.role != Role::Admin && link.author != user.id {
        return Err(ServerFnError::new(
            "Not authorized to delete this link".to_owned(),
        ));
    }

    db::delete_link_by_code(&state.db, &link.code).await?;

    Ok(())
}

#[server(endpoint = "get_link_statistics")]
pub async fn get_link_statistics(link_id: i64) -> Result<Clicks, ServerFnError> {
    let _user = crate::server::auth::require_user().await?;
    let state = crate::server::state::get_state().await?;
    let stats = crate::server::db::get_statistics(&state.db, link_id).await?;
    Ok(Clicks::Extended(stats))
}

/// Check a target url without storing the result (used for live feedback while editing a link).
///
/// # Errors
/// Fails with [`ServerFnError`] if the user is not authenticated or the url is malformed.
#[server(endpoint = "check_target")]
pub async fn check_target(url: String) -> Result<LinkCheck, ServerFnError> {
    use crate::{
        domain::validation::validate_target_url,
        server::{auth::require_user, checker, state::get_state},
    };

    let _user = require_user().await?;
    let state = get_state().await?;
    let url = url.trim().to_string();
    validate_target_url(&url).map_err(ServerFnError::new)?;
    Ok(checker::check_url(&state.http, &url).await)
}

/// Check the target of a stored link right now, store the result and return the check history
/// (most recent first).
///
/// # Errors
/// Fails with [`ServerFnError`] if the user is neither the author nor an admin or the database fails.
#[server(endpoint = "check_link_now")]
pub async fn check_link_now(link_id: i64) -> Result<Vec<LinkCheck>, ServerFnError> {
    use crate::{
        domain::models::Role,
        server::{auth::require_user, checker, db, state::get_state},
    };

    let user = require_user().await?;
    let state = get_state().await?;
    let link = db::get_link_by_id(&state.db, link_id).await?;

    if user.role != Role::Admin && link.author != user.id {
        return Err(ServerFnError::new(
            "Not authorized to check this link".to_owned(),
        ));
    }

    checker::check_and_store(&state, link.id, &link.target).await;
    db::list_link_checks(&state.db, link.id).await
}
