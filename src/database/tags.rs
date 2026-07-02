//! Tag database operations.

use sqlx::{MySqlExecutor, query, query_as, query_scalar};

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
pub async fn create_tag(executor: impl MySqlExecutor<'_>, name: &str, slug: &str) -> Result<u64> {
    let result = query("INSERT INTO tags (name, slug) VALUES (?, ?)")
        .bind(name)
        .bind(slug)
        .execute(executor)
        .await
        .inspect_err(|error| {
            error!(%error, operation = "insert", table = "tags", "database query failed");
        })?;

    Ok(result.last_insert_id())
}

/* ********************************************************************************************** *
 *                                             Delete                                             *
 * ********************************************************************************************** */

/// Delete tag.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to write to database.
#[instrument(skip(executor))]
pub async fn delete_tag(executor: impl MySqlExecutor<'_>, id: u64) -> Result<()> {
    query("DELETE FROM tags WHERE id = ?")
        .bind(id)
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
pub async fn get_tag_by_id(executor: impl MySqlExecutor<'_>, id: u64) -> Result<Option<Tag>> {
    let tag = query_as::<_, Tag>("SELECT id, name, slug FROM tags WHERE id = ?")
        .bind(id)
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
pub async fn get_tag_by_slug(executor: impl MySqlExecutor<'_>, slug: &str) -> Result<Option<Tag>> {
    let tag = query_as::<_, Tag>("SELECT id, name, slug FROM tags WHERE slug = ?")
        .bind(slug)
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
pub async fn list_tags_for_post(
    executor: impl MySqlExecutor<'_>,
    post_id: u64,
) -> Result<Vec<Tag>> {
    let tags = query_as::<_, Tag>(
        "SELECT t.id, t.name, t.slug
         FROM tags t
         JOIN post_tags pt ON pt.tag_id = t.id
         WHERE pt.post_id = ?
         ORDER BY t.name",
    )
    .bind(post_id)
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
pub async fn list_tags_with_count(executor: impl MySqlExecutor<'_>) -> Result<Vec<TagWithCount>> {
    let tags = query_as::<_, TagWithCount>(
        "SELECT t.id, t.name, t.slug, COUNT(pt.post_id) AS count
         FROM tags t
         LEFT JOIN post_tags pt ON pt.tag_id = t.id
         GROUP BY t.id, t.name, t.slug
         ORDER BY t.name",
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
    executor: impl MySqlExecutor<'_>,
) -> Result<Vec<TagWithCount>> {
    let tags = query_as::<_, TagWithCount>(
        "SELECT t.id, t.name, t.slug, COUNT(p.id) AS `count`
         FROM tags t
         LEFT JOIN post_tags pt ON pt.tag_id = t.id
         LEFT JOIN posts p ON p.id = pt.post_id
            AND p.published_at IS NOT NULL
            AND p.published_at <= NOW()
         GROUP BY t.name, t.slug
         ORDER BY t.name",
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
pub async fn tag_name_exists(executor: impl MySqlExecutor<'_>, name: &str) -> Result<bool> {
    let count: i64 = query_scalar("SELECT COUNT(*) FROM tags WHERE name = ?")
        .bind(name)
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
    executor: impl MySqlExecutor<'_>,
    name: &str,
    tag_id: u64,
) -> Result<bool> {
    let count: i64 = query_scalar("SELECT COUNT(*) FROM tags WHERE name = ? AND id <> ?")
        .bind(name)
        .bind(tag_id)
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
pub async fn tag_slug_exists(executor: impl MySqlExecutor<'_>, slug: &str) -> Result<bool> {
    let count: i64 = query_scalar("SELECT COUNT(*) FROM tags WHERE slug = ?")
        .bind(slug)
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
    executor: impl MySqlExecutor<'_>,
    slug: &str,
    tag_id: u64,
) -> Result<bool> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tags WHERE slug = ? AND id <> ?")
        .bind(slug)
        .bind(tag_id)
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
    executor: impl MySqlExecutor<'_>,
    id: u64,
    name: &str,
    slug: &str,
) -> Result<()> {
    query(
        "UPDATE tags
         SET name = ?, slug = ?
         WHERE id = ?",
    )
    .bind(name)
    .bind(slug)
    .bind(id)
    .execute(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "update", table = "tags", "database query failed");
    })?;

    Ok(())
}
