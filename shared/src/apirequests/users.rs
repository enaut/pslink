use enum_map::{Enum, EnumMap};
use serde::{Deserialize, Serialize};

use crate::datatypes::User;

use super::general::{EditMode, Filter, Operation, Ordering};

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct UserRequestForm {
    // The filters up to one for each column
    pub filter: EnumMap<UserOverviewColumns, Filter>,
    // Order According to this column
    pub order: Option<Operation<UserOverviewColumns, Ordering>>,
    // Return a maximum of `amount` results
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

/// The Struct that is responsible for creating and editing users.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct UserDelta {
    pub edit: EditMode,
    pub id: Option<i64>,
    pub username: String,
    pub email: String,
    pub password: Option<String>,
}

impl From<User> for UserDelta {
    /// Automatically create a `UserDelta` from a User.
    fn from(u: User) -> Self {
        Self {
            edit: EditMode::Edit,
            id: Some(u.id),
            username: u.username,
            email: u.email,
            password: None,
        }
    }
}

/// The columns in the user view table. The table can be ordered according to these.
#[allow(clippy::use_self)]
#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq, Hash, Enum)]
pub enum UserOverviewColumns {
    Id,
    Email,
    Username,
}
