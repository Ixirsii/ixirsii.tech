//! HTTP regression tests against isolated `PostgreSQL` databases.

use std::{net::SocketAddr, sync::Arc};

use axum::{
    Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{Request, StatusCode, header},
    response::Response,
};
use chrono::{Duration, Utc};
use sqlx::PgPool;
use tower::ServiceExt;

use crate::{
    auth::{AuthConfig, AuthSecurity},
    database, get_routes, login_csrf,
    session::PostgresSessionStore,
    static_assets::static_url,
};

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

async fn app(pool: &PgPool) -> crate::Result<Router> {
    Ok(get_routes(
        pool.clone(),
        Arc::new(AuthSecurity::new(AuthConfig::for_tests())),
        PostgresSessionStore::new(pool.clone()),
        login_csrf::load_key(pool).await?,
        vec!["http://localhost".to_owned()],
    ))
}

fn request(path: &str) -> Result<Request<Body>, axum::http::Error> {
    Request::builder().uri(path).body(Body::empty())
}

async fn html(response: Response) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    Ok(String::from_utf8(
        to_bytes(response.into_body(), 2_000_000).await?.to_vec(),
    )?)
}

fn cookies(response: &Response) -> Result<String, axum::http::header::ToStrError> {
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|value| {
            Ok(value
                .to_str()?
                .split(';')
                .next()
                .unwrap_or_default()
                .to_owned())
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join("; "))
}

fn token(body: &str) -> Result<&str, &'static str> {
    body.split("name=\"csrf_token\" value=\"")
        .nth(1)
        .and_then(|part| part.split('"').next())
        .ok_or("missing CSRF token")
}

fn login_request(
    cookie: &str,
    csrf: &str,
    password: &str,
) -> Result<Request<Body>, Box<dyn std::error::Error + Send + Sync>> {
    let mut request = Request::builder()
        .method("POST")
        .uri("/login")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(header::COOKIE, cookie)
        .body(Body::from(format!(
            "username=developer&password={password}&csrf_token={csrf}"
        )))?;
    request
        .extensions_mut()
        .insert(ConnectInfo("127.0.0.1:4321".parse::<SocketAddr>()?));
    Ok(request)
}

fn assert_security_headers(response: &Response) {
    for (name, expected) in [
        (
            header::CONTENT_SECURITY_POLICY,
            "default-src 'self'; script-src 'self'; style-src 'self'; font-src 'self'; \
             img-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'none'; \
             frame-ancestors 'none'; form-action 'self'",
        ),
        (header::X_FRAME_OPTIONS, "DENY"),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        (header::REFERRER_POLICY, "strict-origin-when-cross-origin"),
    ] {
        assert_eq!(response.headers()[name], expected);
    }
}

async fn check_preview(app: &Router, cookie: &str, token: &str) -> TestResult {
    for (csrf, status, expected) in [
        (token, StatusCode::OK, "<strong>draft</strong>"),
        ("expired", StatusCode::FORBIDDEN, "page is refreshed"),
    ] {
        let request = Request::builder()
            .method("POST")
            .uri("/admin/posts/preview/body")
            .header(header::ORIGIN, "http://localhost")
            .header(header::COOKIE, cookie)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(format!("csrf_token={csrf}&body=**draft**")))?;
        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), status);
        assert!(html(response).await?.contains(expected));
    }
    Ok(())
}

#[sqlx::test]
async fn account_limit_survives_restart_without_locking_out_another_client(
    pool: PgPool,
) -> TestResult {
    let initial = app(&pool).await?;
    sqlx::query!("INSERT INTO admin_users (username, password_hash) VALUES ($1, $2)", "developer", "$argon2id$v=19$m=19456,t=2,p=1$aXhpcnNpaS1kZXYtb25seQ$8k8VcWHiaaBqwzl4afJ3L7QMp6nhuG4FxUX4O5Qu964").execute(&pool).await?;
    let challenge = initial.clone().oneshot(request("/login")?).await?;
    let cookie = cookies(&challenge)?;
    let body = html(challenge).await?;
    let csrf = token(&body)?;
    for _ in 0..4 {
        let response = initial
            .clone()
            .oneshot(login_request(&cookie, csrf, "wrong")?)
            .await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    let last_slot = initial
        .oneshot(login_request(&cookie, csrf, "local-development-only")?)
        .await?;
    assert_eq!(last_slot.status(), StatusCode::SEE_OTHER);
    let restarted = app(&pool).await?;
    let blocked = restarted
        .clone()
        .oneshot(login_request(&cookie, csrf, "local-development-only")?)
        .await?;
    assert_eq!(blocked.status(), StatusCode::UNAUTHORIZED);
    let mut legitimate = login_request(&cookie, csrf, "local-development-only")?;
    legitimate
        .extensions_mut()
        .insert(ConnectInfo("127.0.0.2:4321".parse::<SocketAddr>()?));
    let response = restarted.oneshot(legitimate).await?;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert!(response.headers().contains_key(header::SET_COOKIE));
    Ok(())
}

#[sqlx::test]
async fn anonymous_login_does_not_create_sessions(pool: PgPool) -> TestResult {
    let app = app(&pool).await?;
    for _ in 0..100 {
        let response = app.clone().oneshot(request("/login")?).await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(cookies(&response)?.starts_with(if cfg!(debug_assertions) {
            "login-csrf="
        } else {
            "__Host-login-csrf="
        }));
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "no-store, private"
        );
    }
    let challenge = app.clone().oneshot(request("/login")?).await?;
    let cookie = cookies(&challenge)?;
    let body = html(challenge).await?;
    let csrf = token(&body)?;
    for (cookie, csrf, expected) in [
        ("", csrf, StatusCode::FORBIDDEN),
        (cookie.as_str(), "tampered", StatusCode::FORBIDDEN),
        (cookie.as_str(), csrf, StatusCode::UNAUTHORIZED),
    ] {
        let response = app
            .clone()
            .oneshot(login_request(cookie, csrf, "wrong")?)
            .await?;
        assert_eq!(response.status(), expected);
    }
    assert_eq!(
        sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!" FROM tower_sessions"#)
            .fetch_one(&pool)
            .await?,
        0
    );
    Ok(())
}

#[sqlx::test]
async fn login_session_and_logout_work_across_replicas(pool: PgPool) -> TestResult {
    let first = app(&pool).await?;
    let second = app(&pool).await?;
    let challenge = first.clone().oneshot(request("/login")?).await?;
    let cookie = cookies(&challenge)?;
    let body = html(challenge).await?;
    let csrf = token(&body)?;
    let hash = crate::admin_cli::hash_password("local-development-only")?;
    assert!(database::insert_admin_user(&pool, "developer", &hash).await?);
    let other_hash = crate::admin_cli::hash_password("different-password")?;
    assert!(!database::insert_admin_user(&pool, "developer", &other_hash).await?);
    let response = second
        .clone()
        .oneshot(login_request(&cookie, csrf, "local-development-only")?)
        .await?;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let session_cookie = cookies(&response)?;
    assert!(
        session_cookie
            .split("; ")
            .any(|part| part.starts_with("id="))
    );
    assert_eq!(
        sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!" FROM tower_sessions"#)
            .fetch_one(&pool)
            .await?,
        1
    );
    let admin_request = || {
        Request::builder()
            .uri("/admin")
            .header(header::COOKIE, &session_cookie)
            .body(Body::empty())
    };
    let admin = first.clone().oneshot(admin_request()?).await?;
    assert_eq!(admin.status(), StatusCode::OK);
    assert_security_headers(&admin);
    let body = html(admin).await?;
    assert_ne!(
        token(&body)?,
        csrf,
        "authenticated session gets a fresh CSRF token"
    );
    check_preview(&second, &session_cookie, token(&body)?).await?;
    let response = second
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/logout")
                .header(header::ORIGIN, "http://localhost")
                .header(header::COOKIE, &session_cookie)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(format!("csrf_token={}", token(&body)?)))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let stale_session = first.oneshot(admin_request()?).await?;
    assert_eq!(stale_session.status(), StatusCode::TEMPORARY_REDIRECT);
    assert!(
        stale_session.headers()[header::LOCATION]
            .to_str()?
            .starts_with("/login")
    );
    assert_eq!(
        sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!" FROM tower_sessions"#)
            .fetch_one(&pool)
            .await?,
        0
    );
    Ok(())
}

#[sqlx::test]
async fn limiter_store_failure_stops_login(pool: PgPool) -> TestResult {
    let app = app(&pool).await?;
    let hash = crate::admin_cli::hash_password("local-development-only")?;
    assert!(database::insert_admin_user(&pool, "developer", &hash).await?);
    let challenge = app.clone().oneshot(request("/login")?).await?;
    let cookie = cookies(&challenge)?;
    let body = html(challenge).await?;
    // Only the limiter is unavailable; the password and session stores still work.
    sqlx::raw_sql("ALTER TABLE login_rate_limits RENAME TO unavailable_limits")
        .execute(&pool)
        .await?;
    let response = app
        .oneshot(login_request(
            &cookie,
            token(&body)?,
            "local-development-only",
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!" FROM tower_sessions"#)
            .fetch_one(&pool)
            .await?,
        0
    );
    Ok(())
}

#[sqlx::test]
async fn security_headers_cover_public_login_admin_errors_and_assets(pool: PgPool) -> TestResult {
    let app = app(&pool).await?;
    for (path, expected) in [
        ("/", 200),
        ("/login", 200),
        ("/admin", 307),
        ("/missing", 404),
        (static_url("main.css"), 200),
        ("/static/missing.css", 404),
    ] {
        let response = app.clone().oneshot(request(path)?).await?;
        assert_eq!(response.status().as_u16(), expected, "{path}");
        assert_security_headers(&response);
    }
    // A database failure still passes through the same outer policy.
    pool.close().await;
    let response = app.oneshot(request("/blog")?).await?;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_security_headers(&response);
    Ok(())
}

#[sqlx::test]
async fn shared_browser_assets_are_reachable_from_rendered_pages(pool: PgPool) -> TestResult {
    use nine_dev_site_lib::{assets, switcher};

    let app = app(&pool).await?;
    let page = html(app.clone().oneshot(request("/about")?).await?).await?;
    assert!(
        page.find(switcher::JS.url).ok_or("missing theme script")?
            < page
                .find("rel=\"stylesheet\"")
                .ok_or("missing stylesheet")?
    );
    assert!(
        page.find("name=\"htmx-config\"")
            .ok_or("missing HTMX config")?
            < page.find(assets::htmx::JS.url).ok_or("missing HTMX")?
    );
    assert!(page.contains(switcher::ThemeSwitcher::default().latte));
    for asset in [
        &switcher::JS,
        &switcher::CSS,
        &assets::theme::PALETTE,
        &assets::htmx::JS,
        &assets::icons::DISCORD,
        &assets::icons::BLUESKY,
    ] {
        assert!(page.contains(asset.url));
        let response = app.clone().oneshot(request(asset.url)?).await?;
        assert_security_headers(&response);
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], asset.content_type);
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            nine_dev_site_lib::serving::IMMUTABLE
        );
        assert_eq!(
            to_bytes(response.into_body(), 200_000).await?.as_ref(),
            asset.bytes
        );
    }
    let response = app.oneshot(request("/licenses")?).await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        html(response)
            .await?
            .contains("Copyright (c) 2021 Catppuccin")
    );
    Ok(())
}

async fn revalidated_bytes(
    app: &Router,
    url: &str,
    mime: &str,
) -> Result<axum::body::Bytes, Box<dyn std::error::Error + Send + Sync>> {
    let first = app.clone().oneshot(request(url)?).await?;
    assert_eq!(first.status(), StatusCode::OK);
    assert_security_headers(&first);
    let cache = "public, max-age=0, must-revalidate";
    assert_eq!(first.headers()[header::CACHE_CONTROL], cache);
    assert_eq!(first.headers()[header::CONTENT_TYPE], mime);
    let etag = &first.headers()[header::ETAG];
    let cached = app
        .clone()
        .oneshot(
            Request::get(url)
                .header(header::IF_NONE_MATCH, etag)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(cached.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(&cached.headers()[header::ETAG], etag);
    assert_eq!(cached.headers()[header::CACHE_CONTROL], cache);
    assert!(to_bytes(cached.into_body(), 0).await?.is_empty());
    Ok(to_bytes(first.into_body(), 2_000_000).await?)
}

#[sqlx::test]
async fn manifest_and_install_icons_revalidate_at_stable_urls(pool: PgPool) -> TestResult {
    let app = app(&pool).await?;
    let bytes = revalidated_bytes(&app, "/site.webmanifest", "application/manifest+json").await?;
    assert_eq!(bytes.as_ref(), include_bytes!("../public/site.webmanifest"));
    let manifest: serde_json::Value = serde_json::from_slice(&bytes)?;
    assert_eq!(manifest["short_name"], "Ixirsii");
    let icons = manifest["icons"].as_array().ok_or("no icons")?;
    assert_eq!(icons.len(), 2);
    for size in ["192x192", "512x512"] {
        let source = format!("android-chrome-{size}.png");
        let icon = icons
            .iter()
            .find(|icon| icon["sizes"] == size)
            .ok_or("missing icon size")?;
        let url = icon["src"].as_str().ok_or("no icon URL")?;
        assert_eq!(url, format!("/{source}"));
        let stable = revalidated_bytes(&app, url, "image/png").await?;
        let response = app.clone().oneshot(request(static_url(&source))?).await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            nine_dev_site_lib::serving::IMMUTABLE
        );
        assert_eq!(to_bytes(response.into_body(), 2_000_000).await?, stable);
    }
    let page = html(app.oneshot(request("/about")?).await?).await?;
    assert!(page.contains("rel=\"manifest\" href=\"/site.webmanifest\""));
    Ok(())
}

#[sqlx::test]
async fn public_pages_feed_and_sidebar_exclude_unpublished_posts(pool: PgPool) -> TestResult {
    let tag = database::create_tag(&pool, "Rust", "rust").await?;
    database::create_tag(&pool, "Empty", "empty").await?;
    let published = Utc::now() - Duration::days(1);
    for number in 0..21 {
        database::create_post_with_tags(
            &pool,
            &format!("Published {number}"),
            &format!("published-{number}"),
            "Body",
            Some("Preview"),
            Some(published),
            &[tag],
        )
        .await?;
    }
    for (slug, date) in [
        ("secret-draft", None),
        ("secret-future", Some(Utc::now() + Duration::days(1))),
    ] {
        database::create_post_with_tags(&pool, slug, slug, "Secret", None, date, &[tag]).await?;
    }
    let app = app(&pool).await?;
    for path in [
        "/blog?page=0",
        "/blog?page=4",
        "/blog?page=4294967295",
        "/blog?page=bad",
        "/tags/rust?page=0",
        "/tags/rust?page=4",
        "/tags/empty?page=2",
    ] {
        assert_eq!(
            app.clone().oneshot(request(path)?).await?.status(),
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
    for path in ["/blog?page=3", "/tags/rust?page=3", "/tags/empty"] {
        let response = app.clone().oneshot(request(path)?).await?;
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        let body = html(response).await?;
        assert!(!body.contains("secret-draft"));
        assert!(!body.contains("secret-future"));
    }
    let response = app.clone().oneshot(request("/feed.xml")?).await?;
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "application/rss+xml; charset=utf-8"
    );
    let feed = html(response).await?;
    assert_eq!(feed.matches("<item>").count(), 20);
    assert!(!feed.contains("secret-"));
    assert!(feed.contains("https://ixirsii.tech/blog/published-0"));
    let body = html(app.oneshot(request("/blog/published-0")?).await?).await?;
    assert!(body.contains("Related posts"));
    assert!(body.contains("/blog/published-1"));
    assert!(!body.contains("secret-"));
    Ok(())
}
