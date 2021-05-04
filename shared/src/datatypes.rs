use serde::{Deserialize, Serialize};
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
    pub password: String,
    pub role: i64,
    pub language: String,
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
