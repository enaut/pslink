use std::ops::Deref;

use serde::{Deserialize, Serialize, Serializer};
use strum_macros::{AsRefStr, EnumIter, EnumString, ToString};
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

#[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub password: Secret,
    pub role: i64,
    pub language: Lang,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Link {
    pub id: i64,
    pub title: String,
    pub target: String,
    pub code: String,
    pub author: i64,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Count {
    pub number: i32,
}
#[derive(Serialize, Debug)]
pub struct Click {
    pub id: i64,
    pub link: i64,
    pub created_at: chrono::NaiveDateTime,
}

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
