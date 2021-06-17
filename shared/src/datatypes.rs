//! The more generic datatypes used in pslink
use std::ops::Deref;

use serde::{Deserialize, Serialize, Serializer};
use strum_macros::{AsRefStr, EnumIter, EnumString, ToString};

use crate::apirequests::users::Role;
/// A generic list returntype containing the User and a Vec containing e.g. Links or Users
#[derive(Clone, Deserialize, Serialize)]
pub struct ListWithOwner<T> {
    pub user: User,
    pub list: Vec<T>,
}

/// A link together with its author and its click-count.
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct FullLink {
    pub link: Link,
    pub user: User,
    pub clicks: Count,
}

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
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Link {
    pub id: i64,
    pub title: String,
    pub target: String,
    pub code: String,
    pub author: i64,
    pub created_at: chrono::NaiveDateTime,
}

/// When statistics are counted
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Count {
    pub number: i32,
}

/// Everytime a shor url is clicked record it for statistical evaluation.
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
}

impl From<String> for Secret {
    fn from(_: String) -> Self {
        Self { secret: None }
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
/// To add an additional language add it to this enum aswell as an appropriate file into the locales folder.
#[allow(clippy::upper_case_acronyms)]
#[derive(
    Debug,
    Copy,
    Clone,
    EnumIter,
    EnumString,
    ToString,
    AsRefStr,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
)]
pub enum Lang {
    #[strum(serialize = "en-US", serialize = "en", serialize = "enUS")]
    EnUS,
    #[strum(serialize = "de-DE", serialize = "de", serialize = "deDE")]
    DeDE,
}
