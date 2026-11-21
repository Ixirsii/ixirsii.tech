//! Admin user database operations.

use axum_login::AuthUser;
use sqlx::{PgExecutor, query_as};

use tracing::{error, instrument};

use crate::Result;

/// User.
#[derive(Clone, Debug)]
pub struct User {
    /// User ID.
    pub id: i64,
    /// Username.
    pub username: String,
    /// Password hash.
    pub password_hash: String,
}

impl AuthUser for User {
    type Id = i64;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.password_hash.as_bytes()
    }
}

/* ********************************************************************************************** *
 *                                             Create                                             *
 * ********************************************************************************************** */

/// Insert a new admin; a username conflict leaves the existing account untouched.
pub async fn insert_admin_user(
    executor: impl PgExecutor<'_>,
    username: &str,
    password_hash: &str,
) -> Result<bool> {
    let result = sqlx::query!(
        "INSERT INTO admin_users (username, password_hash) VALUES ($1, $2)
         ON CONFLICT (username) DO NOTHING",
        username,
        password_hash
    )
    .execute(executor)
    .await?;
    Ok(result.rows_affected() == 1)
}

/* ********************************************************************************************** *
 *                                             Delete                                             *
 * ********************************************************************************************** */

/* ********************************************************************************************** *
 *                                              Read                                              *
 * ********************************************************************************************** */

/// Get user by ID.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn get_admin_user_by_id(executor: impl PgExecutor<'_>, id: i64) -> Result<Option<User>> {
    let user = query_as!(
        User,
        "SELECT id, username, password_hash FROM admin_users WHERE id = $1",
        id
    )
    .fetch_optional(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "select", table = "admin_users", "database query failed");
    })?;

    Ok(user)
}

/// Get user by username.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to read from database.
#[instrument(skip(executor, username))]
pub async fn get_admin_user_by_username(
    executor: impl PgExecutor<'_>,
    username: &str,
) -> Result<Option<User>> {
    let user = query_as!(
        User,
        "SELECT id, username, password_hash FROM admin_users WHERE username = $1",
        username
    )
    .fetch_optional(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "select", table = "admin_users", "database query failed");
    })?;

    Ok(user)
}

/* ********************************************************************************************** *
 *                                             Update                                             *
 * ********************************************************************************************** */

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;

    #[sqlx::test]
    async fn admin_lookup_preserves_case_sensitive_usernames(pool: PgPool) -> Result<()> {
        let id = sqlx::query_scalar!(
            "INSERT INTO admin_users (username, password_hash) VALUES ($1, $2) RETURNING id",
            "TestAdmin",
            "test-hash",
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(
            get_admin_user_by_username(&pool, "TestAdmin")
                .await?
                .map(|user| user.id),
            Some(id)
        );
        assert!(
            get_admin_user_by_username(&pool, "testadmin")
                .await?
                .is_none()
        );
        assert_eq!(
            get_admin_user_by_id(&pool, id)
                .await?
                .map(|user| user.password_hash),
            Some("test-hash".into())
        );
        assert!(get_admin_user_by_id(&pool, i64::MAX).await?.is_none());
        Ok(())
    }
}
