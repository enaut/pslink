use enum_map::{Enum, EnumMap};
use serde::{Deserialize, Serialize};

use crate::datatypes::{FullLink, Link};

use super::general::{EditMode, Filter, Operation, Ordering};

/// A generic list returntype containing the User and a Vec containing e.g. Links or Users
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct LinkRequestForm {
    pub filter: EnumMap<LinkOverviewColumns, Filter>,
    pub order: Option<Operation<LinkOverviewColumns, Ordering>>,
    pub amount: usize,
}

impl Default for LinkRequestForm {
    fn default() -> Self {
        Self {
            filter: EnumMap::default(),
            order: None,
            amount: 20,
        }
    }
}

/// The Struct that is responsible for creating and editing users.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct LinkDelta {
    pub edit: EditMode,
    pub id: Option<i64>,
    pub title: String,
    pub target: String,
    pub code: String,
    pub author: i64,
    pub created_at: Option<chrono::NaiveDateTime>,
}

impl From<Link> for LinkDelta {
    /// Automatically create a `UserDelta` from a User.
    fn from(l: Link) -> Self {
        Self {
            edit: EditMode::Edit,
            id: Some(l.id),
            title: l.title,
            target: l.target,
            code: l.code,
            author: l.author,
            created_at: Some(l.created_at),
        }
    }
}

impl From<FullLink> for LinkDelta {
    /// Automatically create a `UserDelta` from a User.
    fn from(l: FullLink) -> Self {
        Self {
            edit: EditMode::Edit,
            id: Some(l.link.id),
            title: l.link.title,
            target: l.link.target,
            code: l.link.code,
            author: l.link.author,
            created_at: Some(l.link.created_at),
        }
    }
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq, Hash, Enum)]
pub enum LinkOverviewColumns {
    Code,
    Description,
    Target,
    Author,
    Statistics,
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct QrCodeRequest {
    pub link_id: String,
    pub format: QrCodeFormat,
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct SvgQrCodeResponse {
    pub svg: String,
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub enum QrCodeFormat {
    Svg,
    Png,
}
