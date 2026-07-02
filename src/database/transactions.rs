//! Transactional database operations.

use std::collections::HashSet;

use chrono::NaiveDateTime;
use sqlx::MySqlPool;

use tracing::{error, instrument};

use crate::{
    Result,
    database::{
        add_tag_to_post, create_post, list_tags_for_post, remove_tag_from_post, update_post,
    },
};

/* ********************************************************************************************** *
 *                                             Create                                             *
 * ********************************************************************************************** */

/// Create a new post and add tags.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to write to database.
#[instrument(skip(pool, body, preview, tag_ids))]
pub async fn create_post_with_tags(
    pool: &MySqlPool,
    title: &str,
    slug: &str,
    body: &str,
    preview: Option<&str>,
    published_at: Option<NaiveDateTime>,
    tag_ids: &[u64],
) -> Result<u64> {
    let mut transaction = pool.begin().await.inspect_err(|error| {
        error!(%error, operation = "begin", table = "posts", "database transaction failed");
    })?;
    let post_id: u64 =
        create_post(&mut *transaction, title, slug, body, preview, published_at).await?;

    for &tag_id in tag_ids {
        add_tag_to_post(&mut *transaction, post_id, tag_id).await?;
    }

    transaction.commit().await.inspect_err(|error| {
        error!(%error, operation = "commit", table = "posts", "database transaction failed");
    })?;

    Ok(post_id)
}

/* ********************************************************************************************** *
 *                                             Delete                                             *
 * ********************************************************************************************** */

/* ********************************************************************************************** *
 *                                              Read                                              *
 * ********************************************************************************************** */

/* ********************************************************************************************** *
 *                                             Update                                             *
 * ********************************************************************************************** */

/// Update a post and its tags.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to write to database.
#[instrument(skip(pool, body, preview, tag_ids))]
pub async fn update_post_with_tags(
    pool: &MySqlPool,
    post_id: u64,
    title: &str,
    slug: &str,
    body: &str,
    preview: Option<&str>,
    published_at: Option<NaiveDateTime>,
    tag_ids: &[u64],
) -> Result<()> {
    let mut transaction = pool.begin().await.inspect_err(|error| {
        error!(%error, operation = "begin", table = "posts", "database transaction failed");
    })?;

    update_post(
        &mut *transaction,
        post_id,
        title,
        slug,
        body,
        preview,
        published_at,
    )
    .await?;

    let current_tags = list_tags_for_post(&mut *transaction, post_id).await?;
    let current_ids: HashSet<u64> = current_tags.iter().map(|t| t.id).collect();
    let new_ids: HashSet<u64> = tag_ids.iter().copied().collect();

    for tag_id in current_ids.difference(&new_ids) {
        remove_tag_from_post(&mut *transaction, post_id, *tag_id).await?;
    }

    for tag_id in new_ids.difference(&current_ids) {
        add_tag_to_post(&mut *transaction, post_id, *tag_id).await?;
    }

    transaction.commit().await.inspect_err(|error| {
        error!(%error, operation = "commit", table = "posts", "database transaction failed");
    })?;

    Ok(())
}
