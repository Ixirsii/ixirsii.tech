//! Persistent admin session management.

use std::time::Duration as StdDuration;

use async_trait::async_trait;
use axum::{
    extract::Request,
    http::{HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use sqlx::{MySqlPool, mysql::MySqlQueryResult};
use time::{Duration, OffsetDateTime};
use tokio::task::JoinHandle;
use tower_sessions::{
    Expiry, Session,
    session::{self, Id, Record},
    session_store::{self, ExpiredDeletion, SessionStore},
};
use tracing::{error, warn};

use crate::{ADMIN_ROUTE, LOGIN_ROUTE};

const ADMIN_LOGIN_AT_KEY: &str = "admin.login_at";
const ADMIN_IDLE_LIFETIME: Duration = Duration::minutes(30);
const ADMIN_MAX_LIFETIME: Duration = Duration::hours(12);
const CLEANUP_INTERVAL: StdDuration = StdDuration::from_mins(5);

#[derive(Clone, Debug)]
pub struct MySqlSessionStore {
    pool: MySqlPool,
}

impl MySqlSessionStore {
    pub const fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SessionStore for MySqlSessionStore {
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        loop {
            let result =
                sqlx::query("INSERT INTO tower_sessions (id, data, expiry_date) VALUES (?, ?, ?)")
                    .bind(record.id.to_string())
                    .bind(encode_record(record)?)
                    .bind(record.expiry_date)
                    .execute(&self.pool)
                    .await;

            match result {
                Ok(_) => return Ok(()),
                Err(error)
                    if error
                        .as_database_error()
                        .is_some_and(sqlx::error::DatabaseError::is_unique_violation) =>
                {
                    record.id = Id::default();
                }
                Err(error) => return Err(backend_error(error)),
            }
        }
    }

    async fn save(&self, record: &Record) -> session_store::Result<()> {
        let result: MySqlQueryResult =
            sqlx::query("UPDATE tower_sessions SET data = ?, expiry_date = ? WHERE id = ?")
                .bind(encode_record(record)?)
                .bind(record.expiry_date)
                .bind(record.id.to_string())
                .execute(&self.pool)
                .await
                .map_err(backend_error)?;

        if result.rows_affected() == 0 {
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tower_sessions WHERE id = ?)")
                    .bind(record.id.to_string())
                    .fetch_one(&self.pool)
                    .await
                    .map_err(backend_error)?;
            if !exists {
                return Err(session_store::Error::Backend(
                    "session no longer exists".to_string(),
                ));
            }
        }

        Ok(())
    }

    async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
        let data: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT data FROM tower_sessions WHERE id = ? AND expiry_date > UTC_TIMESTAMP(6)",
        )
        .bind(session_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(backend_error)?;

        data.map(|data| {
            rmp_serde::from_slice(&data)
                .map_err(|error| session_store::Error::Decode(error.to_string()))
        })
        .transpose()
    }

    async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
        sqlx::query("DELETE FROM tower_sessions WHERE id = ?")
            .bind(session_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(backend_error)?;
        Ok(())
    }
}

#[async_trait]
impl ExpiredDeletion for MySqlSessionStore {
    async fn delete_expired(&self) -> session_store::Result<()> {
        sqlx::query("DELETE FROM tower_sessions WHERE expiry_date <= UTC_TIMESTAMP(6)")
            .execute(&self.pool)
            .await
            .map_err(backend_error)?;
        Ok(())
    }
}

fn encode_record(record: &Record) -> session_store::Result<Vec<u8>> {
    rmp_serde::to_vec(record).map_err(|error| session_store::Error::Encode(error.to_string()))
}

fn backend_error(error: impl std::fmt::Display) -> session_store::Error {
    session_store::Error::Backend(error.to_string())
}

pub const fn idle_expiry() -> Expiry {
    Expiry::OnInactivity(ADMIN_IDLE_LIFETIME)
}

/// Records the beginning of an authenticated admin session.
///
/// # Errors
/// Returns an error if the session cannot be loaded or updated.
pub async fn mark_admin_login(session: &Session) -> Result<(), session::Error> {
    session
        .insert(
            ADMIN_LOGIN_AT_KEY,
            OffsetDateTime::now_utc().unix_timestamp(),
        )
        .await
}

pub fn spawn_cleanup(store: MySqlSessionStore) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(CLEANUP_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            if let Err(error) = store.delete_expired().await {
                warn!(%error, "Failed to delete expired sessions");
            }
        }
    })
}

pub async fn admin_session_lifetime(session: Session, request: Request, next: Next) -> Response {
    let login_at = match session.get::<i64>(ADMIN_LOGIN_AT_KEY).await {
        Ok(login_at) => login_at,
        Err(error) => {
            error!(%error, "Failed to read admin session lifetime");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if login_at
        .is_some_and(|login_at| session_expiry(OffsetDateTime::now_utc(), login_at).is_none())
    {
        if let Err(error) = session.flush().await {
            error!(%error, "Failed to flush expired admin session");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        return Redirect::to(LOGIN_ROUTE).into_response();
    }

    let response = next.run(request).await;
    let login_at = match session.get::<i64>(ADMIN_LOGIN_AT_KEY).await {
        Ok(login_at) => login_at,
        Err(error) => {
            error!(%error, "Failed to refresh admin session lifetime");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let now = OffsetDateTime::now_utc();
    if let Some(login_at) = login_at {
        let Some(expiry) = session_expiry(now, login_at) else {
            if let Err(error) = session.flush().await {
                error!(%error, "Failed to flush admin session at maximum lifetime");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
            return Redirect::to(LOGIN_ROUTE).into_response();
        };
        session.set_expiry(Some(Expiry::AtDateTime(expiry)));
    } else {
        session.set_expiry(Some(Expiry::AtDateTime(
            now.saturating_add(ADMIN_IDLE_LIFETIME),
        )));
    }

    response
}

pub async fn private_no_store(request: Request, next: Next) -> Response {
    let path = request.uri().path().to_owned();
    let mut response = next.run(request).await;
    set_private_cache_control(&path, &mut response);

    response
}

fn set_private_cache_control(path: &str, response: &mut Response) {
    if is_admin_path(path) {
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store, private"),
        );
    }
}

fn session_expiry(now: OffsetDateTime, login_at: i64) -> Option<OffsetDateTime> {
    let login_at = OffsetDateTime::from_unix_timestamp(login_at).ok()?;
    if login_at > now {
        return None;
    }

    let max_expiry = login_at.saturating_add(ADMIN_MAX_LIFETIME);
    if max_expiry <= now {
        return None;
    }

    Some(std::cmp::min(
        now.saturating_add(ADMIN_IDLE_LIFETIME),
        max_expiry,
    ))
}

fn is_admin_path(path: &str) -> bool {
    path == LOGIN_ROUTE || path == ADMIN_ROUTE || path.starts_with("/admin/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_expiry_is_capped_by_max_lifetime() {
        let login_at = OffsetDateTime::UNIX_EPOCH;
        let now = login_at + Duration::hours(11) + Duration::minutes(45);

        assert_eq!(
            session_expiry(now, login_at.unix_timestamp()),
            Some(login_at + ADMIN_MAX_LIFETIME)
        );
    }

    #[test]
    fn active_session_uses_idle_lifetime() {
        let login_at = OffsetDateTime::UNIX_EPOCH;
        let now = login_at + Duration::hours(1);

        assert_eq!(
            session_expiry(now, login_at.unix_timestamp()),
            Some(now + ADMIN_IDLE_LIFETIME)
        );
    }

    #[test]
    fn expired_or_invalid_login_time_is_rejected() {
        let now = OffsetDateTime::UNIX_EPOCH + ADMIN_MAX_LIFETIME;

        assert_eq!(session_expiry(now, 0), None);
        assert_eq!(session_expiry(now, now.unix_timestamp() + 1), None);
        assert_eq!(session_expiry(now, i64::MAX), None);
    }

    #[test]
    fn private_paths_are_matched_on_route_boundaries() {
        assert!(is_admin_path("/login"));
        assert!(is_admin_path("/admin"));
        assert!(is_admin_path("/admin/posts"));
        assert!(!is_admin_path("/login/help"));
        assert!(!is_admin_path("/administrator"));
        assert!(!is_admin_path("/static/admin.css"));
    }

    #[test]
    fn private_responses_disable_caching() {
        let mut login_response = Response::default();
        set_private_cache_control("/login", &mut login_response);
        assert_eq!(
            login_response.headers().get(header::CACHE_CONTROL),
            Some(&HeaderValue::from_static("no-store, private"))
        );

        let mut public_response = Response::default();
        set_private_cache_control("/blog", &mut public_response);
        assert!(
            !public_response
                .headers()
                .contains_key(header::CACHE_CONTROL)
        );
    }
}
