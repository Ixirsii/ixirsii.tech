use chrono::NaiveDateTime;
use sqlx::MySqlPool;

pub struct Post {
    pub id: u64,
    pub title: String,
    pub slug: String,
    pub body: String,
    pub preview: Option<String>,
    pub published_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

/* ********************************************************************************************** *
 *                                             Create                                             *
 * ********************************************************************************************** */

pub async fn create_post(
    pool: &MySqlPool,
    title: &str,
    slug: &str,
    body: &str,
    preview: Option<&str>,
) -> Result<Post, sqlx::Error> {
    let result = sqlx::query!(
        "INSERT INTO posts (title, slug, body, preview) VALUES (?, ?, ?, ?)",
        title,
        slug,
        body,
        preview
    )
    .execute(pool)
    .await?;

    get_post_by_id(pool, result.last_insert_id()).await
}

/* ********************************************************************************************** *
 *                                             Delete                                             *
 * ********************************************************************************************** */

/* ********************************************************************************************** *
 *                                              Read                                              *
 * ********************************************************************************************** */

pub async fn get_post_by_id(pool: &MySqlPool, id: u64) -> Result<Post, sqlx::Error> {
    sqlx::query_as!(
        Post,
        "SELECT id, title, slug, body, preview, published_at, created_at, updated_at
         FROM posts
         WHERE id = ?",
        id
    )
    .fetch_one(pool)
    .await
}

pub async fn get_post_by_slug(pool: &MySqlPool, slug: &str) -> Result<Option<Post>, sqlx::Error> {
    sqlx::query_as!(
        Post,
        "SELECT id, title, slug, body, preview, published_at, created_at, updated_at
         FROM posts
         WHERE slug = ?",
        slug
    )
    .fetch_optional(pool)
    .await
}

pub async fn list_posts(pool: &MySqlPool, limit: u32) -> Result<Vec<Post>, sqlx::Error> {
    sqlx::query_as!(
        Post,
        "SELECT id, title, slug, body, preview, published_at, created_at, updated_at
         FROM posts
         WHERE published_at IS NOT NULL AND published_at <= NOW()
         ORDER BY published_at DESC
         LIMIT ?",
        limit
    )
    .fetch_all(pool)
    .await
}

/* ********************************************************************************************** *
 *                                             Update                                             *
 * ********************************************************************************************** */
