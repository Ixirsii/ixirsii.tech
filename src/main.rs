//! Main executable.

use std::{env::VarError, future::Future, net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderValue, StatusCode, header},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
    serve,
};
use axum_login::{AuthManagerLayerBuilder, login_required};
use dotenvy::from_path;
use include_dir::{Dir, include_dir};
use sqlx::{PgPool, migrate, postgres::PgPoolOptions};
use tokio::net::TcpListener;
use tower_sessions::{SessionManagerLayer, cookie::SameSite};
use tracing::{error, info, instrument, warn};

pub use types::*;

mod auth;
mod browser;
mod csrf;
mod database;
mod login_csrf;
mod observability;
mod page;
mod session;
mod types;

#[cfg(test)]
mod integration_tests;

pub mod static_assets;

/* ********************************************************************************************** *
 *                                           Constants                                            *
 * ********************************************************************************************** */

pub const ADMIN_ROUTE: &str = "/admin";
pub const LOGIN_ROUTE: &str = "/login";
const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(2);
static STATIC_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/src/static");

/* ********************************************************************************************** *
 *                                              Main                                              *
 * ********************************************************************************************** */

#[instrument]
#[tokio::main]
async fn main() -> Result<()> {
    load_local_dotenv(std::path::Path::new(".env"))?;

    let port = configured_port()?;
    let allowed_origins = configured_allowed_origins()?;
    let auth_security = Arc::new(auth::AuthSecurity::new(auth::AuthConfig::from_env()?));
    let db_pool: PgPool = get_db_pool().await?;
    migrate!().run(&db_pool).await?;
    let login_csrf_key = login_csrf::load_key(&db_pool).await?;
    let session_store = session::PostgresSessionStore::new(db_pool.clone());
    let cleanup_task = session::spawn_cleanup(session_store.clone());

    let addr = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(&addr).await?;
    info!("Listening on {addr}");

    let serve_result = serve(
        listener,
        get_routes(
            db_pool,
            auth_security,
            session_store,
            login_csrf_key,
            allowed_origins,
        )
        .into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await;
    cleanup_task.abort();
    serve_result?;

    Ok(())
}

/* ********************************************************************************************** *
 *                                       Private functions                                        *
 * ********************************************************************************************** */

fn load_local_dotenv(path: &std::path::Path) -> Result<()> {
    match from_path(path) {
        Ok(()) => Ok(()),
        Err(error) if error.not_found() => Ok(()),
        Err(_) => Err(Error::Configuration(
            "could not load local .env file".to_owned(),
        )),
    }
}

async fn get_db_pool() -> Result<PgPool> {
    let max_connections = configured_max_connections()?;
    let database_url = configured_database_url()?;

    match PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(&database_url)
        .await
    {
        Ok(db_pool) => Ok(db_pool),
        Err(err) => {
            error!(error = %err, "Error connecting to database");

            Err(Error::Database(err))
        }
    }
}

fn configured_database_url() -> Result<String> {
    let value = optional_config("DATABASE_URL")?
        .ok_or_else(|| Error::Configuration("DATABASE_URL is required".to_owned()))?;
    validate_database_url(&value)?;
    Ok(value)
}

fn validate_database_url(database_url: &str) -> Result<()> {
    if database_url.trim().is_empty()
        || !(database_url.starts_with("postgres://") || database_url.starts_with("postgresql://"))
    {
        return Err(Error::Configuration(
            "DATABASE_URL must be a nonempty PostgreSQL URL".to_owned(),
        ));
    }
    Ok(())
}

fn optional_config(name: &str) -> Result<Option<String>> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => Err(Error::Configuration(format!(
            "{name} must contain valid Unicode"
        ))),
    }
}

fn positive_u16(name: &str, value: Option<&str>, default: u16) -> Result<u16> {
    value.map_or_else(
        || Ok(default),
        |value| {
            value
                .parse::<u16>()
                .ok()
                .filter(|port| *port > 0)
                .ok_or_else(|| {
                    Error::Configuration(format!("{name} must be an integer from 1 to 65535"))
                })
        },
    )
}

fn configured_port() -> Result<u16> {
    positive_u16("PORT", optional_config("PORT")?.as_deref(), 3000)
}

fn configured_max_connections() -> Result<u32> {
    optional_config("DATABASE_MAX_CONNECTIONS")?.map_or_else(
        || Ok(5),
        |value| {
            value
                .parse::<u32>()
                .ok()
                .filter(|count| *count > 0)
                .ok_or_else(|| {
                    Error::Configuration(
                        "DATABASE_MAX_CONNECTIONS must be a positive integer".to_owned(),
                    )
                })
        },
    )
}

fn configured_allowed_origins() -> Result<Vec<String>> {
    if let Some(value) = optional_config("CSRF_ALLOWED_ORIGINS")? {
        let origins: Vec<_> = value.split(',').map(str::trim).collect();
        if origins.is_empty()
            || origins.iter().any(|origin| {
                let Ok(uri) = origin.parse::<axum::http::Uri>() else {
                    return true;
                };
                !matches!(uri.scheme_str(), Some("http" | "https"))
                    || uri.authority().is_none()
                    || uri.path() != "/"
                    || uri.query().is_some()
            })
        {
            return Err(Error::Configuration(
                "CSRF_ALLOWED_ORIGINS must contain comma-separated HTTP(S) origins".to_owned(),
            ));
        }
        Ok(origins.into_iter().map(ToOwned::to_owned).collect())
    } else {
        warn!("CSRF_ALLOWED_ORIGINS not set; origin check disabled");
        Ok(Vec::new())
    }
}

fn get_routes(
    db_pool: PgPool,
    auth_security: Arc<auth::AuthSecurity>,
    session_store: session::PostgresSessionStore,
    login_csrf_key: axum_extra::extract::cookie::Key,
    allowed_origins: Vec<String>,
) -> Router {
    let admin_routes = Router::new()
        .route("/", get(page::admin_dashboard))
        .route(
            "/posts",
            get(page::admin_posts_index).post(page::create_post),
        )
        .route("/posts/new", get(page::new_post_page))
        .route("/posts/{id}/edit", get(page::edit_post_page))
        .route("/posts/{id}", post(page::update_post))
        .route("/posts/{id}/delete", post(page::delete_post))
        .route("/posts/preview/preview", post(page::preview_post_preview))
        .route("/posts/preview/body", post(page::preview_post_body))
        .route("/tags", get(page::tags_index).post(page::create_tag))
        .route("/tags/{id}", post(page::edit_tag))
        .route("/tags/{id}/delete", post(page::delete_tag))
        .route("/logout", post(page::logout))
        .route_layer(login_required!(AppState, login_url = LOGIN_ROUTE))
        .layer(middleware::from_fn(session::admin_session_lifetime));
    let app_state = AppState {
        login_csrf_key,
        auth_security,
        db_pool: Arc::new(db_pool),
        allowed_origins,
    };
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(!cfg!(debug_assertions))
        .with_same_site(SameSite::Lax)
        .with_expiry(session::idle_expiry());
    let auth_layer = AuthManagerLayerBuilder::new(app_state.clone(), session_layer).build();

    let admin_routes = admin_routes.layer(middleware::from_fn_with_state(
        app_state.clone(),
        csrf::csrf_origin_check,
    ));

    let login_routes = Router::new()
        .route(LOGIN_ROUTE, get(page::login_page).post(page::login_submit))
        .layer(middleware::from_fn(session::admin_session_lifetime));
    let app_routes = Router::new()
        .nest(ADMIN_ROUTE, admin_routes)
        .merge(login_routes)
        .route("/", get(page::home))
        .route("/about", get(page::about_page))
        .route("/blog", get(page::blog_index))
        .route("/feed.xml", get(page::rss_feed))
        .route("/blog/{slug}", get(page::blog_post))
        .route("/code", get(page::code_index))
        .route("/code/dotfiles", get(page::code_dotfiles))
        .route("/code/ixirsii-tech", get(page::code_ixirsii_tech))
        .route("/code/randstring", get(page::code_randstring))
        .route("/hall-of-fame", get(page::hall_of_fame_page))
        .route("/health", get(health_check))
        .route("/tags", get(page::tag_index))
        .route("/tags/search", get(page::tag_search))
        .route("/tags/{slug}", get(page::tag_posts))
        .fallback(page::not_found)
        .with_state(app_state)
        .layer(auth_layer)
        .layer(middleware::from_fn(session::private_no_store));

    Router::new()
        .route("/site.webmanifest", get(browser::manifest))
        .route("/.well-known/{*path}", get(well_known_asset))
        .route("/static/{*path}", get(static_asset))
        .merge(app_routes)
        .layer(middleware::from_fn(browser::security_headers))
        .layer(middleware::from_fn(observability::trace_request))
}

async fn health_check(State(state): State<AppState>) -> StatusCode {
    health_check_status(
        async move {
            let mut connection = state.db_pool.acquire().await?;
            sqlx::query_scalar!("SELECT 1")
                .fetch_one(&mut *connection)
                .await?;
            Ok::<(), sqlx::Error>(())
        },
        HEALTH_CHECK_TIMEOUT,
    )
    .await
}

async fn health_check_status<F, E>(check: F, timeout: Duration) -> StatusCode
where
    F: Future<Output = core::result::Result<(), E>>,
{
    match tokio::time::timeout(timeout, check).await {
        Ok(Ok(())) => StatusCode::OK,
        Ok(Err(_)) | Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}

async fn well_known_asset(Path(path): Path<String>) -> Response {
    if path
        .split('/')
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return StatusCode::NOT_FOUND.into_response();
    }

    let source_path = format!(".well-known/{path}");
    let Some(file) = STATIC_DIR.get_file(&source_path) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let mut response = Body::from(file.contents()).into_response();
    let headers = response.headers_mut();
    let mime = mime_guess::from_path(&source_path).first_or_octet_stream();
    let content_type = HeaderValue::from_str(mime.as_ref())
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));

    headers.insert(header::CONTENT_TYPE, content_type);
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=3600"),
    );

    response
}

async fn static_asset(Path(path): Path<String>) -> Response {
    let Some(source_path) = static_assets::source_path_for_request(&path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some(file) = STATIC_DIR.get_file(source_path) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let mut response = Body::from(file.contents()).into_response();
    let headers = response.headers_mut();
    let mime = mime_guess::from_path(source_path).first_or_octet_stream();
    let content_type = HeaderValue::from_str(mime.as_ref())
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));

    headers.insert(header::CONTENT_TYPE, content_type);
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );

    response
}

#[cfg(test)]
mod tests {
    use std::{
        fs, future,
        path::{Path, PathBuf},
        process::Command,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use axum::http::StatusCode;

    use super::{
        configured_allowed_origins, configured_database_url, configured_port, health_check_status,
        load_local_dotenv, positive_u16, validate_database_url,
    };

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> std::io::Result<Self> {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("ixirsii-dotenv-{}-{nonce}", std::process::id()));
            fs::create_dir(&path)?;
            Ok(Self(path))
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn local_dotenv_missing_does_not_search_parent()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let dir = TestDir::new()?;
        fs::write(
            dir.0.join(".env"),
            "DATABASE_URL=postgres://localhost/parent\nPORT=4321\nCSRF_ALLOWED_ORIGINS=http://parent\n",
        )?;
        let child = dir.0.join("child");
        fs::create_dir(&child)?;

        for local in [false, true] {
            if local {
                fs::write(
                    child.join(".env"),
                    "DATABASE_URL=postgres://localhost/local\nPORT=4567\nCSRF_ALLOWED_ORIGINS=http://local\n",
                )?;
            }
            let output = Command::new(std::env::current_exe()?)
                .args(["--exact", "tests::local_dotenv_configuration_child"])
                .current_dir(&child)
                .env(
                    "IXIRSII_DOTENV_CONFIG_FIXTURE",
                    if local { "local" } else { "missing" },
                )
                .env_remove("DATABASE_URL")
                .env_remove("PORT")
                .env_remove("CSRF_ALLOWED_ORIGINS")
                .output()?;
            assert!(
                output.status.success(),
                "isolated dotenv configuration test failed: {}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }
        Ok(())
    }

    #[test]
    fn local_dotenv_configuration_child() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let Ok(mode) = std::env::var("IXIRSII_DOTENV_CONFIG_FIXTURE") else {
            return Ok(());
        };
        load_local_dotenv(Path::new(".env"))?;
        let port = configured_port()?;
        let origins = configured_allowed_origins()?;
        // Exercise the same later read used by get_db_pool, without connecting.
        let database_url = configured_database_url();
        if mode == "local" {
            assert_eq!(port, 4567);
            assert_eq!(origins, ["http://local"]);
            assert_eq!(database_url?, "postgres://localhost/local");
        } else {
            assert_eq!(port, 3000);
            assert_eq!(origins, Vec::<String>::new());
            assert!(database_url.is_err());
            assert!(std::env::var_os("DATABASE_URL").is_none());
        }
        assert_eq!(configured_port()?, port);
        assert_eq!(configured_allowed_origins()?, origins);
        Ok(())
    }

    #[test]
    fn malformed_local_dotenv_returns_safe_error()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let dir = TestDir::new()?;
        let path = dir.0.join(".env");
        fs::write(&path, "IXIRSII_DOTENV_BAD='unterminated\n")?;

        let error = load_local_dotenv(&path)
            .err()
            .ok_or("malformed file was accepted")?;
        assert_eq!(
            error.to_string(),
            "configuration error: could not load local .env file"
        );
        Ok(())
    }

    #[test]
    fn local_dotenv_keeps_existing_process_value()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let dir = TestDir::new()?;
        let path = dir.0.join(".env");
        fs::write(&path, "IXIRSII_DOTENV_PRECEDENCE=from_file\n")?;
        let output = Command::new(std::env::current_exe()?)
            .args(["--exact", "tests::local_dotenv_precedence_child"])
            .env("IXIRSII_DOTENV_PRECEDENCE_FIXTURE", path)
            .env("IXIRSII_DOTENV_PRECEDENCE", "from_process")
            .output()?;
        assert!(
            output.status.success(),
            "isolated dotenv precedence test failed"
        );
        Ok(())
    }

    #[test]
    fn local_dotenv_precedence_child() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let Some(path) = std::env::var_os("IXIRSII_DOTENV_PRECEDENCE_FIXTURE") else {
            return Ok(());
        };
        load_local_dotenv(Path::new(&path))?;
        assert_eq!(std::env::var("IXIRSII_DOTENV_PRECEDENCE")?, "from_process");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_local_dotenv_returns_error_when_permissions_are_enforced()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::PermissionsExt;

        let dir = TestDir::new()?;
        let path = dir.0.join(".env");
        fs::write(&path, "IXIRSII_DOTENV_UNREADABLE=unexpected\n")?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000))?;
        let is_unreadable = fs::File::open(&path).is_err();
        let result = if is_unreadable {
            Some(load_local_dotenv(&path))
        } else {
            None
        };
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        if let Some(result) = result {
            assert!(result.is_err());
        }
        Ok(())
    }

    #[test]
    fn startup_configuration_rejects_invalid_values() {
        assert_eq!(positive_u16("PORT", None, 3000).ok(), Some(3000));
        assert_eq!(positive_u16("PORT", Some("8080"), 3000).ok(), Some(8080));
        for port in ["", "0", "abc", "65536"] {
            assert!(positive_u16("PORT", Some(port), 3000).is_err());
        }
        assert!(validate_database_url("postgres://localhost/db").is_ok());
        assert!(validate_database_url("postgresql://localhost/db").is_ok());
        for url in ["", "mysql://localhost/db", "https://localhost/db"] {
            assert!(validate_database_url(url).is_err());
        }
    }

    #[tokio::test]
    async fn healthy_database_returns_ok() {
        let status =
            health_check_status(future::ready(Ok::<(), ()>(())), Duration::from_secs(1)).await;

        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn unavailable_database_returns_service_unavailable() {
        let status =
            health_check_status(future::ready(Err::<(), ()>(())), Duration::from_secs(1)).await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn timed_out_database_returns_service_unavailable() {
        let status = health_check_status(
            future::pending::<core::result::Result<(), ()>>(),
            Duration::from_millis(1),
        )
        .await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    }
}
