//! Post database operations.

use chrono::{DateTime, Utc};
use sqlx::{PgExecutor, query, query_as, query_scalar};

use tracing::{error, instrument};

use crate::{
    Result,
    database::{Post, PostSort, PostStatusFilter, Tag},
};

/// Post with tag database row.
#[derive(Debug)]
struct PostWithTagRow {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub body: String,
    pub preview: Option<String>,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub tag_id: Option<i64>,
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
    executor: impl PgExecutor<'_>,
    title: &str,
    slug: &str,
    body: &str,
    preview: Option<&str>,
    published_at: Option<DateTime<Utc>>,
) -> Result<i64> {
    let id = query_scalar!("INSERT INTO posts (title, slug, body, preview, published_at) VALUES ($1, $2, $3, $4, $5) RETURNING id", title, slug, body, preview, published_at)
    .fetch_one(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "insert", table = "posts", "database query failed");
    })?;

    Ok(id)
}

/* ********************************************************************************************** *
 *                                             Delete                                             *
 * ********************************************************************************************** */

/// Delete post.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to write to database.
#[instrument(skip(executor))]
pub async fn delete_post(executor: impl PgExecutor<'_>, post_id: i64) -> Result<()> {
    query!("DELETE FROM posts WHERE id = $1", post_id)
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
pub async fn count_published_posts(executor: impl PgExecutor<'_>) -> Result<u32> {
    let total: i64 = query_scalar!(r#"SELECT COUNT(*) AS "count!" FROM posts WHERE published_at IS NOT NULL AND published_at <= NOW()"#)
    .fetch_one(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "select", table = "posts", "database query failed");
    })?;

    count_as_u32(total)
}

/// Count posts matching an admin dashboard status filter.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn count_posts(executor: impl PgExecutor<'_>, filter: &PostStatusFilter) -> Result<u32> {
    let total = query_scalar!(
        r#"SELECT COUNT(*) AS "count!" FROM posts p
           WHERE $1 = 'all'
              OR ($1 = 'published' AND p.published_at <= NOW())
              OR ($1 = 'scheduled' AND p.published_at > NOW())
              OR ($1 = 'unscheduled' AND p.published_at IS NULL)"#,
        filter_key(filter),
    )
    .fetch_one(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "select", table = "posts", "database query failed");
    })?;

    count_as_u32(total)
}

/// Get post (published or unpublished) by id.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn get_post_by_id(executor: impl PgExecutor<'_>, id: i64) -> Result<Option<Post>> {
    let rows = query_as!(
        PostWithTagRow,
        r#"SELECT p.id AS "id!", p.title AS "title!", p.slug AS "slug!", p.body AS "body!",
                p.preview, p.published_at, p.created_at AS "created_at!", p.updated_at,
                t.id AS "tag_id?", t.name AS "tag_name?", t.slug AS "tag_slug?"
         FROM posts p
         LEFT JOIN post_tags pt ON pt.post_id = p.id
         LEFT JOIN tags t ON t.id = pt.tag_id
         WHERE p.id = $1
         ORDER BY t.name"#,
        id,
    )
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
pub async fn get_post_by_slug(executor: impl PgExecutor<'_>, slug: &str) -> Result<Option<Post>> {
    let rows = query_as!(
        PostWithTagRow,
        r#"SELECT p.id AS "id!", p.title AS "title!", p.slug AS "slug!", p.body AS "body!",
                p.preview, p.published_at, p.created_at AS "created_at!", p.updated_at,
                t.id AS "tag_id?", t.name AS "tag_name?", t.slug AS "tag_slug?"
         FROM posts p
         LEFT JOIN post_tags pt ON pt.post_id = p.id
         LEFT JOIN tags t ON t.id = pt.tag_id
         WHERE p.slug = $1 AND p.published_at <= NOW()
         ORDER BY t.name"#,
        slug,
    )
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
    executor: impl PgExecutor<'_>,
    filter: &PostStatusFilter,
    page: u32,
    page_size: u32,
    sort: &PostSort,
) -> Result<Vec<Post>> {
    let offset = page_offset(page, page_size)?;

    // Limit posts before joining their tags so a post with many tags occupies one page slot.
    // Explicit null placement preserves the MySQL admin ordering: drafts last descending,
    // first ascending. The ID tie-breaker is identical inside and outside the page query.
    let rows = query_as!(
        PostWithTagRow,
        r#"SELECT p.id AS "id!", p.title AS "title!", p.slug AS "slug!", p.body AS "body!",
                p.preview, p.published_at, p.created_at AS "created_at!", p.updated_at,
                t.id AS "tag_id?", t.name AS "tag_name?", t.slug AS "tag_slug?"
         FROM (
             SELECT id, title, slug, body, preview, published_at, created_at, updated_at
             FROM posts
             WHERE $1 = 'all'
                OR ($1 = 'published' AND published_at <= NOW())
                OR ($1 = 'scheduled' AND published_at > NOW())
                OR ($1 = 'unscheduled' AND published_at IS NULL)
             ORDER BY
                 CASE WHEN $2 = 'published_desc' THEN published_at END DESC NULLS LAST,
                 CASE WHEN $2 = 'published_asc' THEN published_at END ASC NULLS FIRST,
                 CASE WHEN $2 = 'created_desc' THEN created_at END DESC,
                 CASE WHEN $2 = 'created_asc' THEN created_at END ASC,
                 id
             LIMIT $3 OFFSET $4
         ) p
         LEFT JOIN post_tags pt ON pt.post_id = p.id
         LEFT JOIN tags t ON t.id = pt.tag_id
         ORDER BY
             CASE WHEN $2 = 'published_desc' THEN p.published_at END DESC NULLS LAST,
             CASE WHEN $2 = 'published_asc' THEN p.published_at END ASC NULLS FIRST,
             CASE WHEN $2 = 'created_desc' THEN p.created_at END DESC,
             CASE WHEN $2 = 'created_asc' THEN p.created_at END ASC,
             p.id, t.name"#,
        filter_key(filter),
        sort_key(sort),
        i64::from(page_size),
        offset,
    )
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
    executor: impl PgExecutor<'_>,
    tag_slug: &str,
    page: u32,
    page_size: u32,
) -> Result<Vec<Post>> {
    let offset = page_offset(page, page_size)?;
    // Tag cards only use previews; do not load full Markdown bodies.
    let rows = query_as!(
        PostWithTagRow,
        r#"SELECT p.id AS "id!", p.title AS "title!", p.slug AS "slug!", '' AS "body!",
                p.preview, p.published_at, p.created_at AS "created_at!", p.updated_at,
                t.id AS "tag_id?", t.name AS "tag_name?", t.slug AS "tag_slug?"
         FROM (
             SELECT p.id, p.title, p.slug, p.preview, p.published_at, p.created_at, p.updated_at
             FROM posts p
             JOIN post_tags pt ON pt.post_id = p.id
             JOIN tags t ON t.id = pt.tag_id
             WHERE t.slug = $1 AND p.published_at <= NOW()
             ORDER BY p.published_at DESC, p.id
             LIMIT $2 OFFSET $3
         ) p
         LEFT JOIN post_tags pt ON pt.post_id = p.id
         LEFT JOIN tags t ON t.id = pt.tag_id
         ORDER BY p.published_at DESC, p.id, t.name"#,
        tag_slug,
        i64::from(page_size),
        offset,
    )
    .fetch_all(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "select", table = "posts", "database query failed");
    })?;
    Ok(merge_tags(rows))
}

/// Count published posts belonging to a tag.
///
/// # Errors
/// Returns a database error if the query fails.
#[instrument(skip(executor))]
pub async fn count_posts_by_tag(executor: impl PgExecutor<'_>, tag_slug: &str) -> Result<u32> {
    let count = query_scalar!(
        r#"SELECT COUNT(*) AS "count!" FROM posts p
           JOIN post_tags pt ON pt.post_id = p.id
           JOIN tags t ON t.id = pt.tag_id
           WHERE t.slug = $1 AND p.published_at <= NOW()"#,
        tag_slug,
    )
    .fetch_one(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "count tagged posts", "database query failed");
    })?;
    count_as_u32(count)
}

pub struct RelatedPost {
    pub title: String,
    pub slug: String,
}

/// Return up to four published posts with shared tags, most overlap first.
///
/// # Errors
/// Returns a database error if the query fails.
#[instrument(skip(executor))]
pub async fn related_posts(
    executor: impl PgExecutor<'_>,
    post_id: i64,
) -> Result<Vec<RelatedPost>> {
    Ok(query_as!(
        RelatedPost,
        r#"SELECT p.title, p.slug FROM posts p
           JOIN post_tags candidate ON candidate.post_id = p.id
           JOIN post_tags current ON current.tag_id = candidate.tag_id AND current.post_id = $1
           WHERE p.id <> $1 AND p.published_at <= NOW()
           GROUP BY p.id
           ORDER BY COUNT(*) DESC, p.published_at DESC, p.id
           LIMIT 4"#,
        post_id,
    )
    .fetch_all(executor)
    .await
    .inspect_err(|error| {
        error!(%error, operation = "related posts", "database query failed");
    })?)
}

/// List a page of posts.
///
/// # Errors
/// Returns [`crate::Error::Database`] on failure to read from database.
#[instrument(skip(executor))]
pub async fn list_published_posts(
    executor: impl PgExecutor<'_>,
    page: u32,
    page_size: u32,
) -> Result<Vec<Post>> {
    let offset = page_offset(page, page_size)?;

    let rows = query_as!(
        PostWithTagRow,
        r#"SELECT p.id AS "id!", p.title AS "title!", p.slug AS "slug!", p.body AS "body!",
                p.preview, p.published_at, p.created_at AS "created_at!", p.updated_at,
                t.id AS "tag_id?", t.name AS "tag_name?", t.slug AS "tag_slug?"
         FROM (
             SELECT id, title, slug, body, preview, published_at, created_at, updated_at
             FROM posts
             WHERE published_at <= NOW()
             ORDER BY published_at DESC, id
             LIMIT $1 OFFSET $2
         ) p
         LEFT JOIN post_tags pt ON pt.post_id = p.id
         LEFT JOIN tags t ON t.id = pt.tag_id
         ORDER BY p.published_at DESC, p.id, t.name"#,
        i64::from(page_size),
        offset,
    )
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
pub async fn slug_exists(executor: impl PgExecutor<'_>, slug: &str) -> Result<bool> {
    let count: i64 = query_scalar!(
        r#"SELECT COUNT(*) AS "count!" FROM posts WHERE slug = $1"#,
        slug
    )
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
    executor: impl PgExecutor<'_>,
    slug: &str,
    post_id: i64,
) -> Result<bool> {
    let count: i64 = query_scalar!(
        r#"SELECT COUNT(*) AS "count!" FROM posts WHERE slug = $1 AND id <> $2"#,
        slug,
        post_id
    )
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
    executor: impl PgExecutor<'_>,
    post_id: i64,
    title: &str,
    slug: &str,
    body: &str,
    preview: Option<&str>,
    published_at: Option<DateTime<Utc>>,
) -> Result<()> {
    query!(
        "UPDATE posts
         SET title = $1, slug = $2, body = $3, preview = $4, published_at = $5, updated_at = NOW()
         WHERE id = $6",
        title,
        slug,
        body,
        preview,
        published_at,
        post_id
    )
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

const fn filter_key(filter: &PostStatusFilter) -> &'static str {
    match filter {
        PostStatusFilter::All => "all",
        PostStatusFilter::Published => "published",
        PostStatusFilter::Scheduled => "scheduled",
        PostStatusFilter::Unscheduled => "unscheduled",
    }
}

const fn sort_key(sort: &PostSort) -> &'static str {
    match sort {
        PostSort::PublishedAtDesc => "published_desc",
        PostSort::PublishedAtAsc => "published_asc",
        PostSort::CreatedAtDesc => "created_desc",
        PostSort::CreatedAtAsc => "created_asc",
    }
}

fn count_as_u32(count: i64) -> Result<u32> {
    u32::try_from(count).map_err(|error| sqlx::Error::Decode(Box::new(error)).into())
}

fn page_offset(page: u32, page_size: u32) -> Result<i64> {
    let offset = u64::from(page.saturating_sub(1)) * u64::from(page_size);
    i64::try_from(offset).map_err(|error| sqlx::Error::Decode(Box::new(error)).into())
}

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

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use sqlx::PgPool;

    use super::*;
    use crate::database::{
        create_post_with_tags, create_tag, list_tags_with_count, list_tags_with_published_count,
    };

    #[sqlx::test]
    async fn pagination_keeps_complete_posts_and_excludes_unpublished(pool: PgPool) -> Result<()> {
        let alpha = create_tag(&pool, "Alpha", "alpha").await?;
        let zeta = create_tag(&pool, "Zeta", "zeta").await?;
        let empty = create_tag(&pool, "Unused", "unused").await?;
        let published_at = Utc::now() - Duration::days(1);
        let mut published_ids = Vec::new();

        // Equal timestamps exercise the ID tie-breaker across the page boundary.
        for number in 0..11 {
            let tags = if number == 10 {
                vec![]
            } else {
                vec![zeta, alpha]
            };
            let id = create_post_with_tags(
                &pool,
                &format!("Post {number}"),
                &format!("post-{number}"),
                "Body",
                Some("Preview"),
                Some(published_at),
                &tags,
            )
            .await?;
            published_ids.push(id);
        }
        for (slug, published_at) in [
            ("draft", None),
            ("scheduled", Some(Utc::now() + Duration::days(1))),
        ] {
            create_post_with_tags(&pool, slug, slug, "Body", None, published_at, &[alpha]).await?;
        }

        assert_eq!(count_published_posts(&pool).await?, 11);
        let first = list_published_posts(&pool, 1, 8).await?;
        let second = list_published_posts(&pool, 2, 8).await?;
        assert_eq!(first.len(), 8);
        assert_eq!(second.len(), 3);
        let actual: Vec<_> = first.iter().chain(&second).map(|post| post.id).collect();
        assert_eq!(actual, published_ids);
        for post in first.iter().chain(second.iter().take(2)) {
            assert_eq!(
                post.tags.iter().map(|tag| tag.id).collect::<Vec<_>>(),
                [alpha, zeta]
            );
        }
        assert!(second[2].tags.is_empty());
        assert!(list_published_posts(&pool, 3, 8).await?.is_empty());
        assert_eq!(
            list_published_posts(&pool, 0, 8).await?[0].id,
            published_ids[0]
        );
        assert!(get_post_by_slug(&pool, "draft").await?.is_none());
        assert!(get_post_by_slug(&pool, "scheduled").await?.is_none());
        let post = get_post_by_slug(&pool, "post-0").await?;
        assert_eq!(post.map(|post| post.tags.len()), Some(2));

        let tagged = list_posts_by_tag(&pool, "alpha", 1, 8).await?;
        assert_eq!(tagged.len(), 8);
        let second_tagged = list_posts_by_tag(&pool, "alpha", 2, 8).await?;
        assert_eq!(second_tagged.len(), 2);
        assert_eq!(count_posts_by_tag(&pool, "alpha").await?, 10);
        assert_eq!(
            tagged
                .iter()
                .chain(&second_tagged)
                .map(|p| p.id)
                .collect::<Vec<_>>(),
            published_ids[..10]
        );
        assert!(
            tagged
                .iter()
                .chain(&second_tagged)
                .all(|p| p.body.is_empty())
        );
        assert!(list_posts_by_tag(&pool, "alpha", 3, 8).await?.is_empty());
        assert_eq!(count_posts_by_tag(&pool, "unused").await?, 0);
        assert!(tagged.iter().all(|post| post.tags.len() == 2));
        let public_counts = list_tags_with_published_count(&pool).await?;
        assert_eq!(
            public_counts
                .iter()
                .find(|tag| tag.id == alpha)
                .map(|tag| tag.count),
            Some(10)
        );
        assert_eq!(
            public_counts
                .iter()
                .find(|tag| tag.id == empty)
                .map(|tag| tag.count),
            Some(0)
        );
        let all_counts = list_tags_with_count(&pool).await?;
        assert_eq!(
            all_counts
                .iter()
                .find(|tag| tag.id == alpha)
                .map(|tag| tag.count),
            Some(12)
        );
        Ok(())
    }

    #[sqlx::test]
    async fn admin_filters_and_sorts_preserve_null_and_page_ordering(pool: PgPool) -> Result<()> {
        let now = Utc::now();
        let tag = create_tag(&pool, "Tag", "tag").await?;
        let mut ids = Vec::new();
        for (slug, published_at, created_at) in [
            (
                "old",
                Some(now - Duration::days(2)),
                now - Duration::hours(1),
            ),
            (
                "new",
                Some(now - Duration::days(1)),
                now - Duration::hours(4),
            ),
            (
                "future",
                Some(now + Duration::days(1)),
                now - Duration::hours(3),
            ),
            ("draft", None, now - Duration::hours(2)),
        ] {
            let id = create_post_with_tags(&pool, slug, slug, "Body", None, published_at, &[tag])
                .await?;
            query!(
                "UPDATE posts SET created_at = $1 WHERE id = $2",
                created_at,
                id
            )
            .execute(&pool)
            .await?;
            ids.push(id);
        }
        let [old, new, future, draft] = ids[..] else {
            return Err(crate::Error::System("missing test posts".into()));
        };
        for (filter, expected) in [
            (PostStatusFilter::All, vec![future, new, old, draft]),
            (PostStatusFilter::Published, vec![new, old]),
            (PostStatusFilter::Scheduled, vec![future]),
            (PostStatusFilter::Unscheduled, vec![draft]),
        ] {
            let posts = list_posts(&pool, &filter, 1, 8, &PostSort::PublishedAtDesc).await?;
            assert_eq!(
                posts.iter().map(|post| post.id).collect::<Vec<_>>(),
                expected
            );
            assert_eq!(
                usize::try_from(count_posts(&pool, &filter).await?).ok(),
                Some(expected.len())
            );
        }
        for (sort, expected) in [
            (PostSort::PublishedAtDesc, vec![future, new, old, draft]),
            (PostSort::PublishedAtAsc, vec![draft, old, new, future]),
            (PostSort::CreatedAtDesc, vec![old, draft, future, new]),
            (PostSort::CreatedAtAsc, vec![new, future, draft, old]),
        ] {
            let first = list_posts(&pool, &PostStatusFilter::All, 1, 2, &sort).await?;
            let second = list_posts(&pool, &PostStatusFilter::All, 2, 2, &sort).await?;
            assert_eq!(
                first
                    .iter()
                    .chain(&second)
                    .map(|post| post.id)
                    .collect::<Vec<_>>(),
                expected
            );
            assert!(first.iter().chain(&second).all(|post| post.tags.len() == 1));
        }
        assert!(get_post_by_id(&pool, draft).await?.is_some());
        Ok(())
    }

    #[sqlx::test]
    async fn related_posts_rank_shared_tags_and_exclude_current_and_hidden(
        pool: PgPool,
    ) -> Result<()> {
        let alpha = create_tag(&pool, "Alpha", "alpha").await?;
        let beta = create_tag(&pool, "Beta", "beta").await?;
        let published = Utc::now() - Duration::days(1);
        let current = create_post_with_tags(
            &pool,
            "Current",
            "current",
            "Body",
            None,
            Some(published),
            &[alpha, beta],
        )
        .await?;
        for number in 0..5 {
            create_post_with_tags(
                &pool,
                "One shared tag",
                &format!("one-{number}"),
                "Body",
                None,
                Some(published),
                &[alpha],
            )
            .await?;
        }
        create_post_with_tags(
            &pool,
            "Two shared tags",
            "two",
            "Body",
            None,
            Some(published - Duration::days(1)),
            &[alpha, beta],
        )
        .await?;
        create_post_with_tags(&pool, "Draft", "draft", "Body", None, None, &[alpha, beta]).await?;
        create_post_with_tags(
            &pool,
            "Future",
            "future",
            "Body",
            None,
            Some(published + Duration::days(2)),
            &[alpha, beta],
        )
        .await?;
        let untagged = create_post_with_tags(
            &pool,
            "Untagged",
            "untagged",
            "Body",
            None,
            Some(published),
            &[],
        )
        .await?;
        let related = related_posts(&pool, current).await?;
        assert_eq!(
            related
                .iter()
                .map(|post| post.slug.as_str())
                .collect::<Vec<_>>(),
            ["two", "one-0", "one-1", "one-2"]
        );
        assert!(related_posts(&pool, untagged).await?.is_empty());
        Ok(())
    }

    #[test]
    fn count_and_offset_conversions_do_not_wrap() -> Result<()> {
        assert_eq!(count_as_u32(i64::from(u32::MAX))?, u32::MAX);
        assert!(count_as_u32(i64::from(u32::MAX) + 1).is_err());
        assert!(count_as_u32(-1).is_err());
        assert_eq!(page_offset(0, 8)?, 0);
        assert_eq!(page_offset(u32::MAX, 8)?, 34_359_738_352);
        assert!(page_offset(u32::MAX, u32::MAX).is_err());
        Ok(())
    }
}
