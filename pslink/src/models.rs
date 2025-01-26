use std::str::FromStr;

use crate::{ServerConfig, ServerError};

use async_trait::async_trait;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};

use argon2::PasswordHasher as _;
use pslink_shared::{
    apirequests::{links::LinkDelta, users::Role},
    datatypes::{Count, Lang, Link, Secret, Statistics, User, WeekCount},
};
use sqlx::Row;
use tracing::{error, info, instrument};

/// The operations a User should support.
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
        new_language: Lang,
    ) -> Result<(), ServerError>;
    async fn count_admins(server_config: &ServerConfig) -> Result<Count, ServerError>;
}

#[async_trait]
impl UserDbOperations<Self> for User {
    /// get a user by its id
    ///
    /// # Errors
    /// fails with [`ServerError`] if the user does not exist or the database cannot be acessed.
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
                role: Role::convert(row.role),
                language: Lang::from_str(&row.language).expect("Should parse"),
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
                role: Role::convert(row.role),
                language: Lang::from_str(&row.language).expect("Should parse"),
            });
        user.map_err(ServerError::Database)
    }

    /// get all users
    ///
    /// # Errors
    /// fails with [`ServerError`] if the database cannot be acessed.
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
                        role: Role::convert(r.get("role")),
                        language: Lang::from_str(r.get("language"))
                            .expect("should parse correctly"),
                    })
                    .collect()
            });
        user.map_err(ServerError::Database)
    }

    /// change a user
    ///
    /// # Errors
    /// fails with [`ServerError`] if the user does not exist, some constraints are not satisfied or the database cannot be acessed.
    #[instrument()]
    async fn update_user(&self, server_config: &ServerConfig) -> Result<(), ServerError> {
        let role_i64 = self.role.to_i64();
        sqlx::query!(
            "UPDATE users SET
            username = ?,
            email = ?,
            password = ?,
            role = ? where id = ?",
            self.username,
            self.email,
            self.password.secret,
            role_i64,
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
        let new_role = match self.role {
            r @ Role::NotAuthenticated | r @ Role::Disabled => r,
            Role::Regular => Role::Admin,
            Role::Admin => Role::Regular,
        };
        let role_i64 = new_role.to_i64();
        sqlx::query!("UPDATE users SET role = ? where id = ?", role_i64, self.id)
            .execute(&server_config.db_pool)
            .await?;
        Ok(())
    }

    /// set the language setting of a user
    ///
    /// # Errors
    /// fails with [`ServerError`] if the user does not exist or the database cannot be acessed.
    #[instrument()]
    async fn set_language(
        self,
        server_config: &ServerConfig,
        new_language: Lang,
    ) -> Result<(), ServerError> {
        let lang_code = new_language.to_string();
        sqlx::query!(
            "UPDATE users SET language = ? where id = ?",
            lang_code,
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

/// Relevant parameters when creating a new user
/// Use the [`NewUser::new`] constructor to store the password encrypted. Otherwise it will not work.
#[derive(Debug, Deserialize)]
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub password: String,
}

impl NewUser {
    /// Create a new user that can then be inserted in the database
    ///
    /// The password is encrypted using the secret before creating.
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

    /// encrypt the password.
    ///
    /// This function uses the Secret from the config settings to encrypt the password
    #[instrument()]
    pub(crate) fn hash_password(password: &str, secret: &Secret) -> Result<String, ServerError> {
        dotenv().ok();
        let secret = secret.secret.as_ref().unwrap().clone().into_bytes();
        let argon2 = argon2::Argon2::new_with_secret(
            &secret,
            argon2::Algorithm::default(),
            argon2::Version::default(),
            argon2::Params::default(),
        )
        .expect("Failed to create argon2 hasher");
        let salt = argon2::password_hash::SaltString::generate(&mut rand::rngs::OsRng);
        let hash = argon2.hash_password(password.as_bytes(), &salt)?;

        Ok(hash.to_string())
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

/// Operations that should be supported by links
#[async_trait]
pub trait LinkDbOperations<T> {
    async fn get_link_by_code(code: &str, server_config: &ServerConfig) -> Result<T, ServerError>;
    async fn get_link_by_id(id: i64, server_config: &ServerConfig) -> Result<T, ServerError>;
    async fn delete_link_by_code(
        code: &str,
        server_config: &ServerConfig,
    ) -> Result<(), ServerError>;
    async fn update_link(&self, server_config: &ServerConfig) -> Result<(), ServerError>;
    async fn get_statistics(
        code: i64,
        server_config: &ServerConfig,
    ) -> Result<Statistics, ServerError>;
}

#[async_trait]
impl LinkDbOperations<Self> for Link {
    /// Get a link statistics by its id
    ///
    /// # Errors
    /// fails with [`ServerError`] if the database cannot be acessed or the link is not found.
    #[instrument()]
    async fn get_statistics(
        link_id: i64,
        server_config: &ServerConfig,
    ) -> Result<Statistics, ServerError> {
        // Verify that the code exists to avoid injections in the next query
        let code = sqlx::query!("select code from links where id=?", link_id)
            .fetch_one(&server_config.db_pool)
            .await?
            .code;
        // The query to get the statistics carefully check code before to avoid injections.
        let qry = format!(
            r#"SELECT created_at AS month,
    cast(strftime('%W', created_at) AS String) AS week,
    count(*) AS total
FROM clicks
WHERE month > date('now', 'start of month', '-1 year')
    AND link = '{}'
GROUP BY week
ORDER BY month"#,
            link_id
        );
        // Execute and map the query to the desired type
        let values: Vec<WeekCount> = sqlx::query(&qry)
            .fetch_all(&server_config.db_pool)
            .await?
            .into_iter()
            .map(|c| WeekCount {
                month: c.get("month"),
                total: Count {
                    number: c.get("total"),
                },
                week: c.get("week"),
            })
            .collect();
        let total = sqlx::query_as!(
            Count,
            "select count(*) as number from clicks join links on clicks.link = links.id where links.code = ?",
            code
        ).fetch_one(&server_config.db_pool).await?;
        tracing::info!("Found Statistics: {:?}", &values);
        Ok(Statistics {
            link_id,
            total,
            values,
        })
    }
    /// Get a link by its code (the short url code)
    ///
    /// # Errors
    /// fails with [`ServerError`] if the database cannot be acessed or the link is not found.
    #[instrument()]
    async fn get_link_by_code(
        code: &str,
        server_config: &ServerConfig,
    ) -> Result<Self, ServerError> {
        let link = sqlx::query_as!(
            Self,
            "Select * from links where code = ? COLLATE NOCASE",
            code
        )
        .fetch_one(&server_config.db_pool)
        .await;
        tracing::info!("Found link: {:?}", &link);
        link.map_err(ServerError::Database)
    }

    /// Get a link by its id
    ///
    /// # Errors
    /// fails with [`ServerError`] if the database cannot be acessed or the link is not found.
    #[instrument()]
    async fn get_link_by_id(id: i64, server_config: &ServerConfig) -> Result<Self, ServerError> {
        let link = sqlx::query_as!(Self, "Select * from links where id = ? ", id)
            .fetch_one(&server_config.db_pool)
            .await;
        tracing::info!("Found link: {:?}", &link);
        link.map_err(ServerError::Database)
    }

    /// Delete a link by its code
    ///
    /// # Errors
    /// fails with [`ServerError`] if the database cannot be acessed or the link is not found.
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

    /// Update a link with new values, carful when changing the code the old link becomes invalid.
    /// This could be a problem when it is printed or published somewhere.
    ///
    /// # Errors
    /// fails with [`ServerError`] if the database cannot be acessed or the link is not found.
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

/// Relevant parameters when creating a new link.
#[derive(Serialize, Debug)]
pub struct NewLink {
    pub title: String,
    pub target: String,
    pub code: String,
    pub author: i64,
    pub created_at: chrono::NaiveDateTime,
}

impl NewLink {
    /// Take a [`LinkDelta`] and create a [`NewLink`] instance. `created_at` is populated with the current time.
    pub(crate) fn from_link_delta(link: LinkDelta, uid: i64) -> Self {
        Self {
            title: link.title,
            target: link.target,
            code: link.code,
            author: uid,
            created_at: chrono::Local::now().naive_utc(),
        }
    }

    /// Insert the new link into the database
    ///
    /// # Errors
    /// fails with [`ServerError`] if the database cannot be acessed or constraints are not met.
    pub async fn insert(self, server_config: &ServerConfig) -> Result<(), ServerError> {
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

/// Whenever a link is clicked the click is registered for statistical purposes.
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
