use enum_map::{Enum, EnumMap};
use serde::{Deserialize, Serialize};

use super::general::{Filter, Operation, Ordering};

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct UserRequestForm {
    pub filter: EnumMap<UserOverviewColumns, Filter>,
    pub order: Option<Operation<UserOverviewColumns, Ordering>>,
    pub amount: usize,
}

impl Default for UserRequestForm {
    fn default() -> Self {
        Self {
            filter: EnumMap::default(),
            order: None,
            amount: 20,
        }
    }
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq, Hash, Enum)]
pub enum UserOverviewColumns {
    Id,
    Email,
    Username,
}
