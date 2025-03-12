//! Types for user requesting and saving

use enum_map::{Enum, EnumMap};
use serde::{Deserialize, Serialize};

use crate::datatypes::User;

use super::general::{EditMode, Filter, Operation, Ordering};

/// Request an ordered and filtered list of users from the server.
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

/// Data to login.
#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub struct LoginUser {
    pub username: String,
    pub password: String,
}

/// The Struct that is responsible for creating and editing users.
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
    /// Automatically create a `UserDelta` from a User.
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub user: Option<User>,
    pub hostname: String,
}

/// The columns in the user view table. The table can be ordered according to these.
#[allow(clippy::use_self)]
#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq, Hash, Enum)]
pub enum UserOverviewColumns {
    Id,
    Email,
    Username,
}

/// The possible roles a user could have. They are stored as i64 in the database
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Copy)]
pub enum Role {
    NotAuthenticated,
    Disabled,
    Regular,
    Admin,
}

impl Role {
    #[must_use]
    pub const fn convert(i: i64) -> Self {
        match i {
            0 => Self::Disabled,
            1 => Self::Regular,
            2 => Self::Admin,
            _ => Self::NotAuthenticated,
        }
    }

    #[must_use]
    pub const fn to_i64(self) -> i64 {
        match self {
            Role::NotAuthenticated => 3,
            Role::Disabled => 0,
            Role::Regular => 1,
            Role::Admin => 2,
        }
    }
}

impl Default for Role {
    fn default() -> Self {
        Self::Regular
    }
}
