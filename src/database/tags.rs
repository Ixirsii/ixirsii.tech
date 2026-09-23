//! Tag database operations.

use sqlx::{PgExecutor, query, query_as, query_scalar};

use tracing::{error, instrument};

use crate::{
    Result,
    database::{Tag, TagWithCount},
};

/* ********************************************************************************************** *
 *                                             Create                                             *
 * ********************************************************************************************** */

/// Create a new tag.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to write to database.
#[instrument(skip(executor))]
pub async fn create_tag(executor: impl PgExecutor<'_>, name: &str, slug: &str) -> Result<i64> {
    let id = query_scalar!(
        "INSERT INTO tags (name, slug) VALUES ($1, $2) RETURNING id",
        name,
        slug
    )
    .fetch_one(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "insert", table = "tags", "database query failed");
    })?;

    Ok(id)
}

/* ********************************************************************************************** *
 *                                             Delete                                             *
 * ********************************************************************************************** */

/// Delete tag.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to write to database.
#[instrument(skip(executor))]
pub async fn delete_tag(executor: impl PgExecutor<'_>, id: i64) -> Result<()> {
    query!("DELETE FROM tags WHERE id = $1", id)
        .execute(executor)
        .await
        .inspect_err(|error| {
            error!(%error, operation = "delete", table = "tags", "database query failed");
        })?;

    Ok(())
}

/* ********************************************************************************************** *
 *                                              Read                                              *
 * ********************************************************************************************** */

/// Get tag by id.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn get_tag_by_id(executor: impl PgExecutor<'_>, id: i64) -> Result<Option<Tag>> {
    let tag = query_as!(Tag, "SELECT id, name, slug FROM tags WHERE id = $1", id)
        .fetch_optional(executor)
        .await
        .inspect_err(|error| {
            error!(%error, operation = "select", table = "tags", "database query failed");
        })?;

    Ok(tag)
}

/// Get tag by slug.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn get_tag_by_slug(executor: impl PgExecutor<'_>, slug: &str) -> Result<Option<Tag>> {
    let tag = query_as!(Tag, "SELECT id, name, slug FROM tags WHERE slug = $1", slug)
        .fetch_optional(executor)
        .await
        .inspect_err(|error| {
            error!(%error, operation = "select", table = "tags", "database query failed");
        })?;

    Ok(tag)
}

/// Get tags for a post.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn list_tags_for_post(executor: impl PgExecutor<'_>, post_id: i64) -> Result<Vec<Tag>> {
    let tags = query_as!(
        Tag,
        "SELECT t.id, t.name, t.slug
         FROM tags t
         JOIN post_tags pt ON pt.tag_id = t.id
         WHERE pt.post_id = $1
         ORDER BY t.name",
        post_id
    )
    .fetch_all(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "select", table = "tags", "database query failed");
    })?;

    Ok(tags)
}

/// List tags with their counts.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn list_tags_with_count(executor: impl PgExecutor<'_>) -> Result<Vec<TagWithCount>> {
    let tags = query_as!(
        TagWithCount,
        r#"SELECT t.id, t.name, t.slug, COUNT(pt.post_id) AS "count!"
         FROM tags t
         LEFT JOIN post_tags pt ON pt.tag_id = t.id
         GROUP BY t.id, t.name, t.slug
         ORDER BY t.name"#
    )
    .fetch_all(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "select", table = "tags", "database query failed");
    })?;

    Ok(tags)
}

/// List tags with their published counts.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn list_tags_with_published_count(
    executor: impl PgExecutor<'_>,
) -> Result<Vec<TagWithCount>> {
    let tags = query_as!(
        TagWithCount,
        r#"SELECT t.id, t.name, t.slug, COUNT(p.id) AS "count!"
         FROM tags t
         LEFT JOIN post_tags pt ON pt.tag_id = t.id
         LEFT JOIN posts p ON p.id = pt.post_id
            AND p.published_at IS NOT NULL
            AND p.published_at <= NOW()
         GROUP BY t.id, t.name, t.slug
         ORDER BY t.name"#
    )
    .fetch_all(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "select", table = "tags", "database query failed");
    })?;

    Ok(tags)
}

/// Check if the tag name already exists.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn tag_name_exists(executor: impl PgExecutor<'_>, name: &str) -> Result<bool> {
    let count: i64 = query_scalar!(
        r#"SELECT COUNT(*) AS "count!" FROM tags WHERE name = $1"#,
        name
    )
    .fetch_one(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "select", table = "tags", "database query failed");
    })?;

    Ok(count > 0)
}

/// Check if the tag name already exists for a different tag.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn tag_name_exists_for_other_tag(
    executor: impl PgExecutor<'_>,
    name: &str,
    tag_id: i64,
) -> Result<bool> {
    let count: i64 = query_scalar!(
        r#"SELECT COUNT(*) AS "count!" FROM tags WHERE name = $1 AND id <> $2"#,
        name,
        tag_id
    )
    .fetch_one(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "select", table = "tags", "database query failed");
    })?;

    Ok(count > 0)
}

/// Check if the tag slug exists.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn tag_slug_exists(executor: impl PgExecutor<'_>, slug: &str) -> Result<bool> {
    let count: i64 = query_scalar!(
        r#"SELECT COUNT(*) AS "count!" FROM tags WHERE slug = $1"#,
        slug
    )
    .fetch_one(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "select", table = "tags", "database query failed");
    })?;

    Ok(count > 0)
}

/// Check if the tag slug exists for a different tag.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn tag_slug_exists_for_other_tag(
    executor: impl PgExecutor<'_>,
    slug: &str,
    tag_id: i64,
) -> Result<bool> {
    let count: i64 = query_scalar!(
        r#"SELECT COUNT(*) AS "count!" FROM tags WHERE slug = $1 AND id <> $2"#,
        slug,
        tag_id
    )
    .fetch_one(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "select", table = "tags", "database query failed");
    })?;

    Ok(count > 0)
}

/* ********************************************************************************************** *
 *                                             Update                                             *
 * ********************************************************************************************** */

/// Update existing tag.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to write to database.
#[instrument(skip(executor))]
pub async fn update_tag(
    executor: impl PgExecutor<'_>,
    id: i64,
    name: &str,
    slug: &str,
) -> Result<()> {
    query!(
        "UPDATE tags
         SET name = $1, slug = $2
         WHERE id = $3",
        name,
        slug,
        id
    )
    .execute(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "update", table = "tags", "database query failed");
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;

    #[sqlx::test]
    async fn tag_crud_and_unique_checks_match_the_schema(pool: PgPool) -> Result<()> {
        let id = create_tag(&pool, "Rust", "rust").await?;
        assert!(tag_name_exists(&pool, "Rust").await?);
        assert!(tag_slug_exists(&pool, "rust").await?);
        assert!(!tag_name_exists_for_other_tag(&pool, "Rust", id).await?);
        assert!(!tag_slug_exists_for_other_tag(&pool, "rust", id).await?);
        assert!(tag_name_exists_for_other_tag(&pool, "Rust", i64::MAX).await?);
        assert!(tag_slug_exists_for_other_tag(&pool, "rust", i64::MAX).await?);
        assert!(create_tag(&pool, "Other", "rust").await.is_err());
        assert!(create_tag(&pool, "Rust", "other").await.is_err());

        update_tag(&pool, id, "PostgreSQL", "postgresql").await?;
        let tag = get_tag_by_id(&pool, id)
            .await?
            .ok_or_else(|| crate::Error::System("missing tag".into()))?;
        assert_eq!(tag.name, "PostgreSQL");
        assert_eq!(
            get_tag_by_slug(&pool, "postgresql")
                .await?
                .map(|tag| tag.id),
            Some(id)
        );
        assert!(!tag_slug_exists(&pool, "rust").await?);
        delete_tag(&pool, id).await?;
        assert!(get_tag_by_id(&pool, id).await?.is_none());
        Ok(())
    }
}
