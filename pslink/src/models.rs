use crate::{forms::LinkForm, Secret, ServerConfig, ServerError};

use argonautica::Hasher;
use async_trait::async_trait;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};

use shared::{
    apirequests::links::LinkDelta,
    datatypes::{Count, Link, User},
};
use sqlx::Row;
use tracing::{error, info, instrument};

#[async_trait]
pub trait UserDbOperations<T> {
    async fn get_user(id: i64, server_config: &ServerConfig) -> Result<T, ServerError>;
    async fn get_user_by_name(name: &str, server_config: &ServerConfig) -> Result<T, ServerError>;
    async fn get_all_users(server_config: &ServerConfig) -> Result<Vec<T>, ServerError>;
    async fn update_user(&self, server_config: &ServerConfig) -> Result<(), ServerError>;
    async fn toggle_admin(self, server_config: &ServerConfig) -> Result<(), ServerError>;
    async fn set_language(
        self,
        server_config: &ServerConfig,
        new_language: &str,
    ) -> Result<(), ServerError>;
    async fn count_admins(server_config: &ServerConfig) -> Result<Count, ServerError>;
}

#[async_trait]
impl UserDbOperations<Self> for User {
    #[instrument()]
    async fn get_user(id: i64, server_config: &ServerConfig) -> Result<Self, ServerError> {
        let user = sqlx::query!("Select * from users where id = ? ", id)
            .fetch_one(&server_config.db_pool)
            .await
            .map(|row| Self {
                id: row.id,
                username: row.username,
                email: row.email,
                password: Secret::new(row.password),
                role: row.role,
                language: row.language,
            });
        user.map_err(ServerError::Database)
    }

    /// get a user by its username
    ///
    /// # Errors
    /// fails with [`ServerError`] if the user does not exist or the database cannot be acessed.
    #[instrument()]
    async fn get_user_by_name(
        name: &str,
        server_config: &ServerConfig,
    ) -> Result<Self, ServerError> {
        let user = sqlx::query!("Select * from users where username = ? ", name)
            .fetch_one(&server_config.db_pool)
            .await
            .map(|row| Self {
                id: row.id,
                username: row.username,
                email: row.email,
                password: Secret::new(row.password),
                role: row.role,
                language: row.language,
            });
        user.map_err(ServerError::Database)
    }

    #[instrument()]
    async fn get_all_users(server_config: &ServerConfig) -> Result<Vec<Self>, ServerError> {
        let user = sqlx::query("Select * from users")
            .fetch_all(&server_config.db_pool)
            .await
            .map(|row| {
                row.into_iter()
                    .map(|r| Self {
                        id: r.get("id"),
                        username: r.get("username"),
                        email: r.get("email"),
                        password: Secret::new(r.get("password")),
                        role: r.get("role"),
                        language: r.get("language"),
                    })
                    .collect()
            });
        user.map_err(ServerError::Database)
    }

    #[instrument()]
    async fn update_user(&self, server_config: &ServerConfig) -> Result<(), ServerError> {
        sqlx::query!(
            "UPDATE users SET
            username = ?,
            email = ?,
            password = ?,
            role = ? where id = ?",
            self.username,
            self.email,
            self.password.secret,
            self.role,
            self.id
        )
        .execute(&server_config.db_pool)
        .await?;
        Ok(())
    }

    /// Change an admin user to normal user and a normal user to admin
    ///
    /// # Errors
    /// fails with [`ServerError`] if the database cannot be acessed. (the user should exist)
    #[instrument()]
    async fn toggle_admin(self, server_config: &ServerConfig) -> Result<(), ServerError> {
        let new_role = 2 - (self.role + 1) % 2;
        sqlx::query!("UPDATE users SET role = ? where id = ?", new_role, self.id)
            .execute(&server_config.db_pool)
            .await?;
        Ok(())
    }

    #[instrument()]
    async fn set_language(
        self,
        server_config: &ServerConfig,
        new_language: &str,
    ) -> Result<(), ServerError> {
        sqlx::query!(
            "UPDATE users SET language = ? where id = ?",
            new_language,
            self.id
        )
        .execute(&server_config.db_pool)
        .await?;
        Ok(())
    }

    /// Count the admin accounts
    ///
    /// this is usefull for determining if any admins exist at all.
    ///
    /// # Errors
    /// fails with [`ServerError`] if the database cannot be acessed.
    #[instrument()]
    async fn count_admins(server_config: &ServerConfig) -> Result<Count, ServerError> {
        let num = sqlx::query_as!(Count, "select count(*) as number from users where role = 2")
            .fetch_one(&server_config.db_pool)
            .await?;
        Ok(num)
    }
}

#[derive(Debug, Deserialize)]
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub password: String,
}

impl NewUser {
    /// Create a new user that can then be inserted in the database
    ///
    /// # Errors
    /// fails with [`ServerError`] if the password could not be encrypted.
    #[instrument()]
    pub fn new(
        username: String,
        email: String,
        password: &str,
        secret: &Secret,
    ) -> Result<Self, ServerError> {
        let hash = Self::hash_password(password, secret)?;

        Ok(Self {
            username,
            email,
            password: hash,
        })
    }

    #[instrument()]
    pub(crate) fn hash_password(password: &str, secret: &Secret) -> Result<String, ServerError> {
        dotenv().ok();

        let hash = Hasher::default()
            .with_password(password)
            .with_secret_key(secret.secret.as_ref().expect("A secret key was not given"))
            .hash()?;

        Ok(hash)
    }

    /// Insert this user into the database
    ///
    /// # Errors
    /// fails with [`ServerError`] if the database cannot be acessed.
    #[instrument()]
    pub async fn insert_user(&self, server_config: &ServerConfig) -> Result<(), ServerError> {
        sqlx::query!(
            "Insert into users (
            username,
            email,
            password,
            role) VALUES (?,?,?,1)",
            self.username,
            self.email,
            self.password,
        )
        .execute(&server_config.db_pool)
        .await?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginUser {
    pub username: String,
    pub password: String,
}

#[async_trait]
pub trait LinkDbOperations<T> {
    async fn get_link_by_code(code: &str, server_config: &ServerConfig) -> Result<T, ServerError>;
    async fn get_link_by_id(id: i64, server_config: &ServerConfig) -> Result<T, ServerError>;
    async fn delete_link_by_code(
        code: &str,
        server_config: &ServerConfig,
    ) -> Result<(), ServerError>;
    async fn update_link(&self, server_config: &ServerConfig) -> Result<(), ServerError>;
}

#[async_trait]
impl LinkDbOperations<Self> for Link {
    #[instrument()]
    async fn get_link_by_code(
        code: &str,
        server_config: &ServerConfig,
    ) -> Result<Self, ServerError> {
        let link = sqlx::query_as!(Self, "Select * from links where code = ? ", code)
            .fetch_one(&server_config.db_pool)
            .await;
        tracing::info!("Found link: {:?}", &link);
        link.map_err(ServerError::Database)
    }
    #[instrument()]
    async fn get_link_by_id(id: i64, server_config: &ServerConfig) -> Result<Self, ServerError> {
        let link = sqlx::query_as!(Self, "Select * from links where id = ? ", id)
            .fetch_one(&server_config.db_pool)
            .await;
        tracing::info!("Found link: {:?}", &link);
        link.map_err(ServerError::Database)
    }

    #[instrument()]
    async fn delete_link_by_code(
        code: &str,
        server_config: &ServerConfig,
    ) -> Result<(), ServerError> {
        sqlx::query!("DELETE from links where code = ? ", code)
            .execute(&server_config.db_pool)
            .await?;
        Ok(())
    }

    #[instrument()]
    async fn update_link(&self, server_config: &ServerConfig) -> Result<(), ServerError> {
        info!("{:?}", self);
        let qry = sqlx::query!(
            "UPDATE links SET
            title = ?,
            target = ?,
            code = ?,
            author = ?,
            created_at = ? where id = ?",
            self.title,
            self.target,
            self.code,
            self.author,
            self.created_at,
            self.id
        );
        match qry.execute(&server_config.db_pool).await {
            Ok(_) => Ok(()),
            Err(e) => {
                //error!("{}", qry);
                error!("{}", e);
                Err(e.into())
            }
        }
    }
}

#[derive(Serialize, Debug)]
pub struct NewLink {
    pub title: String,
    pub target: String,
    pub code: String,
    pub author: i64,
    pub created_at: chrono::NaiveDateTime,
}

impl NewLink {
    pub(crate) fn from_link_form(form: LinkForm, uid: i64) -> Self {
        Self {
            title: form.title,
            target: form.target,
            code: form.code,
            author: uid,
            created_at: chrono::Local::now().naive_utc(),
        }
    }
    pub(crate) fn from_link_delta(link: LinkDelta, uid: i64) -> Self {
        Self {
            title: link.title,
            target: link.target,
            code: link.code,
            author: uid,
            created_at: chrono::Local::now().naive_utc(),
        }
    }

    pub(crate) async fn insert(self, server_config: &ServerConfig) -> Result<(), ServerError> {
        sqlx::query!(
            "Insert into links (
                title,
                target,
                code,
                author,
                created_at) VALUES (?,?,?,?,?)",
            self.title,
            self.target,
            self.code,
            self.author,
            self.created_at,
        )
        .execute(&server_config.db_pool)
        .await?;
        Ok(())
    }
}

#[derive(Serialize)]
pub struct NewClick {
    pub link: i64,
    pub created_at: chrono::NaiveDateTime,
}

impl NewClick {
    #[must_use]
    pub fn new(link_id: i64) -> Self {
        Self {
            link: link_id,
            created_at: chrono::Local::now().naive_utc(),
        }
    }

    pub(crate) async fn insert_click(
        self,
        server_config: &ServerConfig,
    ) -> Result<(), ServerError> {
        sqlx::query!(
            "Insert into clicks (
            link,
            created_at) VALUES (?,?)",
            self.link,
            self.created_at,
        )
        .execute(&server_config.db_pool)
        .await?;
        Ok(())
    }
}
