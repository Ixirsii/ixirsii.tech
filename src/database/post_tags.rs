use sqlx::MySqlPool;

use super::{Post, Tag};

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

pub async fn get_posts_by_tag(pool: &MySqlPool, tag_slug: &str) -> Result<Vec<Post>, sqlx::Error> {
    sqlx::query_as!(
        Post,
        "SELECT p.id, p.title, p.slug, p.body, p.preview, p.published_at, p.created_at, p.updated_at
         FROM posts p
         JOIN post_tags pt ON pt.post_id = p.id
         JOIN tags t ON t.id = pt.tag_id
         WHERE t.slug = ?
           AND p.published_at IS NOT NULL AND p.published_at <= NOW()
         ORDER BY p.published_at DESC",
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
