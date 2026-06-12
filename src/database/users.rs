//! Admin user database operations.

use axum_login::AuthUser;
use sqlx::{FromRow, MySqlExecutor, query_as};

use tracing::instrument;

use crate::{Error, Result};

/// User.
#[derive(Clone, Debug, FromRow)]
pub struct User {
    /// User ID.
    pub id: u64,
    /// Username.
    pub username: String,
    /// Password hash.
    pub password_hash: String,
}

impl AuthUser for User {
    type Id = u64;

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

/* ********************************************************************************************** *
 *                                             Delete                                             *
 * ********************************************************************************************** */

/* ********************************************************************************************** *
 *                                              Read                                              *
 * ********************************************************************************************** */

/// Get user by ID.
///
/// # Errors
/// Returns [`Error::DatabaseSelectError`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn get_admin_user_by_id(
    executor: impl MySqlExecutor<'_>,
    id: u64,
) -> Result<Option<User>> {
    query_as::<_, User>("SELECT id, username, password_hash FROM admin_users WHERE id = ?")
        .bind(id)
        .fetch_optional(executor)
        .await
        .map_err(|sql_err| Error::DatabaseSelectError(sql_err.to_string()))
}

/// Get user by username.
///
/// # Errors
/// Returns [`Error::DatabaseSelectError`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn get_admin_user_by_username(
    executor: impl MySqlExecutor<'_>,
    username: &str,
) -> Result<Option<User>> {
    query_as::<_, User>("SELECT id, username, password_hash FROM admin_users WHERE username = ?")
        .bind(username)
        .fetch_optional(executor)
        .await
        .map_err(|sql_err| Error::DatabaseSelectError(sql_err.to_string()))
}

/* ********************************************************************************************** *
 *                                             Update                                             *
 * ********************************************************************************************** */
