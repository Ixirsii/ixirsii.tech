//! Database operations.

mod post_tags;
mod posts;
mod tags;
mod transactions;
mod types;
mod users;

pub use post_tags::*;
pub use posts::*;
pub use tags::*;
pub use transactions::*;
pub use types::*;
pub use users::*;

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    #[sqlx::test]
    async fn migrations_create_an_empty_production_schema(pool: PgPool) -> sqlx::Result<()> {
        let counts = sqlx::query!(
            r#"SELECT
                (SELECT COUNT(*) FROM posts) AS "posts!",
                (SELECT COUNT(*) FROM tags) AS "tags!",
                (SELECT COUNT(*) FROM post_tags) AS "post_tags!",
                (SELECT COUNT(*) FROM admin_users) AS "admin_users!",
                (SELECT COUNT(*) FROM tower_sessions) AS "sessions!""#,
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(counts.posts, 0);
        assert_eq!(counts.tags, 0);
        assert_eq!(counts.post_tags, 0);
        assert_eq!(counts.admin_users, 0);
        assert_eq!(counts.sessions, 0);
        Ok(())
    }
}
