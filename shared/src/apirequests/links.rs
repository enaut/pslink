//! types for link requesting and saving.

use enum_map::{Enum, EnumMap};
use serde::{Deserialize, Serialize};

use crate::datatypes::{FullLink, Link};

use super::general::{EditMode, Filter, Operation, Ordering};

/// Request a list of users respecting the filter and ordering.
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
            amount: 60,
        }
    }
}

/// The Struct that is responsible for creating and editing links.
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
    /// Automatically create a `LinkDelta` from a Link.
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
    /// Automatically create a `LinkDelta` from a Link.
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
    /// Automatically create a `LinkDelta` from a `FullLink`.
    fn from(l: FullLink) -> Self {
        Self {
            edit: EditMode::Edit,
            id: Some(l.link.id),
            title: l.link.title,
            target: l.link.target,
            code: l.link.code,
            author: Some(l.link.author),
        }
    }
}
impl From<&FullLink> for LinkDelta {
    /// Automatically create a `LinkDelta` from a `FullLink`.
    fn from(l: &FullLink) -> Self {
        Self {
            edit: EditMode::Edit,
            id: Some(l.link.id),
            title: l.link.title.clone(),
            target: l.link.target.clone(),
            code: l.link.code.clone(),
            author: Some(l.link.author),
        }
    }
}

/// An enumeration of the filterable columns
#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq, Hash, Enum)]
pub enum LinkOverviewColumns {
    Code,
    Description,
    Target,
    Author,
    Statistics,
}

/// A struct to request the statistics of a link
#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct StatisticsRequest {
    pub link_id: i64,
}

/// A struct to request a qr-code from the server
#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct QrCodeRequest {
    pub link_id: String,
    pub format: QrCodeFormat,
}

/// The response to a qr-request
#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct SvgQrCodeResponse {
    pub svg: String,
}

/// Available formats of qr-codes
#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub enum QrCodeFormat {
    Svg,
    Png,
}
