//! Post tags database operations.

use sqlx::{PgExecutor, query};

use tracing::{error, instrument};

use crate::Result;

/* ********************************************************************************************** *
 *                                             Create                                             *
 * ********************************************************************************************** */

/// Add a tag to a post.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to write to database.
#[instrument(skip(executor))]
pub async fn add_tag_to_post(
    executor: impl PgExecutor<'_>,
    post_id: i64,
    tag_id: i64,
) -> Result<()> {
    query!(
        "INSERT INTO post_tags (post_id, tag_id) VALUES ($1, $2)",
        post_id,
        tag_id
    )
    .execute(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "insert", table = "post_tags", "database query failed");
    })?;

    Ok(())
}

/* ********************************************************************************************** *
 *                                             Delete                                             *
 * ********************************************************************************************** */

/// Remove tag from post.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to write to database.
#[instrument(skip(executor))]
pub async fn remove_tag_from_post(
    executor: impl PgExecutor<'_>,
    post_id: i64,
    tag_id: i64,
) -> Result<()> {
    query!(
        "DELETE FROM post_tags WHERE post_id = $1 AND tag_id = $2",
        post_id,
        tag_id
    )
    .execute(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "delete", table = "post_tags", "database query failed");
    })?;

    Ok(())
}

/* ********************************************************************************************** *
 *                                              Read                                              *
 * ********************************************************************************************** */

/* ********************************************************************************************** *
 *                                             Update                                             *
 * ********************************************************************************************** */
