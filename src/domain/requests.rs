//! Request and response DTOs for fullstack API endpoints.

use std::ops::Deref;

use enum_map::{Enum, EnumMap};
use serde::{Deserialize, Serialize};

use crate::domain::models::{FullLink, Link, Role, User};

/// Filter one column according to the containing string.
#[derive(Clone, Deserialize, Serialize, Debug, Default, PartialEq, Eq)]
pub struct Filter {
    pub sieve: String,
}

impl Deref for Filter {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.sieve
    }
}

/// Possible order directions.
#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Debug)]
pub enum Ordering {
    Ascending,
    Descending,
}

/// An operation on a column.
#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Debug)]
pub struct Operation<T, V> {
    pub column: T,
    pub value: V,
}

/// To differentiate between creating a new record, editing, and deleting.
#[derive(Clone, Copy, Deserialize, Serialize, Debug, PartialEq, Eq, Default)]
pub enum EditMode {
    #[default]
    Create,
    Edit,
    Delete(bool),
}

/// An enumeration of the filterable and sortable link columns.
#[derive(Clone, Copy, Deserialize, Serialize, Debug, PartialEq, Eq, Hash, Enum)]
pub enum LinkOverviewColumns {
    Code,
    Description,
    Target,
    Author,
    Statistics,
}

/// Request a list of links respecting the filter, ordering, and pagination.
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct LinkRequestForm {
    pub filter: EnumMap<LinkOverviewColumns, Filter>,
    pub order: Option<Operation<LinkOverviewColumns, Ordering>>,
    pub offset: usize,
    pub amount: usize,
}

impl Default for LinkRequestForm {
    fn default() -> Self {
        Self {
            filter: EnumMap::default(),
            order: None,
            offset: 0,
            amount: 50,
        }
    }
}

/// Payload for creating and editing links.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct LinkDelta {
    pub edit: EditMode,
    pub id: Option<i64>,
    pub title: String,
    pub target: String,
    pub code: String,
    pub author: Option<i64>,
}

impl From<Link> for LinkDelta {
    fn from(l: Link) -> Self {
        Self {
            edit: EditMode::Edit,
            id: Some(l.id),
            title: l.title,
            target: l.target,
            code: l.code,
            author: Some(l.author),
        }
    }
}

impl From<&Link> for LinkDelta {
    fn from(l: &Link) -> Self {
        Self {
            edit: EditMode::Edit,
            id: Some(l.id),
            title: l.title.clone(),
            target: l.target.clone(),
            code: l.code.clone(),
            author: Some(l.author),
        }
    }
}

impl From<FullLink> for LinkDelta {
    fn from(l: FullLink) -> Self {
        Self::from(l.link)
    }
}

impl From<&FullLink> for LinkDelta {
    fn from(l: &FullLink) -> Self {
        Self::from(&l.link)
    }
}

/// The columns in the user view table.
#[derive(Clone, Copy, Deserialize, Serialize, Debug, PartialEq, Eq, Hash, Enum)]
pub enum UserOverviewColumns {
    Id,
    Email,
    Username,
}

/// Request an ordered, filtered, and paginated list of users from the server.
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct UserRequestForm {
    pub filter: EnumMap<UserOverviewColumns, Filter>,
    pub order: Option<Operation<UserOverviewColumns, Ordering>>,
    pub offset: usize,
    pub amount: usize,
}

impl Default for UserRequestForm {
    fn default() -> Self {
        Self {
            filter: EnumMap::default(),
            order: None,
            offset: 0,
            amount: 20,
        }
    }
}

/// Payload for creating and editing users.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct UserDelta {
    pub edit: EditMode,
    pub id: Option<i64>,
    pub username: String,
    pub email: String,
    pub password: Option<String>,
    pub role: Role,
}

impl From<User> for UserDelta {
    fn from(u: User) -> Self {
        Self {
            edit: EditMode::Edit,
            id: Some(u.id),
            username: u.username,
            email: u.email,
            password: None,
            role: u.role,
        }
    }
}

/// Session and server metadata returned to the frontend on initialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub user: Option<User>,
    pub hostname: String,
    pub base_url: String,
    pub brand_name: String,
    pub is_demo: bool,
}
