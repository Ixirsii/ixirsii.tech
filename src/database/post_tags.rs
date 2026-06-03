use sqlx::MySqlPool;

use super::{PostWithTagRow, Tag};

/* ********************************************************************************************** *
 *                                             Create                                             *
 * ********************************************************************************************** */

pub async fn add_tag_to_post(
    pool: &MySqlPool,
    post_id: u64,
    tag_id: u64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO post_tags (post_id, tag_id) VALUES (?, ?)",
        post_id,
        tag_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/* ********************************************************************************************** *
 *                                             Delete                                             *
 * ********************************************************************************************** */

/* ********************************************************************************************** *
 *                                              Read                                              *
 * ********************************************************************************************** */

pub async fn get_posts_by_tag(
    pool: &MySqlPool,
    tag_slug: &str,
) -> Result<Vec<PostWithTagRow>, sqlx::Error> {
    sqlx::query_as!(
        PostWithTagRow,
        "SELECT p.id, p.title, p.slug, p.body, p.preview, p.published_at, p.created_at, p.updated_at,
                t.name AS tag_name, t.slug AS tag_slug
         FROM posts p
         JOIN post_tags pt_filter ON pt_filter.post_id = p.id
         JOIN tags t_filter ON t_filter.id = pt_filter.tag_id
         LEFT JOIN post_tags pt ON pt.post_id = p.id
         LEFT JOIN tags t ON t.id = pt.tag_id
         WHERE t_filter.slug = ?
           AND p.published_at IS NOT NULL AND p.published_at <= NOW()
         ORDER BY p.published_at DESC, p.id, t.name",
        tag_slug
    )
    .fetch_all(pool)
    .await
}

pub async fn get_tags_for_post(pool: &MySqlPool, post_id: u64) -> Result<Vec<Tag>, sqlx::Error> {
    sqlx::query_as!(
        Tag,
        "SELECT t.id, t.name, t.slug
         FROM tags t
         JOIN post_tags pt ON pt.tag_id = t.id
         WHERE pt.post_id = ?
         ORDER BY t.name",
        post_id
    )
    .fetch_all(pool)
    .await
}

/* ********************************************************************************************** *
 *                                             Update                                             *
 * ********************************************************************************************** */
