//! Tag database operations.

use sqlx::{MySqlExecutor, query, query_as, query_scalar};

use tracing::instrument;

use crate::{
    Error, Result,
    database::{Tag, TagWithCount},
};

/* ********************************************************************************************** *
 *                                             Create                                             *
 * ********************************************************************************************** */

/// Create a new tag.
///
/// # Errors
/// Returns [`Error::DatabaseInsertError`] on failure to write to database.
#[instrument(skip(executor))]
pub async fn create_tag(executor: impl MySqlExecutor<'_>, name: &str, slug: &str) -> Result<u64> {
    query("INSERT INTO tags (name, slug) VALUES (?, ?)")
        .bind(name)
        .bind(slug)
        .execute(executor)
        .await
        .map(|result| result.last_insert_id())
        .map_err(|sql_err| Error::DatabaseInsertError(sql_err.to_string()))
}

/* ********************************************************************************************** *
 *                                             Delete                                             *
 * ********************************************************************************************** */

/// Delete tag.
///
/// # Errors
/// Returns [`Error::DatabaseDeleteError`] on failure to write to database.
#[instrument(skip(executor))]
pub async fn delete_tag(executor: impl MySqlExecutor<'_>, id: u64) -> Result<()> {
    query("DELETE FROM tags WHERE id = ?")
        .bind(id)
        .execute(executor)
        .await
        .map(|_| ())
        .map_err(|sql_err| Error::DatabaseDeleteError(sql_err.to_string()))
}

/* ********************************************************************************************** *
 *                                              Read                                              *
 * ********************************************************************************************** */

/// Get tag by id.
///
/// # Errors
/// Returns [`Error::DatabaseSelectError`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn get_tag_by_id(executor: impl MySqlExecutor<'_>, id: u64) -> Result<Option<Tag>> {
    query_as::<_, Tag>("SELECT id, name, slug FROM tags WHERE id = ?")
        .bind(id)
        .fetch_optional(executor)
        .await
        .map_err(|sql_err| Error::DatabaseSelectError(sql_err.to_string()))
}

/// Get tag by slug.
///
/// # Errors
/// Returns [`Error::DatabaseSelectError`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn get_tag_by_slug(executor: impl MySqlExecutor<'_>, slug: &str) -> Result<Option<Tag>> {
    query_as::<_, Tag>("SELECT id, name, slug FROM tags WHERE slug = ?")
        .bind(slug)
        .fetch_optional(executor)
        .await
        .map_err(|sql_err| Error::DatabaseSelectError(sql_err.to_string()))
}

/// Get tags for a post.
///
/// # Errors
/// Returns [`Error::DatabaseSelectError`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn list_tags_for_post(
    executor: impl MySqlExecutor<'_>,
    post_id: u64,
) -> Result<Vec<Tag>> {
    query_as::<_, Tag>(
        "SELECT t.id, t.name, t.slug
         FROM tags t
         JOIN post_tags pt ON pt.tag_id = t.id
         WHERE pt.post_id = ?
         ORDER BY t.name",
    )
    .bind(post_id)
    .fetch_all(executor)
    .await
    .map_err(|sql_err| Error::DatabaseSelectError(sql_err.to_string()))
}

/// List tags with their counts.
///
/// # Errors
/// Returns [`Error::DatabaseSelectError`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn list_tags_with_count(executor: impl MySqlExecutor<'_>) -> Result<Vec<TagWithCount>> {
    query_as::<_, TagWithCount>(
        "SELECT t.id, t.name, t.slug, COUNT(pt.post_id) AS count
         FROM tags t
         LEFT JOIN post_tags pt ON pt.tag_id = t.id
         GROUP BY t.id, t.name, t.slug
         ORDER BY t.name",
    )
    .fetch_all(executor)
    .await
    .map_err(|sql_err| Error::DatabaseSelectError(sql_err.to_string()))
}

/// List tags with their published counts.
///
/// # Errors
/// Returns [`Error::DatabaseSelectError`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn list_tags_with_published_count(
    executor: impl MySqlExecutor<'_>,
) -> Result<Vec<TagWithCount>> {
    query_as::<_, TagWithCount>(
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
    .map_err(|sql_err| Error::DatabaseSelectError(sql_err.to_string()))
}

/// Check if the tag name already exists.
///
/// # Errors
/// Returns [`Error::DatabaseSelectError`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn tag_name_exists(executor: impl MySqlExecutor<'_>, name: &str) -> Result<bool> {
    query_scalar("SELECT COUNT(*) FROM tags WHERE name = ?")
        .bind(name)
        .fetch_one(executor)
        .await
        .map(|count: i64| count > 0)
        .map_err(|sql_err| Error::DatabaseSelectError(sql_err.to_string()))
}

/// Check if the tag name already exists for a different tag.
///
/// # Errors
/// Returns [`Error::DatabaseSelectError`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn tag_name_exists_for_other_tag(
    executor: impl MySqlExecutor<'_>,
    name: &str,
    tag_id: u64,
) -> Result<bool> {
    query_scalar("SELECT COUNT(*) FROM tags WHERE name = ? AND id <> ?")
        .bind(name)
        .bind(tag_id)
        .fetch_one(executor)
        .await
        .map(|count: i64| count > 0)
        .map_err(|sql_err| Error::DatabaseSelectError(sql_err.to_string()))
}

/// Check if the tag slug exists.
///
/// # Errors
/// Returns [`Error::DatabaseSelectError`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn tag_slug_exists(executor: impl MySqlExecutor<'_>, slug: &str) -> Result<bool> {
    query_scalar("SELECT COUNT(*) FROM tags WHERE slug = ?")
        .bind(slug)
        .fetch_one(executor)
        .await
        .map(|count: i64| count > 0)
        .map_err(|sql_err| Error::DatabaseSelectError(sql_err.to_string()))
}

/// Check if the tag slug exists for a different tag.
///
/// # Errors
/// Returns [`Error::DatabaseSelectError`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn tag_slug_exists_for_other_tag(
    executor: impl MySqlExecutor<'_>,
    slug: &str,
    tag_id: u64,
) -> Result<bool> {
    sqlx::query_scalar("SELECT COUNT(*) FROM tags WHERE slug = ? AND id <> ?")
        .bind(slug)
        .bind(tag_id)
        .fetch_one(executor)
        .await
        .map(|count: i64| count > 0)
        .map_err(|sql_err| Error::DatabaseSelectError(sql_err.to_string()))
}

/* ********************************************************************************************** *
 *                                             Update                                             *
 * ********************************************************************************************** */

/// Update existing tag.
///
/// # Errors
/// Returns [`Error::DatabaseUpdateError`] on failure to write to database.
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
    .map(|_| ())
    .map_err(|sql_err| Error::DatabaseUpdateError(sql_err.to_string()))
}
