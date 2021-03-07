use crate::{forms::LinkForm, ServerConfig, ServerError};

use super::schema::{clicks, links, users};
use argonautica::Hasher;
use diesel::{Identifiable, Insertable, Queryable};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};

#[derive(Identifiable, Queryable, PartialEq, Serialize, Clone, Debug)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub password: String,
    pub role: i32,
}

#[derive(Debug, Deserialize, Insertable)]
#[table_name = "users"]
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub password: String,
}

impl NewUser {
    pub(crate) fn new(
        username: String,
        email: String,
        password: &str,
        config: &ServerConfig,
    ) -> Result<Self, ServerError> {
        let hash = Self::hash_password(password, config)?;

        Ok(Self {
            username,
            email,
            password: hash,
        })
    }

    pub(crate) fn hash_password(
        password: &str,
        config: &ServerConfig,
    ) -> Result<String, ServerError> {
        dotenv().ok();

        let secret = &config.secret;

        let hash = Hasher::default()
            .with_password(password)
            .with_secret_key(secret)
            .hash()?;

        Ok(hash)
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginUser {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Debug, Queryable)]
pub struct Link {
    pub id: i32,
    pub title: String,
    pub target: String,
    pub code: String,
    pub author: i32,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Serialize, Insertable)]
#[table_name = "links"]
pub struct NewLink {
    pub title: String,
    pub target: String,
    pub code: String,
    pub author: i32,
    pub created_at: chrono::NaiveDateTime,
}

impl NewLink {
    pub(crate) fn from_link_form(form: LinkForm, uid: i32) -> Self {
        Self {
            title: form.title,
            target: form.target,
            code: form.code,
            author: uid,
            created_at: chrono::Local::now().naive_utc(),
        }
    }
}

#[derive(Serialize, Debug, Queryable)]
pub struct Click {
    pub id: i32,
    pub link: i32,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Serialize, Insertable)]
#[table_name = "clicks"]
pub struct NewClick {
    pub link: i32,
    pub created_at: chrono::NaiveDateTime,
}

impl NewClick {
    #[must_use]
    pub fn new(link_id: i32) -> Self {
        Self {
            link: link_id,
            created_at: chrono::Local::now().naive_utc(),
        }
    }
}

#[derive(Serialize, Debug, Queryable)]
pub struct Count {
    count: i32,
}
