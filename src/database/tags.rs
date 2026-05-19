use sqlx::MySqlPool;

pub struct Tag {
    pub id: u64,
    pub name: String,
    pub slug: String,
}

/* ********************************************************************************************** *
 *                                             Create                                             *
 * ********************************************************************************************** */

pub async fn create_tag(pool: &MySqlPool, name: &str, slug: &str) -> Result<Tag, sqlx::Error> {
    let result = sqlx::query!("INSERT INTO tags (name, slug) VALUES (?, ?)", name, slug)
        .execute(pool)
        .await?;

    get_tag_by_id(pool, result.last_insert_id()).await
}

/* ********************************************************************************************** *
 *                                             Delete                                             *
 * ********************************************************************************************** */

/* ********************************************************************************************** *
 *                                              Read                                              *
 * ********************************************************************************************** */

pub async fn get_tag_by_id(pool: &MySqlPool, id: u64) -> Result<Tag, sqlx::Error> {
    sqlx::query_as!(Tag, "SELECT id, name, slug FROM tags WHERE id = ?", id)
        .fetch_one(pool)
        .await
}

pub async fn get_tag_by_slug(pool: &MySqlPool, slug: &str) -> Result<Option<Tag>, sqlx::Error> {
    sqlx::query_as!(Tag, "SELECT id, name, slug FROM tags WHERE slug = ?", slug)
        .fetch_optional(pool)
        .await
}

pub async fn list_tags(pool: &MySqlPool) -> Result<Vec<Tag>, sqlx::Error> {
    sqlx::query_as!(Tag, "SELECT id, name, slug FROM tags ORDER BY name")
        .fetch_all(pool)
        .await
}

/* ********************************************************************************************** *
 *                                             Update                                             *
 * ********************************************************************************************** */
