//! Database access layer using SQLx for users, links, and click statistics.

use std::str::FromStr;

use argon2::{PasswordHasher as _, PasswordVerifier as _};
use chrono::{NaiveDate, NaiveDateTime};
use dioxus::{
    logger::tracing::{error, info},
    prelude::ServerFnError,
};
use serde::{Deserialize, Serialize};
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool};

use crate::{
    domain::{
        models::{Clicks, Count, FullLink, Lang, Link, Role, Secret, Statistics, User, WeekCount},
        requests::{
            LinkDelta, LinkOverviewColumns, LinkRequestForm, Ordering, UserOverviewColumns,
            UserRequestForm,
        },
    },
    server::ServerResultExt,
};

/// Get a user by their database ID.
///
/// # Errors
/// Fails with [`ServerFnError`] if the user does not exist or the database cannot be accessed.
pub async fn get_user(pool: &SqlitePool, id: i64) -> Result<User, ServerFnError> {
    let row = sqlx::query!("Select * from users where id = ? ", id)
        .fetch_one(pool)
        .await
        .sfe()?;
    Ok(User {
        id: row.id,
        username: row.username,
        email: row.email,
        role: Role::convert(row.role),
        language: Lang::from_str(&row.language).unwrap_or_default(),
    })
}

/// Get a user by their username.
///
/// # Errors
/// Fails with [`ServerFnError`] if the user does not exist or the database cannot be accessed.
pub async fn get_user_by_name(pool: &SqlitePool, name: &str) -> Result<User, ServerFnError> {
    let (user, _) = get_user_with_password_by_name(pool, name).await?;
    Ok(user)
}

/// Get a user along with their stored password hash by username.
///
/// # Errors
/// Fails with [`ServerFnError`] if the user does not exist or the database cannot be accessed.
pub async fn get_user_with_password_by_name(
    pool: &SqlitePool,
    name: &str,
) -> Result<(User, String), ServerFnError> {
    let row = sqlx::query!("Select * from users where username = ? ", name)
        .fetch_one(pool)
        .await
        .sfe()?;
    let user = User {
        id: row.id,
        username: row.username,
        email: row.email,
        role: Role::convert(row.role),
        language: Lang::from_str(&row.language).unwrap_or_default(),
    };
    Ok((user, row.password))
}

/// Get a user along with their stored password hash by user ID.
///
/// # Errors
/// Fails with [`ServerFnError`] if the user does not exist or the database cannot be accessed.
pub async fn get_user_with_password(
    pool: &SqlitePool,
    id: i64,
) -> Result<(User, String), ServerFnError> {
    let row = sqlx::query!("Select * from users where id = ? ", id)
        .fetch_one(pool)
        .await
        .sfe()?;
    let user = User {
        id: row.id,
        username: row.username,
        email: row.email,
        role: Role::convert(row.role),
        language: Lang::from_str(&row.language).unwrap_or_default(),
    };
    Ok((user, row.password))
}

/// Query a filtered, ordered, and paginated list of users using parameterized bindings.
///
/// # Errors
/// Fails with [`ServerFnError`] if the database query fails.
pub async fn list_users_filtered(
    pool: &SqlitePool,
    params: &UserRequestForm,
) -> Result<Vec<User>, ServerFnError> {
    let mut qb: QueryBuilder<Sqlite> =
        QueryBuilder::new("SELECT id, username, email, role, language FROM users");

    let active_filters: Vec<(UserOverviewColumns, &str)> = params
        .filter
        .iter()
        .filter_map(|(col, filter)| {
            let trimmed = filter.sieve.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some((col, trimmed))
            }
        })
        .collect();

    if !active_filters.is_empty() {
        qb.push(" WHERE ");
        let mut separated = qb.separated(" AND ");
        for (col, sieve) in active_filters {
            let col_sql = match col {
                UserOverviewColumns::Id => "CAST(id AS TEXT)",
                UserOverviewColumns::Username => "username",
                UserOverviewColumns::Email => "email",
            };
            separated
                .push(col_sql)
                .push_unseparated(" LIKE ")
                .push_bind_unseparated(format!("%{}%", sieve));
        }
    }

    if let Some(order) = &params.order {
        let col_sql = match order.column {
            UserOverviewColumns::Id => "id",
            UserOverviewColumns::Username => "username",
            UserOverviewColumns::Email => "email",
        };
        let dir_sql = match order.value {
            Ordering::Ascending => "ASC",
            Ordering::Descending => "DESC",
        };
        qb.push(" ORDER BY ").push(col_sql).push(" ").push(dir_sql);
    }

    qb.push(" LIMIT ")
        .push_bind(params.amount as i64)
        .push(" OFFSET ")
        .push_bind(params.offset as i64);

    let rows = qb.build().fetch_all(pool).await.sfe()?;
    let users = rows
        .into_iter()
        .map(|v| User {
            id: v.get("id"),
            username: v.get("username"),
            email: v.get("email"),
            role: Role::convert(v.get("role")),
            language: Lang::from_str(v.get("language")).unwrap_or_default(),
        })
        .collect();

    Ok(users)
}

/// Update an existing user's profile, role, and password hash.
///
/// # Errors
/// Fails with [`ServerFnError`] if constraints are violated or the database cannot be accessed.
pub async fn update_user(
    pool: &SqlitePool,
    user: &User,
    password_hash: &str,
) -> Result<(), ServerFnError> {
    let role_i64 = user.role.to_i64();
    sqlx::query!(
        "UPDATE users SET
        username = ?,
        email = ?,
        password = ?,
        role = ? where id = ?",
        user.username,
        user.email,
        password_hash,
        role_i64,
        user.id
    )
    .execute(pool)
    .await
    .sfe()?;
    Ok(())
}

/// Set the preferred language of a user.
///
/// # Errors
/// Fails with [`ServerFnError`] if the database cannot be accessed.
pub async fn set_language(
    pool: &SqlitePool,
    user_id: i64,
    new_language: Lang,
) -> Result<(), ServerFnError> {
    let lang_code = new_language.to_string();
    sqlx::query!(
        "UPDATE users SET language = ? where id = ?",
        lang_code,
        user_id
    )
    .execute(pool)
    .await
    .sfe()?;
    Ok(())
}

/// Count the number of administrator accounts.
///
/// # Errors
/// Fails with [`ServerFnError`] if the database cannot be accessed.
pub async fn count_admins(pool: &SqlitePool) -> Result<Count, ServerFnError> {
    let num = sqlx::query_as!(Count, "select count(*) as number from users where role = 2")
        .fetch_one(pool)
        .await
        .sfe()?;
    Ok(num)
}

/// Delete a user by their ID and reassign their links to `current_user_id` within a transaction.
///
/// # Errors
/// Fails with [`ServerFnError`] if the transaction fails.
pub async fn delete_user(
    pool: &SqlitePool,
    id: i64,
    current_user_id: i64,
) -> Result<(), ServerFnError> {
    let mut tx = pool.begin().await.sfe()?;
    sqlx::query!(
        "UPDATE links SET author = ? where author = ?",
        current_user_id,
        id
    )
    .execute(&mut *tx)
    .await
    .sfe()?;
    sqlx::query!("DELETE from users where id = ? ", id)
        .execute(&mut *tx)
        .await
        .sfe()?;
    tx.commit().await.sfe()?;
    Ok(())
}

/// Relevant parameters when creating a new user.
#[derive(Debug, Deserialize)]
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub password: String,
}

impl NewUser {
    /// Create a new user with an Argon2-hashed password.
    ///
    /// # Errors
    /// Fails with [`ServerFnError`] if the password could not be hashed.
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

    /// Hash a plaintext password using Argon2id and the server's secret pepper.
    pub(crate) fn hash_password(password: &str, secret: &Secret) -> Result<String, ServerFnError> {
        let secret_bytes = secret
            .secret
            .as_ref()
            .ok_or_else(|| ServerFnError::new("Server secret is not initialized"))?
            .as_bytes();
        let argon2 = argon2::Argon2::new_with_secret(
            secret_bytes,
            argon2::Algorithm::default(),
            argon2::Version::default(),
            argon2::Params::default(),
        )
        .map_err(|e| ServerFnError::new(format!("Password hasher error: {}", e)))?;
        let salt = argon2::password_hash::SaltString::generate(&mut rand::rngs::OsRng);
        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| ServerFnError::new(format!("Password error: {}", e)))?;

        Ok(hash.to_string())
    }

    /// Verify a plaintext password against a stored Argon2 password hash.
    pub(crate) fn verify_password(
        password: &str,
        password_hash: &str,
        secret: &Secret,
    ) -> Result<(), ServerFnError> {
        let parsed_hash = argon2::PasswordHash::new(password_hash)
            .map_err(|_| ServerFnError::new("Unauthorized"))?;
        let secret_bytes = secret
            .secret
            .as_ref()
            .ok_or_else(|| ServerFnError::new("Server secret is not initialized"))?
            .as_bytes();
        argon2::Argon2::new_with_secret(
            secret_bytes,
            argon2::Algorithm::default(),
            argon2::Version::default(),
            argon2::Params::default(),
        )
        .map_err(|_| ServerFnError::new("Unauthorized"))?
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| ServerFnError::new("Unauthorized"))
    }

    /// Insert this user into the database with the given [`Role`].
    ///
    /// # Errors
    /// Fails with [`ServerFnError`] if the database cannot be accessed or constraints fail.
    pub async fn insert_user(&self, pool: &SqlitePool, role: Role) -> Result<(), ServerFnError> {
        let role_i64 = role.to_i64();
        sqlx::query!(
            "Insert into users (
            username,
            email,
            password,
            role) VALUES (?,?,?,?)",
            self.username,
            self.email,
            self.password,
            role_i64,
        )
        .execute(pool)
        .await
        .sfe()?;
        Ok(())
    }
}

/// Create an admin user in a single query.
///
/// # Errors
/// Fails with [`ServerFnError`] if inserting the user fails.
pub async fn create_admin(pool: &SqlitePool, new_user: &NewUser) -> Result<(), ServerFnError> {
    new_user.insert_user(pool, Role::Admin).await?;
    info!("Admin user created: {}", &new_user.username);
    Ok(())
}

/// Query a filtered, ordered, and paginated list of links with author info and click counts.
///
/// # Errors
/// Fails with [`ServerFnError`] if the database query fails.
pub async fn list_links_filtered(
    pool: &SqlitePool,
    params: &LinkRequestForm,
) -> Result<Vec<FullLink>, ServerFnError> {
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
        "SELECT
            links.id as lid,
            links.title as ltitle,
            links.target as ltarget,
            links.code as lcode,
            links.author as lauthor,
            links.created_at as ldate,
            users.id as usid,
            users.username as usern,
            users.email as uemail,
            users.role as urole,
            users.language as ulang,
            COUNT(clicks.id) as counter
        FROM links
        JOIN users ON links.author = users.id
        LEFT JOIN clicks ON links.id = clicks.link",
    );

    let where_filters: Vec<(LinkOverviewColumns, &str)> = params
        .filter
        .iter()
        .filter_map(|(col, filter)| {
            let trimmed = filter.sieve.trim();
            if trimmed.is_empty() || col == LinkOverviewColumns::Statistics {
                None
            } else {
                Some((col, trimmed))
            }
        })
        .collect();

    if !where_filters.is_empty() {
        qb.push(" WHERE ");
        let mut separated = qb.separated(" AND ");
        for (col, sieve) in where_filters {
            let col_sql = match col {
                LinkOverviewColumns::Code => "links.code",
                LinkOverviewColumns::Description => "links.title",
                LinkOverviewColumns::Target => "links.target",
                LinkOverviewColumns::Author => "users.username",
                LinkOverviewColumns::Statistics => unreachable!(),
            };
            separated
                .push(col_sql)
                .push_unseparated(" LIKE ")
                .push_bind_unseparated(format!("%{}%", sieve));
        }
    }

    qb.push(" GROUP BY links.id");

    let stats_filter = params.filter[LinkOverviewColumns::Statistics].sieve.trim();
    if !stats_filter.is_empty() {
        qb.push(" HAVING CAST(COUNT(clicks.id) AS TEXT) LIKE ")
            .push_bind(format!("%{}%", stats_filter));
    }

    if let Some(order) = &params.order {
        let col_sql = match order.column {
            LinkOverviewColumns::Code => "lcode",
            LinkOverviewColumns::Description => "ltitle",
            LinkOverviewColumns::Target => "ltarget",
            LinkOverviewColumns::Author => "usern",
            LinkOverviewColumns::Statistics => "counter",
        };
        let dir_sql = match order.value {
            Ordering::Ascending => "ASC",
            Ordering::Descending => "DESC",
        };
        qb.push(" ORDER BY ").push(col_sql).push(" ").push(dir_sql);
    }

    qb.push(" LIMIT ")
        .push_bind(params.amount as i64)
        .push(" OFFSET ")
        .push_bind(params.offset as i64);

    let rows = qb.build().fetch_all(pool).await.sfe()?;
    let links = rows
        .into_iter()
        .map(|v| FullLink {
            link: Link {
                id: v.get("lid"),
                title: v.get("ltitle"),
                target: v.get("ltarget"),
                code: v.get("lcode"),
                author: v.get("lauthor"),
                created_at: v.get("ldate"),
            },
            user: User {
                id: v.get("usid"),
                username: v.get("usern"),
                email: v.get("uemail"),
                role: Role::convert(v.get("urole")),
                language: Lang::from_str(v.get("ulang")).unwrap_or_default(),
            },
            clicks: Clicks::Count(Count {
                number: v.get("counter"),
            }),
        })
        .collect();

    Ok(links)
}

/// Get 52-week click statistics for a link by its ID.
///
/// # Errors
/// Fails with [`ServerFnError`] if the database cannot be accessed or the link is not found.
pub async fn get_statistics(pool: &SqlitePool, link_id: i64) -> Result<Statistics, ServerFnError> {
    let link = sqlx::query!("select id,code from links where id=?", link_id)
        .fetch_one(pool)
        .await
        .sfe()?;

    let qry = "WITH all_weeks AS (
  WITH RECURSIVE weeks(date_value) AS (
    SELECT date('now', 'start of month', '-1 year')
    UNION ALL
    SELECT date(date_value, '+7 days')
    FROM weeks
    WHERE date_value < date('now')
  )
  SELECT 
    date_value AS full_date,
    strftime('%Y.%W', date_value) AS year_week,
    strftime('%W', date_value) AS week
  FROM weeks
),
clicks_per_week AS (
  SELECT
    strftime('%Y.%W', created_at) AS year_week,
    COUNT(*) AS total
  FROM clicks
  WHERE link = ?
    AND created_at > date('now', 'start of month', '-1 year')
  GROUP BY year_week
)
SELECT
  aw.full_date,
  aw.week,
  COALESCE(cpw.total, 0) AS total
FROM all_weeks aw
LEFT JOIN clicks_per_week cpw ON cpw.year_week = aw.year_week
ORDER BY aw.full_date";

    let code = link.code;
    let values: Vec<WeekCount> = sqlx::query(qry)
        .bind(link.id)
        .fetch_all(pool)
        .await
        .sfe()?
        .into_iter()
        .map(|c| {
            let date_str: String = c.get("full_date");
            let full_date = date_str.parse::<NaiveDate>().expect("failed to parse date");
            let week_str: String = c.get("week");
            let week: i32 = week_str.parse().unwrap_or(0);

            WeekCount {
                full_date,
                total: Count {
                    number: c.get("total"),
                },
                week,
            }
        })
        .collect();

    let total = sqlx::query_as!(
        Count,
        "select count(*) as number from clicks join links on clicks.link = links.id where links.code = ?",
        code
    )
    .fetch_one(pool)
    .await
    .sfe()?;

    Ok(Statistics {
        link_id,
        total,
        values,
    })
}

/// Get a link by its code (case-insensitive).
///
/// # Errors
/// Fails with [`ServerFnError`] if the database cannot be accessed or the link is not found.
pub async fn get_link_by_code(pool: &SqlitePool, code: &str) -> Result<Link, ServerFnError> {
    let link = sqlx::query_as!(
        Link,
        "Select id, title, target, code, author, created_at as \"created_at: NaiveDateTime\" from links where code = ? COLLATE NOCASE",
        code
    )
    .fetch_one(pool)
    .await;
    info!("Found link: {:?}", &link);
    link.sfe()
}

/// Get a link by its ID.
///
/// # Errors
/// Fails with [`ServerFnError`] if the database cannot be accessed or the link is not found.
pub async fn get_link_by_id(pool: &SqlitePool, id: i64) -> Result<Link, ServerFnError> {
    let link = sqlx::query_as!(
        Link,
        "Select id, title, target, code, author, created_at as \"created_at: NaiveDateTime\" from links where id = ? ",
        id
    )
    .fetch_one(pool)
    .await;
    info!("Found link: {:?}", &link);
    link.sfe()
}

/// Delete a link by its code.
///
/// # Errors
/// Fails with [`ServerFnError`] if the database cannot be accessed.
pub async fn delete_link_by_code(pool: &SqlitePool, code: &str) -> Result<(), ServerFnError> {
    sqlx::query!("DELETE from links where code = ? ", code)
        .execute(pool)
        .await
        .sfe()?;
    Ok(())
}

/// Update an existing link.
///
/// # Errors
/// Fails with [`ServerFnError`] if constraints are violated or the database cannot be accessed.
pub async fn update_link(pool: &SqlitePool, link: &Link) -> Result<(), ServerFnError> {
    info!("{:?}", link);
    let qry = sqlx::query!(
        "UPDATE links SET
        title = ?,
        target = ?,
        code = ?,
        author = ?,
        created_at = ? where id = ?",
        link.title,
        link.target,
        link.code,
        link.author,
        link.created_at,
        link.id
    );
    match qry.execute(pool).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("{}", e);
            Err(ServerFnError::new(e))
        }
    }
}

/// Generate a unique 6-character random alphanumeric short code.
pub async fn generate_unique_code(pool: &SqlitePool) -> Result<String, ServerFnError> {
    for _ in 0..10 {
        let candidate: String =
            rand::Rng::sample_iter(rand::thread_rng(), &rand::distributions::Alphanumeric)
                .take(6)
                .map(char::from)
                .collect::<String>()
                .to_ascii_lowercase();
        if candidate != "app" && get_link_by_code(pool, &candidate).await.is_err() {
            return Ok(candidate);
        }
    }
    Err(ServerFnError::new("Failed to generate unique link code"))
}

/// Relevant parameters when creating a new link.
#[derive(Serialize, Debug)]
pub struct NewLink {
    pub title: String,
    pub target: String,
    pub code: String,
    pub author: i64,
    pub created_at: NaiveDateTime,
}

impl NewLink {
    /// Create a [`NewLink`] from a [`LinkDelta`] and author ID.
    #[must_use]
    pub fn from_link_delta(link: LinkDelta, uid: i64) -> Self {
        Self {
            title: link.title,
            target: link.target,
            code: link.code,
            author: uid,
            created_at: chrono::Local::now().naive_utc(),
        }
    }

    /// Insert the new link into the database.
    ///
    /// # Errors
    /// Fails with [`ServerFnError`] if the database cannot be accessed or constraints are not met.
    pub async fn insert(self, pool: &SqlitePool) -> Result<(), ServerFnError> {
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
        .execute(pool)
        .await
        .sfe()?;
        Ok(())
    }
}

/// Whenever a link is clicked the click is registered for statistical purposes.
#[derive(Serialize)]
pub struct NewClick {
    pub link: i64,
    pub created_at: NaiveDateTime,
}

impl NewClick {
    #[must_use]
    pub fn new(link_id: i64) -> Self {
        Self {
            link: link_id,
            created_at: chrono::Local::now().naive_utc(),
        }
    }

    pub async fn insert_click(self, pool: &SqlitePool) -> Result<(), ServerFnError> {
        sqlx::query!(
            "Insert into clicks (
            link,
            created_at) VALUES (?,?)",
            self.link,
            self.created_at,
        )
        .execute(pool)
        .await
        .sfe()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::requests::{Filter, Operation};
    use crate::server::cli::MIGRATOR;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn test_user_lifecycle_and_password_verification() {
        let pool = setup_test_db().await;
        let secret = Secret::new("super-secret-pepper-key".to_string());

        let admin_new = NewUser::new(
            "admin".to_string(),
            "admin@example.com".to_string(),
            "correct-horse-battery",
            &secret,
        )
        .unwrap();
        create_admin(&pool, &admin_new).await.unwrap();

        let (admin_user, stored_hash) =
            get_user_with_password_by_name(&pool, "admin").await.unwrap();
        assert_eq!(admin_user.role, Role::Admin);
        assert!(NewUser::verify_password("correct-horse-battery", &stored_hash, &secret).is_ok());
        assert!(NewUser::verify_password("wrong-password", &stored_hash, &secret).is_err());

        set_language(&pool, admin_user.id, Lang::DeDE).await.unwrap();
        let updated = get_user(&pool, admin_user.id).await.unwrap();
        assert_eq!(updated.language, Lang::DeDE);

        let count = count_admins(&pool).await.unwrap();
        assert_eq!(count.number, 1);
    }

    #[tokio::test]
    async fn test_links_filtering_with_special_chars_and_user_deletion_reassignment() {
        let pool = setup_test_db().await;
        let secret = Secret::new("test-secret-key".to_string());

        let admin = NewUser::new(
            "admin".to_string(),
            "admin@test.org".to_string(),
            "adminpass",
            &secret,
        )
        .unwrap();
        create_admin(&pool, &admin).await.unwrap();

        let regular = NewUser::new(
            "alice".to_string(),
            "alice@example.org".to_string(),
            "alicepass",
            &secret,
        )
        .unwrap();
        regular.insert_user(&pool, Role::Regular).await.unwrap();
        let alice = get_user_by_name(&pool, "alice").await.unwrap();

        NewLink {
            title: "Rust & Dioxus Guide".to_string(),
            target: "https://dioxuslabs.com/learn/0.6/".to_string(),
            code: "dx-guide".to_string(),
            author: alice.id,
            created_at: chrono::Local::now().naive_utc(),
        }
        .insert(&pool)
        .await
        .unwrap();

        let link = get_link_by_code(&pool, "DX-GUIDE").await.unwrap();
        assert_eq!(link.code, "dx-guide");

        // Record two clicks
        NewClick::new(link.id).insert_click(&pool).await.unwrap();
        NewClick::new(link.id).insert_click(&pool).await.unwrap();

        // Filter by URL with colons, slashes, and dots (previously stripped by alphanumeric filter!)
        let mut form = LinkRequestForm::default();
        form.filter[LinkOverviewColumns::Target] = Filter {
            sieve: "https://dioxuslabs.com/learn".to_string(),
        };
        let filtered = list_links_filtered(&pool, &form).await.unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].clicks, Clicks::Count(Count { number: 2 }));

        // Verify statistics query
        let stats = get_statistics(&pool, link.id).await.unwrap();
        assert_eq!(stats.total.number, 2);
        assert!(!stats.values.is_empty());

        // Filter users by email containing '@' and '.'
        let mut user_form = UserRequestForm::default();
        user_form.filter[UserOverviewColumns::Email] = Filter {
            sieve: "@example.org".to_string(),
        };
        user_form.order = Some(Operation {
            column: UserOverviewColumns::Username,
            value: Ordering::Ascending,
        });
        let users = list_users_filtered(&pool, &user_form).await.unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].username, "alice");

        // Delete alice and reassign her links to admin (id = 1)
        delete_user(&pool, alice.id, 1).await.unwrap();
        let reassigned_link = get_link_by_id(&pool, link.id).await.unwrap();
        assert_eq!(reassigned_link.author, 1);
    }
}
