//! Post tags database operations.

use sqlx::{MySqlExecutor, query};

use tracing::instrument;

use crate::{Error, Result};

/* ********************************************************************************************** *
 *                                             Create                                             *
 * ********************************************************************************************** */

/// Add a tag to a post.
///
/// # Errors
/// Returns [`Error::DatabaseInsertError`] on failure to write to database.
#[instrument(skip(executor))]
pub async fn add_tag_to_post(
    executor: impl MySqlExecutor<'_>,
    post_id: u64,
    tag_id: u64,
) -> Result<()> {
    query("INSERT INTO post_tags (post_id, tag_id) VALUES (?, ?)")
        .bind(post_id)
        .bind(tag_id)
        .execute(executor)
        .await
        .map(|_| ())
        .map_err(|sql_err| Error::DatabaseInsertError(sql_err.to_string()))
}

/* ********************************************************************************************** *
 *                                             Delete                                             *
 * ********************************************************************************************** */

/// Remove tag from post.
///
/// # Errors
/// Returns [`Error::DatabaseDeleteError`] on failure to write to database.
#[instrument(skip(executor))]
pub async fn remove_tag_from_post(
    executor: impl MySqlExecutor<'_>,
    post_id: u64,
    tag_id: u64,
) -> Result<()> {
    query("DELETE FROM post_tags WHERE post_id = ? AND tag_id = ?")
        .bind(post_id)
        .bind(tag_id)
        .execute(executor)
        .await
        .map(|_| ())
        .map_err(|sql_err| Error::DatabaseDeleteError(sql_err.to_string()))
}

/* ********************************************************************************************** *
 *                                              Read                                              *
 * ********************************************************************************************** */

/* ********************************************************************************************** *
 *                                             Update                                             *
 * ********************************************************************************************** */
