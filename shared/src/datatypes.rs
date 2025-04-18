//! The more generic data-types used in pslink
use std::ops::Deref;

use crate::apirequests::users::Role;
use serde::{Deserialize, Serialize, Serializer};

use strum_macros::{AsRefStr, EnumIter, EnumString};

/// A generic list return type containing the User and a Vec containing e.g. Links or Users
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct ListWithOwner<T> {
    pub user: User,
    pub list: Vec<T>,
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
            (Clicks::Count(l0), Clicks::Extended(r0)) => l0.number == r0.total.number,
            (Clicks::Extended(l0), Clicks::Count(r0)) => l0.total.number == r0.number,
        }
    }
}

impl PartialOrd for Clicks {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::Count(l0), Self::Count(r0)) => l0.number.partial_cmp(&r0.number),
            (Self::Extended(l0), Self::Extended(r0)) => {
                l0.total.number.partial_cmp(&r0.total.number)
            }
            (Clicks::Count(l0), Clicks::Extended(r0)) => l0.number.partial_cmp(&r0.total.number),
            (Clicks::Extended(l0), Clicks::Count(r0)) => l0.total.number.partial_cmp(&r0.number),
        }
    }
}

impl Ord for Clicks {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Count(l0), Self::Count(r0)) => l0.number.cmp(&r0.number),
            (Self::Extended(l0), Self::Extended(r0)) => l0.total.number.cmp(&r0.total.number),
            (Clicks::Count(l0), Clicks::Extended(r0)) => l0.number.cmp(&r0.total.number),
            (Clicks::Extended(l0), Clicks::Count(r0)) => l0.total.number.cmp(&r0.number),
        }
    }
}

impl Eq for Clicks {}

/// A User of the pslink service
#[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub password: Secret,
    pub role: Role,
    pub language: Lang,
}

/// A short url of the link service
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Link {
    pub id: i64,
    pub title: String,
    pub target: String,
    pub code: String,
    pub author: i64,
    pub created_at: chrono::NaiveDateTime,
}

/// When statistics are counted
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, PartialOrd)]
pub struct Count {
    pub number: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WeekCount {
    pub full_date: chrono::NaiveDateTime,
    pub total: Count,
    pub week: i32,
}
impl Eq for WeekCount {}

impl PartialOrd for WeekCount {
    fn partial_cmp(&self, other: &Self) -> std::option::Option<std::cmp::Ordering> {
        self.total.number.partial_cmp(&other.total.number)
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

/// Every time a short url is clicked record it for statistical evaluation.
#[derive(Serialize, Debug)]
pub struct Click {
    pub id: i64,
    pub link: i64,
    pub created_at: chrono::NaiveDateTime,
}

/// The Password: Display, Debug and serialize do not include the Password to prevent leaks of sensible information in logs or similar.
#[derive(PartialEq, Clone, Deserialize)]
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

/// Loadable is a type that has not been loaded but will be in future. It can be used to indicate the loading process.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Loadable<T> {
    Data(Option<T>),
    Loading,
}

impl<T> Deref for Loadable<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        match self {
            Loadable::Data(t) => t,
            Loadable::Loading => &None,
        }
    }
}

/// An `enum` containing the available languages.
/// To add an additional language add it to this enum as well as an appropriate file into the locales folder.
#[allow(clippy::upper_case_acronyms)]
#[derive(
    Debug, Copy, Clone, EnumIter, EnumString, AsRefStr, Eq, PartialEq, Serialize, Deserialize,
)]
#[strum(ascii_case_insensitive)]
pub enum Lang {
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

/// A generic returntype containing the User and a single item
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Item<T> {
    pub user: User,
    pub item: T,
}
