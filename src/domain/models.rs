//! Core domain models used across client and server.

use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize, Serializer};
use strum_macros::{AsRefStr, EnumIter, EnumString};

/// The possible roles a user could have. They are stored as i64 in the database.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Copy, Default)]
pub enum Role {
    NotAuthenticated,
    Disabled,
    #[default]
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
            Self::NotAuthenticated => 3,
            Self::Disabled => 0,
            Self::Regular => 1,
            Self::Admin => 2,
        }
    }
}

/// An `enum` containing the available languages.
/// To add an additional language add it to this enum as well as an appropriate file into the translations folder.
#[allow(clippy::upper_case_acronyms)]
#[derive(
    Debug,
    Copy,
    Clone,
    EnumIter,
    EnumString,
    AsRefStr,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
)]
#[strum(ascii_case_insensitive)]
pub enum Lang {
    #[default]
    #[strum(serialize = "en-US", serialize = "en", serialize = "enUS")]
    EnUS,
    #[strum(serialize = "de-DE", serialize = "de", serialize = "deDE")]
    DeDE,
}

impl std::fmt::Display for Lang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// A User of the pslink service (public/shared representation without password hashes).
#[derive(PartialEq, Eq, Serialize, Deserialize, Clone, Debug)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub role: Role,
    pub language: Lang,
}

/// A short url of the link service.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub id: i64,
    pub title: String,
    pub target: String,
    pub code: String,
    pub author: i64,
    pub created_at: NaiveDateTime,
}

/// A link together with its author and its click-count.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct FullLink {
    pub link: Link,
    pub user: User,
    pub clicks: Clicks,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum Clicks {
    Count(Count),
    Extended(Statistics),
}

impl PartialEq for Clicks {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Count(l0), Self::Count(r0)) => l0.number == r0.number,
            (Self::Extended(l0), Self::Extended(r0)) => l0.total.number == r0.total.number,
            (Self::Count(l0), Self::Extended(r0)) => l0.number == r0.total.number,
            (Self::Extended(l0), Self::Count(r0)) => l0.total.number == r0.number,
        }
    }
}

impl PartialOrd for Clicks {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Clicks {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Count(l0), Self::Count(r0)) => l0.number.cmp(&r0.number),
            (Self::Extended(l0), Self::Extended(r0)) => l0.total.number.cmp(&r0.total.number),
            (Self::Count(l0), Self::Extended(r0)) => l0.number.cmp(&r0.total.number),
            (Self::Extended(l0), Self::Count(r0)) => l0.total.number.cmp(&r0.number),
        }
    }
}

impl Eq for Clicks {}

/// When statistics are counted.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Count {
    pub number: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WeekCount {
    pub full_date: NaiveDate,
    pub total: Count,
    pub week: i32,
}

impl Eq for WeekCount {}

impl PartialOrd for WeekCount {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for WeekCount {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.total.number.cmp(&other.total.number)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Statistics {
    pub link_id: i64,
    pub total: Count,
    pub values: Vec<WeekCount>,
}

/// Secret value: Display, Debug, and Serialize do not expose the inner secret string.
#[derive(PartialEq, Eq, Clone, Deserialize)]
#[serde(from = "String")]
pub struct Secret {
    pub secret: Option<String>,
    pub is_random: bool,
}

impl From<String> for Secret {
    fn from(_: String) -> Self {
        Self {
            secret: None,
            is_random: false,
        }
    }
}

impl Serialize for Secret {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str("*****SECRET*****")
    }
}

impl Secret {
    #[must_use]
    pub const fn new(secret: String) -> Self {
        Self {
            secret: Some(secret),
            is_random: false,
        }
    }

    #[cfg(feature = "server")]
    #[must_use]
    pub fn random() -> Self {
        let secret = rand::Rng::sample_iter(rand::thread_rng(), &rand::distributions::Alphanumeric)
            .take(30)
            .map(char::from)
            .collect();
        Self {
            secret: Some(secret),
            is_random: true,
        }
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("*****SECRET*****")
    }
}

impl std::fmt::Display for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("*****SECRET*****")
    }
}
