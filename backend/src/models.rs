use std::str::FromStr;

use dioxus::{
    logger::tracing::{error, info},
    prelude::ServerFnError,
};
use serde::{Deserialize, Serialize};

use argon2::PasswordHasher as _;
use pslink_shared::{
    apirequests::{links::LinkDelta, users::Role},
    datatypes::{Count, Lang, Link, Secret, Statistics, User, WeekCount},
};
use sqlx::Row;

/// The operations a User should support.

#[allow(dead_code)]
pub trait UserDbOperations<T> {
    async fn get_user(id: i64) -> Result<T, ServerFnError>;
    async fn get_user_by_name(name: &str) -> Result<T, ServerFnError>;
    async fn get_all_users() -> Result<Vec<T>, ServerFnError>;
    async fn update_user(&self) -> Result<(), ServerFnError>;
    async fn toggle_admin(self) -> Result<(), ServerFnError>;
    async fn set_language(self, new_language: Lang) -> Result<(), ServerFnError>;
    async fn count_admins() -> Result<Count, ServerFnError>;
}

impl UserDbOperations<Self> for User {
    /// get a user by its id
    ///
    /// # Errors
    /// fails with [`ServerFnError`] if the user does not exist or the database cannot be acessed.

    async fn get_user(id: i64) -> Result<Self, ServerFnError> {
        let db = crate::get_db().await;
        let user = sqlx::query!("Select * from users where id = ? ", id)
            .fetch_one(&db)
            .await
            .map(|row| Self {
                id: row.id,
                username: row.username,
                email: row.email,
                password: Secret::new(row.password),
                role: Role::convert(row.role),
                language: Lang::from_str(&row.language).expect("Should parse"),
            });
        user.map_err(|e| e.into())
    }

    /// get a user by its username
    ///
    /// # Errors
    /// fails with [`ServerFnError`] if the user does not exist or the database cannot be acessed.

    async fn get_user_by_name(name: &str) -> Result<Self, ServerFnError> {
        let db = crate::get_db().await;
        let user = sqlx::query!("Select * from users where username = ? ", name)
            .fetch_one(&db)
            .await
            .map(|row| Self {
                id: row.id,
                username: row.username,
                email: row.email,
                password: Secret::new(row.password),
                role: Role::convert(row.role),
                language: Lang::from_str(&row.language).expect("Should parse"),
            });
        user.map_err(|e| e.into())
    }

    /// get all users
    ///
    /// # Errors
    /// fails with [`ServerFnError`] if the database cannot be acessed.

    async fn get_all_users() -> Result<Vec<Self>, ServerFnError> {
        let db = crate::get_db().await;
        let user = sqlx::query("Select * from users")
            .fetch_all(&db)
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
        user.map_err(|e| e.into())
    }

    /// change a user
    ///
    /// # Errors
    /// fails with [`ServerFnError`] if the user does not exist, some constraints are not satisfied or the database cannot be acessed.

    async fn update_user(&self) -> Result<(), ServerFnError> {
        let db = crate::get_db().await;
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
        .execute(&db)
        .await?;
        Ok(())
    }

    /// Change an admin user to normal user and a normal user to admin
    ///
    /// # Errors
    /// fails with [`ServerFnError`] if the database cannot be acessed. (the user should exist)

    async fn toggle_admin(self) -> Result<(), ServerFnError> {
        let db = crate::get_db().await;
        let new_role = match self.role {
            r @ Role::NotAuthenticated | r @ Role::Disabled => r,
            Role::Regular => Role::Admin,
            Role::Admin => Role::Regular,
        };
        let role_i64 = new_role.to_i64();
        sqlx::query!("UPDATE users SET role = ? where id = ?", role_i64, self.id)
            .execute(&db)
            .await?;
        Ok(())
    }

    /// set the language setting of a user
    ///
    /// # Errors
    /// fails with [`ServerFnError`] if the user does not exist or the database cannot be acessed.

    async fn set_language(self, new_language: Lang) -> Result<(), ServerFnError> {
        let db = crate::get_db().await;
        let lang_code = new_language.to_string();
        sqlx::query!(
            "UPDATE users SET language = ? where id = ?",
            lang_code,
            self.id
        )
        .execute(&db)
        .await?;
        Ok(())
    }

    /// Count the admin accounts
    ///
    /// this is usefull for determining if any admins exist at all.
    ///
    /// # Errors
    /// fails with [`ServerFnError`] if the database cannot be acessed.

    async fn count_admins() -> Result<Count, ServerFnError> {
        let db = crate::get_db().await;
        let num = sqlx::query_as!(Count, "select count(*) as number from users where role = 2")
            .fetch_one(&db)
            .await?;
        Ok(num)
    }
}

/// Relevant parameters when creating a new user
/// Use the [`NewUser::new`] constructor to store the password encrypted. Otherwise it will not work.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub password: String,
}
#[allow(dead_code)]
impl NewUser {
    /// Create a new user that can then be inserted in the database
    ///
    /// The password is encrypted using the secret before creating.
    ///
    /// # Errors
    /// fails with [`ServerFnError`] if the password could not be encrypted.

    pub fn new(
        username: String,
        email: String,
        password: &str,
        secret: &Secret,
    ) -> Result<Self, ServerFnError> {
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

    pub(crate) fn hash_password(password: &str, secret: &Secret) -> Result<String, ServerFnError> {
        let secret = secret.secret.as_ref().unwrap().clone().into_bytes();
        let argon2 = argon2::Argon2::new_with_secret(
            &secret,
            argon2::Algorithm::default(),
            argon2::Version::default(),
            argon2::Params::default(),
        )
        .expect("Failed to create argon2 hasher");
        let salt = argon2::password_hash::SaltString::generate(&mut rand::rngs::OsRng);
        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| ServerFnError::new(format!("Password error: {}", e)))?;

        Ok(hash.to_string())
    }

    /// Insert this user into the database
    ///
    /// # Errors
    /// fails with [`ServerFnError`] if the database cannot be acessed.

    pub async fn insert_user(&self) -> Result<(), ServerFnError> {
        let db = crate::get_db().await;
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
        .execute(&db)
        .await?;
        Ok(())
    }
}

/// Operations that should be supported by links
#[allow(dead_code)]
pub trait LinkDbOperations<T> {
    async fn get_link_by_code(code: &str) -> Result<T, ServerFnError>;
    async fn get_link_by_id(id: i64) -> Result<T, ServerFnError>;
    async fn delete_link_by_code(code: &str) -> Result<(), ServerFnError>;
    async fn update_link(&self) -> Result<(), ServerFnError>;
    async fn get_statistics(code: i64) -> Result<Statistics, ServerFnError>;
}

impl LinkDbOperations<Self> for Link {
    /// Get a link statistics by its id
    ///
    /// # Errors
    /// fails with [`ServerFnError`] if the database cannot be acessed or the link is not found.

    async fn get_statistics(link_id: i64) -> Result<Statistics, ServerFnError> {
        let db = crate::get_db().await;
        // Verify that the code exists to avoid injections in the next query
        let code = sqlx::query!("select code from links where id=?", link_id)
            .fetch_one(&db)
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
            .fetch_all(&db)
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
        ).fetch_one(&db).await?;
        info!("Found Statistics: {:?}", &values);
        Ok(Statistics {
            link_id,
            total,
            values,
        })
    }
    /// Get a link by its code (the short url code)
    ///
    /// # Errors
    /// fails with [`ServerFnError`] if the database cannot be acessed or the link is not found.

    async fn get_link_by_code(code: &str) -> Result<Self, ServerFnError> {
        let db = crate::get_db().await;
        let link = sqlx::query_as!(
            Self,
            "Select * from links where code = ? COLLATE NOCASE",
            code
        )
        .fetch_one(&db)
        .await;
        info!("Found link: {:?}", &link);
        link.map_err(|e| e.into())
    }

    /// Get a link by its id
    ///
    /// # Errors
    /// fails with [`ServerFnError`] if the database cannot be acessed or the link is not found.

    async fn get_link_by_id(id: i64) -> Result<Self, ServerFnError> {
        let db = crate::get_db().await;
        let link = sqlx::query_as!(Self, "Select * from links where id = ? ", id)
            .fetch_one(&db)
            .await;
        info!("Found link: {:?}", &link);
        link.map_err(|e| e.into())
    }

    /// Delete a link by its code
    ///
    /// # Errors
    /// fails with [`ServerFnError`] if the database cannot be acessed or the link is not found.

    async fn delete_link_by_code(code: &str) -> Result<(), ServerFnError> {
        let db = crate::get_db().await;
        sqlx::query!("DELETE from links where code = ? ", code)
            .execute(&db)
            .await?;
        Ok(())
    }

    /// Update a link with new values, carful when changing the code the old link becomes invalid.
    /// This could be a problem when it is printed or published somewhere.
    ///
    /// # Errors
    /// fails with [`ServerFnError`] if the database cannot be acessed or the link is not found.

    async fn update_link(&self) -> Result<(), ServerFnError> {
        let db = crate::get_db().await;
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
        match qry.execute(&db).await {
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
    /// fails with [`ServerFnError`] if the database cannot be acessed or constraints are not met.
    pub async fn insert(self) -> Result<(), ServerFnError> {
        let db = crate::get_db().await;
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
        .execute(&db)
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

#[allow(dead_code)]
impl NewClick {
    #[must_use]
    pub fn new(link_id: i64) -> Self {
        Self {
            link: link_id,
            created_at: chrono::Local::now().naive_utc(),
        }
    }

    pub(crate) async fn insert_click(self) -> Result<(), ServerFnError> {
        let db = crate::get_db().await;
        sqlx::query!(
            "Insert into clicks (
            link,
            created_at) VALUES (?,?)",
            self.link,
            self.created_at,
        )
        .execute(&db)
        .await?;
        Ok(())
    }
}
