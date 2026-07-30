//! Post database operations.

use chrono::NaiveDateTime;
use sqlx::{FromRow, MySqlExecutor, query, query_as, query_scalar};

use tracing::{error, instrument};

use crate::{
    Result,
    database::{Post, PostSort, PostStatusFilter, SqlClause, Tag},
};

/// Post with tag database row.
#[derive(Debug, FromRow)]
struct PostWithTagRow {
    pub id: u64,
    pub title: String,
    pub slug: String,
    pub body: String,
    pub preview: Option<String>,
    pub created_at: NaiveDateTime,
    pub published_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub tag_id: Option<u64>,
    pub tag_name: Option<String>,
    pub tag_slug: Option<String>,
}

/* ********************************************************************************************** *
 *                                             Create                                             *
 * ********************************************************************************************** */

/// Create a new post.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to write to database.
#[instrument(skip(executor, body, preview))]
pub async fn create_post(
    executor: impl MySqlExecutor<'_>,
    title: &str,
    slug: &str,
    body: &str,
    preview: Option<&str>,
    published_at: Option<NaiveDateTime>,
) -> Result<u64> {
    let result = query(
        "INSERT INTO posts (title, slug, body, preview, published_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(title)
    .bind(slug)
    .bind(body)
    .bind(preview)
    .bind(published_at)
    .execute(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "insert", table = "posts", "database query failed");
    })?;

    Ok(result.last_insert_id())
}

/* ********************************************************************************************** *
 *                                             Delete                                             *
 * ********************************************************************************************** */

/// Delete post.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to write to database.
#[instrument(skip(executor))]
pub async fn delete_post(executor: impl MySqlExecutor<'_>, post_id: u64) -> Result<()> {
    query("DELETE FROM posts WHERE id = ?")
        .bind(post_id)
        .execute(executor)
        .await
        .inspect_err(|error| {
            error!(%error, operation = "delete", table = "posts", "database query failed");
        })?;

    Ok(())
}

/* ********************************************************************************************** *
 *                                              Read                                              *
 * ********************************************************************************************** */

/// Count number of posts which have been published.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn count_published_posts(executor: impl MySqlExecutor<'_>) -> Result<u32> {
    let total: i32 = query_scalar(
        "SELECT COUNT(*) FROM posts WHERE published_at IS NOT NULL AND published_at <= NOW()",
    )
    .fetch_one(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "select", table = "posts", "database query failed");
    })?;

    Ok(total.cast_unsigned())
}

/// Get post (published or unpublished) by id.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn get_post_by_id(executor: impl MySqlExecutor<'_>, id: u64) -> Result<Option<Post>> {
    let rows = query_as::<_, PostWithTagRow>(
        "SELECT p.id, p.title, p.slug, p.body, p.preview, p.published_at, p.created_at,\
                p.updated_at, t.id AS tag_id, t.name AS tag_name, t.slug AS tag_slug
         FROM (
             SELECT id, title, slug, body, preview, published_at, created_at, updated_at
             FROM posts
             WHERE id = ?
         ) p
         LEFT JOIN post_tags pt ON pt.post_id = p.id
         LEFT JOIN tags t ON t.id = pt.tag_id
         ORDER BY t.name",
    )
    .bind(id)
    .fetch_all(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "select", table = "posts", "database query failed");
    })?;

    Ok(merge_tags_to_option(rows))
}

/// Get post by slug.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn get_post_by_slug(
    executor: impl MySqlExecutor<'_>,
    slug: &str,
) -> Result<Option<Post>> {
    let rows = query_as::<_, PostWithTagRow>(
        "SELECT p.id, p.title, p.slug, p.body, p.preview, p.published_at, p.created_at,\
                p.updated_at, t.id AS tag_id, t.name AS tag_name, t.slug AS tag_slug
         FROM (
             SELECT id, title, slug, body, preview, published_at, created_at, updated_at
             FROM posts
             WHERE slug = ? AND published_at IS NOT NULL AND published_at <= NOW()
         ) p
         LEFT JOIN post_tags pt ON pt.post_id = p.id
         LEFT JOIN tags t ON t.id = pt.tag_id
         ORDER BY t.name",
    )
    .bind(slug)
    .fetch_all(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "select", table = "posts", "database query failed");
    })?;

    Ok(merge_tags_to_option(rows))
}

/// List posts (possibly including unpublished) for admin dashboard.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn list_posts(
    executor: impl MySqlExecutor<'_>,
    filter: &PostStatusFilter,
    page: u32,
    page_size: u32,
    sort: &PostSort,
) -> Result<Vec<Post>> {
    let offset = (page.saturating_sub(1)) * page_size;
    let order_clause = sort.get_sql_clause();
    let where_clause = filter.get_sql_clause();
    let sql = format!(
        "SELECT p.id, p.title, p.slug, p.body, p.preview, p.published_at, p.created_at,\
                p.updated_at, t.id AS tag_id, t.name AS tag_name, t.slug AS tag_slug
         FROM (
             SELECT id, title, slug, body, preview, published_at, created_at, updated_at
             FROM posts
             WHERE published_at IS NOT NULL AND published_at <= NOW()
             ORDER BY published_at DESC
             LIMIT ? OFFSET ?
         ) p
         LEFT JOIN post_tags pt ON pt.post_id = p.id
         LEFT JOIN tags t ON t.id = pt.tag_id
         {where_clause}
         ORDER BY {order_clause}, p.id"
    );

    let rows = query_as::<_, PostWithTagRow>(&sql)
        .bind(page_size)
        .bind(offset)
        .fetch_all(executor)
        .await
        .inspect_err(|error| {
            error!(%error, operation = "select", table = "posts", "database query failed");
        })?;

    Ok(merge_tags(rows))
}

/// Get posts which have a tag.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn list_posts_by_tag(
    executor: impl MySqlExecutor<'_>,
    tag_slug: &str,
) -> Result<Vec<Post>> {
    let rows = query_as::<_, PostWithTagRow>(
        "SELECT p.id, p.title, p.slug, p.body, p.preview, p.published_at, p.created_at, p.updated_at,
                t.id AS tag_id, t.name AS tag_name, t.slug AS tag_slug
         FROM posts p
         JOIN post_tags pt_filter ON pt_filter.post_id = p.id
         JOIN tags t_filter ON t_filter.id = pt_filter.tag_id
         LEFT JOIN post_tags pt ON pt.post_id = p.id
         LEFT JOIN tags t ON t.id = pt.tag_id
         WHERE t_filter.slug = ?
           AND p.published_at IS NOT NULL AND p.published_at <= NOW()
         ORDER BY p.published_at DESC, p.id, t.name")
    .bind(tag_slug)
    .fetch_all(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "select", table = "posts", "database query failed");
    })?;

    Ok(merge_tags(rows))
}

/// List a page of posts.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn list_published_posts(
    executor: impl MySqlExecutor<'_>,
    page: u32,
    page_size: u32,
) -> Result<Vec<Post>> {
    let offset = (page.saturating_sub(1)) * page_size;

    let rows = query_as::<_, PostWithTagRow>(
        "SELECT p.id, p.title, p.slug, p.body, p.preview, p.published_at, p.created_at,\
                p.updated_at, t.id AS tag_id, t.name AS tag_name, t.slug AS tag_slug
         FROM (
             SELECT id, title, slug, body, preview, published_at, created_at, updated_at
             FROM posts
             WHERE published_at IS NOT NULL AND published_at <= NOW()
             ORDER BY published_at DESC
             LIMIT ? OFFSET ?
         ) p
         LEFT JOIN post_tags pt ON pt.post_id = p.id
         LEFT JOIN tags t ON t.id = pt.tag_id
         ORDER BY p.published_at DESC, p.id, t.name",
    )
    .bind(page_size)
    .bind(offset)
    .fetch_all(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "select", table = "posts", "database query failed");
    })?;

    Ok(merge_tags(rows))
}

/// Check if the provided slug already exists for a post.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn slug_exists(executor: impl MySqlExecutor<'_>, slug: &str) -> Result<bool> {
    let count: i64 = query_scalar("SELECT COUNT(*) FROM posts WHERE slug = ?")
        .bind(slug)
        .fetch_one(executor)
        .await
        .inspect_err(|error| {
            error!(%error, operation = "select", table = "posts", "database query failed");
        })?;

    Ok(count > 0)
}

/// Check if the provided slug already exists for a different post.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn slug_exists_for_other_post(
    executor: impl MySqlExecutor<'_>,
    slug: &str,
    post_id: u64,
) -> Result<bool> {
    let count: i64 = query_scalar("SELECT COUNT(*) FROM posts WHERE slug = ? AND id <> ?")
        .bind(slug)
        .bind(post_id)
        .fetch_one(executor)
        .await
        .inspect_err(|error| {
            error!(%error, operation = "select", table = "posts", "database query failed");
        })?;

    Ok(count > 0)
}

/* ********************************************************************************************** *
 *                                             Update                                             *
 * ********************************************************************************************** */

/// Update existing post.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to write to database.
#[instrument(skip(executor, body, preview))]
pub async fn update_post(
    executor: impl MySqlExecutor<'_>,
    post_id: u64,
    title: &str,
    slug: &str,
    body: &str,
    preview: Option<&str>,
    published_at: Option<NaiveDateTime>,
) -> Result<()> {
    query(
        "UPDATE posts
         SET title = ?, slug = ?, body = ?, preview = ?, published_at = ?, updated_at = NOW()
         WHERE id = ?",
    )
    .bind(title)
    .bind(slug)
    .bind(body)
    .bind(preview)
    .bind(published_at)
    .bind(post_id)
    .execute(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "update", table = "posts", "database query failed");
    })?;

    Ok(())
}

/* ********************************************************************************************** *
 *                                       Private functions                                        *
 * ********************************************************************************************** */

fn merge_tags(rows: Vec<PostWithTagRow>) -> Vec<Post> {
    let mut current_post_id = None;
    let mut posts: Vec<Post> = Vec::new();

    for row in rows {
        if current_post_id != Some(row.id) {
            current_post_id = Some(row.id);

            posts.push(Post {
                id: row.id,
                title: row.title,
                slug: row.slug,
                body: row.body,
                preview: row.preview,
                created_at: row.created_at,
                published_at: row.published_at,
                updated_at: row.updated_at,
                tags: Vec::new(),
            });
        }

        if let (Some(post), Some(id), Some(name), Some(slug)) =
            (posts.last_mut(), row.tag_id, row.tag_name, row.tag_slug)
        {
            post.tags.push(Tag { id, name, slug });
        }
    }

    posts
}

fn merge_tags_to_option(rows: Vec<PostWithTagRow>) -> Option<Post> {
    rows.into_iter()
        .map(
            |PostWithTagRow {
                 id,
                 title,
                 slug,
                 body,
                 preview,
                 created_at,
                 published_at,
                 updated_at,
                 tag_id,
                 tag_name,
                 tag_slug,
             }| {
                let tags = match (tag_id, tag_name, tag_slug) {
                    (Some(id), Some(name), Some(slug)) => vec![Tag { id, name, slug }],
                    _ => Vec::new(),
                };

                Post {
                    id,
                    title,
                    slug,
                    body,
                    preview,
                    created_at,
                    published_at,
                    updated_at,
                    tags,
                }
            },
        )
        .reduce(|mut accumulator, row| {
            accumulator.tags.extend_from_slice(&row.tags);

            accumulator
        })
}
