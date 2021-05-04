use enum_map::{Enum, EnumMap};
use serde::{Deserialize, Serialize};

use super::general::{Filter, Operation, Ordering};

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

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq, Hash, Enum)]
pub enum LinkOverviewColumns {
    Code,
    Description,
    Target,
    Author,
    Statistics,
}
