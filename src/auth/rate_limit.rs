//! PostgreSQL-backed sliding windows for admin login attempts.

use std::time::Duration;

use sqlx::PgPool;
use tracing::{error, warn};

pub(super) struct RateLimiter {
    scope: &'static str,
    limit: i64,
    window_seconds: i64,
    max_identities: i64,
}

impl RateLimiter {
    pub(super) fn new(
        scope: &'static str,
        limit: usize,
        window: Duration,
        max_identities: usize,
    ) -> Self {
        // BIGINT already exceeds any representable history, row count or age.
        Self {
            scope,
            limit: i64::try_from(limit).unwrap_or(i64::MAX),
            window_seconds: i64::try_from(window.as_secs()).unwrap_or(i64::MAX),
            max_identities: i64::try_from(max_identities).unwrap_or(i64::MAX),
        }
    }

    pub(super) async fn allow(&self, pool: &PgPool, key: [u8; 32]) -> crate::Result<bool> {
        let allowed = sqlx::query_scalar!(
            r#"SELECT reserve_login_attempt($1, $2, $3, $4, $5) AS "allowed!""#,
            self.scope,
            &key[..],
            self.limit,
            self.window_seconds,
            self.max_identities,
        )
        .fetch_one(pool)
        .await
        .inspect_err(|_| {
            error!(
                event = "authentication_store_error",
                dimension = self.scope,
                "could not reserve login attempt"
            );
        })?;
        if !allowed {
            warn!(
                event = "authentication_rejected",
                reason = "rate_limited",
                dimension = self.scope,
                "authentication rejected"
            );
        }
        Ok(allowed)
    }

    pub(super) async fn delete_expired(&self, pool: &PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            "WITH expired AS (
                SELECT identity_hash FROM login_rate_limits
                WHERE scope = $1 AND last_attempt_at <= login_window_start(statement_timestamp(), $2)
                FOR UPDATE SKIP LOCKED
             )
             DELETE FROM login_rate_limits r USING expired
             WHERE r.scope = $1 AND r.identity_hash = expired.identity_hash",
            self.scope,
            self.window_seconds,
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn second_pool(pool: &PgPool) -> sqlx::Result<PgPool> {
        pool.options()
            .clone()
            .max_connections(1)
            .connect_with((*pool.connect_options()).clone())
            .await
    }

    #[sqlx::test]
    async fn replicas_share_new_and_existing_reservations(pool: PgPool) -> crate::Result<()> {
        let other = second_pool(&pool).await?;
        let a = RateLimiter::new("client", 3, Duration::MAX, 1);
        let b = RateLimiter::new("client", 3, Duration::MAX, 1);
        let key = [1; 32];

        // Concurrent first use must create one history and charge both attempts.
        let both = tokio::try_join!(a.allow(&pool, key), b.allow(&other, key))?;
        assert_eq!(both, (true, true));
        let (left, right) = tokio::try_join!(a.allow(&pool, key), b.allow(&other, key))?;
        assert_ne!(left, right, "exactly one replica gets the last slot");
        assert!(!b.allow(&other, key).await?);
        other.close().await;
        Ok(())
    }

    #[sqlx::test]
    async fn capacity_is_shared_per_scope_and_expired_keys_are_reclaimed(
        pool: PgPool,
    ) -> crate::Result<()> {
        let other = second_pool(&pool).await?;
        let window = Duration::from_secs(300);
        let a = RateLimiter::new("client", 2, window, 1);
        let b = RateLimiter::new("client", 2, window, 1);
        let accounts = RateLimiter::new("account_client", 1, window, 1);
        let (first, second) = tokio::try_join!(a.allow(&pool, [1; 32]), b.allow(&other, [2; 32]))?;
        assert_ne!(first, second, "only one new client fits");
        let (active, denied) = if first {
            ([1; 32], [2; 32])
        } else {
            ([2; 32], [1; 32])
        };
        assert!(
            b.allow(&other, active).await?,
            "existing clients still have their own budget"
        );
        assert!(
            accounts.allow(&pool, denied).await?,
            "account capacity is independent"
        );

        let age_clients = || {
            sqlx::query!(
                "UPDATE login_rate_limits SET attempts = ARRAY[now() - interval '5 minutes'],
             last_attempt_at = now() - interval '5 minutes' WHERE scope = 'client'"
            )
            .execute(&pool)
        };
        age_clients().await?;
        assert!(
            a.allow(&pool, denied).await?,
            "admission reclaims capacity without waiting for cleanup"
        );
        assert!(!a.allow(&pool, active).await?);
        assert!(!accounts.allow(&other, denied).await?);

        // Idle keys also expire without another login; concurrent sweeps are safe.
        age_clients().await?;
        tokio::try_join!(a.delete_expired(&pool), b.delete_expired(&other))?;
        accounts.delete_expired(&pool).await?;
        assert_eq!(
            sqlx::query_scalar!("SELECT scope FROM login_rate_limits")
                .fetch_all(&pool)
                .await?,
            ["account_client"]
        );
        other.close().await;
        Ok(())
    }

    #[sqlx::test]
    async fn individual_attempts_expire_and_rejections_do_not_extend_the_window(
        pool: PgPool,
    ) -> crate::Result<()> {
        let limiter = RateLimiter::new("account_client", 2, Duration::from_secs(300), 10);
        let key = [3; 32];
        assert!(limiter.allow(&pool, key).await?);
        assert!(limiter.allow(&pool, key).await?);
        let history =
            || sqlx::query_scalar!("SELECT attempts FROM login_rate_limits").fetch_one(&pool);
        let before = history().await?;
        assert!(!limiter.allow(&pool, key).await?);
        assert_eq!(before, history().await?);

        // Only the older reservation expires; the other must still count.
        sqlx::query!("UPDATE login_rate_limits SET attempts[1] = now() - interval '5 minutes'")
            .execute(&pool)
            .await?;
        assert!(limiter.allow(&pool, key).await?);
        assert!(!limiter.allow(&pool, key).await?);
        Ok(())
    }
}
