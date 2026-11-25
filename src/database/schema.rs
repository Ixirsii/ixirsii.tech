//! Verify the release's migration history before accepting traffic.

use sqlx::{PgPool, migrate::Migrator};

use crate::{Error, Result};

static MIGRATOR: Migrator = sqlx::migrate!();
const BEHIND: &str =
    "database is not fully migrated; run `cargo sqlx migrate run` for this release";
const MISMATCH: &str = "database migration history does not match this executable";

/// Check the applied migration history without modifying the database.
///
/// # Errors
/// Returns an error if the history cannot be read or does not match this release.
pub async fn check_schema(pool: &PgPool) -> Result<()> {
    let mut connection = pool.acquire().await?;
    check_history(&mut connection).await
}

async fn check_history(connection: &mut sqlx::PgConnection) -> Result<()> {
    let applied =
        sqlx::query!("SELECT version, checksum, success FROM _sqlx_migrations ORDER BY version")
            .fetch_all(connection)
            .await
            .map_err(|error| {
                if error
                    .as_database_error()
                    .and_then(sqlx::error::DatabaseError::code)
                    .as_deref()
                    == Some("42P01")
                {
                    Error::Configuration(BEHIND.into())
                } else {
                    error.into()
                }
            })?;
    let mut expected = MIGRATOR
        .iter()
        .filter(|m| m.migration_type.is_up_migration());
    for row in applied {
        if !row.success
            || !expected
                .next()
                .is_some_and(|m| m.version == row.version && m.checksum.as_ref() == row.checksum)
        {
            return Err(Error::Configuration(MISMATCH.into()));
        }
    }
    if expected.next().is_some() {
        return Err(Error::Configuration(BEHIND.into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = false)]
    async fn empty_database_is_left_empty(pool: PgPool) -> Result<()> {
        assert!(
            matches!(check_schema(&pool).await, Err(Error::Configuration(msg)) if msg == BEHIND)
        );
        assert!(
            sqlx::query_scalar!("SELECT to_regclass('_sqlx_migrations') IS NULL AS \"absent!\"")
                .fetch_one(&pool)
                .await?
        );
        Ok(())
    }

    #[sqlx::test]
    async fn requires_an_exact_successful_history(pool: PgPool) -> Result<()> {
        check_schema(&pool).await?;
        // Use independent transactions so each case starts from the same history.
        for (mutation, message) in [
            (
                "DELETE FROM _sqlx_migrations WHERE version = (SELECT max(version) FROM _sqlx_migrations)",
                BEHIND,
            ),
            (
                "UPDATE _sqlx_migrations SET version = -1 WHERE version = (SELECT min(version) FROM _sqlx_migrations)",
                MISMATCH,
            ),
            (
                "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) VALUES (9223372036854775807, 'new release', true, '', 0)",
                MISMATCH,
            ),
            ("UPDATE _sqlx_migrations SET success = false", MISMATCH),
            ("UPDATE _sqlx_migrations SET checksum = ''", MISMATCH),
        ] {
            let mut tx = pool.begin().await?;
            sqlx::raw_sql(mutation).execute(&mut *tx).await?;
            assert!(
                matches!(check_history(&mut tx).await, Err(Error::Configuration(msg)) if msg == message)
            );
            tx.rollback().await?;
        }
        Ok(())
    }
}
