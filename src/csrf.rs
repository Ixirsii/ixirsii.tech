use std::fmt::Write;

use argon2::password_hash::rand_core::{OsRng, RngCore};
use axum::{
    extract::{Request, State},
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use tower_sessions::{Session, session};

use crate::AppState;

const CSRF_TOKEN_KEY: &str = "csrf.token";

#[derive(Deserialize)]
pub struct CsrfForm {
    pub csrf_token: String,
}

pub async fn csrf_token(session: &Session) -> Result<String, session::Error> {
    if let Some(token) = session.get(CSRF_TOKEN_KEY).await? {
        return Ok(token);
    }

    let token = generate_token();
    session.insert(CSRF_TOKEN_KEY, token.clone()).await?;

    Ok(token)
}

pub async fn verify_csrf_token(
    session: &Session,
    submitted_token: &str,
) -> Result<bool, session::Error> {
    let Some(expected_token) = session.get::<String>(CSRF_TOKEN_KEY).await? else {
        return Ok(false);
    };

    Ok(!submitted_token.is_empty() && submitted_token == expected_token)
}

pub async fn rotate_csrf_token(session: &Session) -> Result<(), session::Error> {
    let token = generate_token();
    session.insert(CSRF_TOKEN_KEY, token).await?;
    Ok(())
}

/// Middleware that checks the Origin/Referer header of state-changing requests
/// against the configured allowed origins list.
///
/// If `allowed_origins` is empty (not configured), the check is skipped.
pub async fn csrf_origin_check(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let check_methods = [Method::POST, Method::PUT, Method::DELETE, Method::PATCH];
    let is_state_changing = check_methods.contains(request.method());

    if !is_state_changing || state.allowed_origins.is_empty() {
        return next.run(request).await;
    }

    let origin = request
        .headers()
        .get("origin")
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string);

    let allowed = if let Some(ref o) = origin {
        state.allowed_origins.iter().any(|allowed| allowed == o)
    } else {
        request
            .headers()
            .get("referer")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|referer| referer_matches_origin(referer, &state.allowed_origins))
    };

    if allowed {
        next.run(request).await
    } else {
        (StatusCode::FORBIDDEN, "Origin check failed").into_response()
    }
}

fn referer_matches_origin(referer: &str, allowed_origins: &[String]) -> bool {
    let Some(scheme_end) = referer.find("://") else {
        return false;
    };
    let after_scheme = &referer[scheme_end + 3..];
    let host_end = after_scheme.find('/').unwrap_or(after_scheme.len());
    let origin = format!("{}://{}", &referer[..scheme_end], &after_scheme[..host_end]);

    allowed_origins.iter().any(|allowed| allowed == &origin)
}

pub fn generate_token() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let mut token = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        let _ = write!(&mut token, "{byte:02x}");
    }

    token
}
