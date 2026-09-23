//! Transactional database operations.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

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
    pool: &PgPool,
    title: &str,
    slug: &str,
    body: &str,
    preview: Option<&str>,
    published_at: Option<DateTime<Utc>>,
    tag_ids: &[i64],
) -> Result<i64> {
    let mut transaction = pool.begin().await.inspect_err(|error| {
        error!(%error, operation = "begin", table = "posts", "database transaction failed");
    })?;
    let post_id: i64 =
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
    pool: &PgPool,
    post_id: i64,
    title: &str,
    slug: &str,
    body: &str,
    preview: Option<&str>,
    published_at: Option<DateTime<Utc>>,
    tag_ids: &[i64],
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
    let current_ids: HashSet<i64> = current_tags.iter().map(|t| t.id).collect();
    let new_ids: HashSet<i64> = tag_ids.iter().copied().collect();

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{
        PostStatusFilter, count_posts, create_tag, delete_post, delete_tag, get_post_by_id,
        list_tags_with_count, slug_exists, slug_exists_for_other_post,
    };

    #[sqlx::test]
    async fn mutations_are_atomic_and_foreign_keys_cascade(pool: PgPool) -> Result<()> {
        let first_tag = create_tag(&pool, "First", "first").await?;
        let second_tag = create_tag(&pool, "Second", "second").await?;
        let missing_tag = i64::MAX;
        let invalid = create_post_with_tags(
            &pool,
            "Invalid",
            "invalid",
            "Body",
            None,
            None,
            &[first_tag, missing_tag],
        )
        .await;
        assert!(invalid.is_err());
        assert_eq!(count_posts(&pool, &PostStatusFilter::All).await?, 0);

        let id = create_post_with_tags(
            &pool,
            "Original",
            "original",
            "Original body",
            Some("Preview"),
            None,
            &[first_tag],
        )
        .await?;
        assert!(slug_exists(&pool, "original").await?);
        assert!(!slug_exists_for_other_post(&pool, "original", id).await?);
        assert!(slug_exists_for_other_post(&pool, "original", missing_tag).await?);
        assert!(
            create_post_with_tags(
                &pool,
                "Duplicate",
                "original",
                "Body",
                None,
                None,
                &[second_tag],
            )
            .await
            .is_err()
        );
        assert!(add_tag_to_post(&pool, id, first_tag).await.is_err());

        // The failed tag insertion must roll back the edited fields and removed old tag.
        assert!(
            update_post_with_tags(
                &pool,
                id,
                "Changed",
                "changed",
                "Changed body",
                None,
                None,
                &[missing_tag],
            )
            .await
            .is_err()
        );
        let unchanged = get_post_by_id(&pool, id)
            .await?
            .ok_or_else(|| crate::Error::System("missing original post".into()))?;
        assert_eq!(unchanged.title, "Original");
        assert_eq!(unchanged.slug, "original");
        assert_eq!(unchanged.body, "Original body");
        assert_eq!(unchanged.preview.as_deref(), Some("Preview"));
        assert!(unchanged.updated_at.is_none());
        assert_eq!(
            unchanged.tags.iter().map(|tag| tag.id).collect::<Vec<_>>(),
            [first_tag]
        );

        let published_at = Utc::now() - chrono::Duration::days(1);
        update_post_with_tags(
            &pool,
            id,
            "Changed",
            "changed",
            "Changed body",
            None,
            Some(published_at),
            &[second_tag],
        )
        .await?;
        let changed = get_post_by_id(&pool, id)
            .await?
            .ok_or_else(|| crate::Error::System("missing updated post".into()))?;
        assert_eq!(changed.title, "Changed");
        assert_eq!(changed.body, "Changed body");
        assert!(changed.preview.is_none());
        assert!(changed.updated_at.is_some());
        assert_eq!(
            changed.published_at.map(|date| date.timestamp()),
            Some(published_at.timestamp())
        );
        assert_eq!(
            changed.tags.iter().map(|tag| tag.id).collect::<Vec<_>>(),
            [second_tag]
        );

        delete_tag(&pool, second_tag).await?;
        assert!(list_tags_for_post(&pool, id).await?.is_empty());
        add_tag_to_post(&pool, id, first_tag).await?;
        delete_post(&pool, id).await?;
        assert!(get_post_by_id(&pool, id).await?.is_none());
        assert_eq!(list_tags_with_count(&pool).await?[0].count, 0);
        Ok(())
    }
}
