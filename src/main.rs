//! Main executable.

use std::{future::Future, net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    middleware,
    routing::{get, post},
    serve,
};
use axum_login::{AuthManagerLayerBuilder, login_required};
use nine_dev_site_lib::{startup, telemetry::RequestMetrics};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::net::TcpListener;
use tower_sessions::{SessionManagerLayer, cookie::SameSite};
use tracing::{error, info, warn};

pub use types::*;

mod admin_cli;
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

/* ********************************************************************************************** *
 *                                              Main                                              *
 * ********************************************************************************************** */

fn main() -> Result<()> {
    load_local_dotenv()?;
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match args.as_slice() {
        [] => {}
        [admin, create, username] if admin == "admin" && create == "create" => {
            let username = username
                .to_str()
                .ok_or_else(|| Error::Configuration("username must be valid UTF-8".to_owned()))?;
            return create_admin(username);
        }
        _ => {
            return Err(Error::Configuration(
                "usage: ixirsii-tech [admin create <username>]".to_owned(),
            ));
        }
    }

    let telemetry = observability::init()?;
    let server_result = run(telemetry.requests());
    telemetry.shutdown();
    server_result
}

#[tokio::main(flavor = "current_thread")]
async fn create_admin(username: &str) -> Result<()> {
    admin_cli::create(username).await
}

#[tokio::main]
async fn run(requests: RequestMetrics) -> Result<()> {
    let port = configured_port()?;
    let allowed_origins = configured_allowed_origins()?;
    let auth_security = Arc::new(auth::AuthSecurity::new(auth::AuthConfig::from_env()?));
    let db_pool: PgPool = get_db_pool().await?;

    let server_result = run_http(&db_pool, port, allowed_origins, auth_security, requests).await;
    db_pool.close().await;
    server_result
}

async fn run_http(
    db_pool: &PgPool,
    port: u16,
    allowed_origins: Vec<String>,
    auth_security: Arc<auth::AuthSecurity>,
    requests: RequestMetrics,
) -> Result<()> {
    database::check_schema(db_pool).await?;
    let login_csrf_key = login_csrf::load_key(db_pool).await?;
    let session_store = session::PostgresSessionStore::new(db_pool.clone());

    let address = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(address).await?;
    let shutdown = shutdown_signal()?;
    let cleanup_task = session::spawn_cleanup(session_store.clone());
    info!(target: observability::LIFECYCLE_TARGET, %address, "listening");

    let routes = observability::with_requests(
        get_routes(
            db_pool.clone(),
            auth_security,
            session_store,
            login_csrf_key,
            allowed_origins,
        ),
        requests,
    )
    .into_make_service_with_connect_info::<SocketAddr>();
    let serve_result = serve(listener, routes)
        .with_graceful_shutdown(shutdown)
        .await;
    cleanup_task.abort();
    let _ = cleanup_task.await;
    serve_result.map_err(Error::from)
}

fn shutdown_signal() -> std::io::Result<impl Future<Output = ()>> {
    #[cfg(unix)]
    let (mut interrupt, mut terminate) = {
        use tokio::signal::unix::{SignalKind, signal};

        (
            signal(SignalKind::interrupt())?,
            signal(SignalKind::terminate())?,
        )
    };

    Ok(async move {
        #[cfg(unix)]
        tokio::select! {
            _ = interrupt.recv() => {}
            _ = terminate.recv() => {}
        }
        #[cfg(not(unix))]
        if let Err(error) = tokio::signal::ctrl_c().await {
            error!(target: observability::LIFECYCLE_TARGET, %error, "could not listen for interrupt");
        }

        info!(target: observability::LIFECYCLE_TARGET, "shutting down HTTP server");
    })
}

/* ********************************************************************************************** *
 *                                       Private functions                                        *
 * ********************************************************************************************** */

fn load_local_dotenv() -> Result<()> {
    startup::load_local_dotenv().map_err(|error| Error::Configuration(error.to_string()))
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
            error!(target: observability::LIFECYCLE_TARGET, "error connecting to database");

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
    startup::optional_env(name).map_err(|error| Error::Configuration(error.to_string()))
}

fn configured_port() -> Result<u16> {
    startup::port().map_err(|error| Error::Configuration(error.to_string()))
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
        warn!(
            target: observability::LIFECYCLE_TARGET,
            "CSRF_ALLOWED_ORIGINS not set; origin check disabled"
        );
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
        .route("/licenses", get(page::licenses_page))
        .route("/tags", get(page::tag_index))
        .route("/tags/search", get(page::tag_search))
        .route("/tags/{slug}", get(page::tag_posts))
        .fallback(page::not_found)
        .with_state(app_state)
        .layer(auth_layer)
        .layer(middleware::from_fn(session::private_no_store));

    static_assets::routes()
        .merge(app_routes)
        .layer(middleware::from_fn(browser::security_headers))
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

#[cfg(test)]
mod tests {
    use std::{
        fs, future,
        path::PathBuf,
        process::Command,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use axum::http::StatusCode;

    use super::{
        configured_allowed_origins, configured_database_url, configured_port, health_check_status,
        load_local_dotenv, validate_database_url,
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
        load_local_dotenv()?;
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
        for contents in [
            "IXIRSII_DOTENV_BAD='unterminated\n",
            "IXIRSII_DOTENV_BAD='secret\0value'\n",
        ] {
            fs::write(dir.0.join(".env"), contents)?;
            assert_local_dotenv_rejected(&dir)?;
        }
        Ok(())
    }

    fn assert_local_dotenv_rejected(dir: &TestDir) -> std::io::Result<()> {
        let output = Command::new(std::env::current_exe()?)
            .args(["--exact", "tests::invalid_local_dotenv_child"])
            .current_dir(&dir.0)
            .env("IXIRSII_INVALID_DOTENV_FIXTURE", "1")
            .output()?;
        assert!(
            output.status.success(),
            "isolated invalid dotenv test failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        Ok(())
    }

    #[test]
    fn invalid_local_dotenv_child() -> std::result::Result<(), Box<dyn std::error::Error>> {
        if std::env::var_os("IXIRSII_INVALID_DOTENV_FIXTURE").is_none() {
            return Ok(());
        }
        let error = load_local_dotenv()
            .err()
            .ok_or("invalid file was accepted")?;
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
        fs::write(dir.0.join(".env"), "IXIRSII_DOTENV_PRECEDENCE=from_file\n")?;
        let output = Command::new(std::env::current_exe()?)
            .args(["--exact", "tests::local_dotenv_precedence_child"])
            .current_dir(&dir.0)
            .env("IXIRSII_DOTENV_PRECEDENCE_FIXTURE", "1")
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
        if std::env::var_os("IXIRSII_DOTENV_PRECEDENCE_FIXTURE").is_none() {
            return Ok(());
        }
        load_local_dotenv()?;
        assert_eq!(std::env::var("IXIRSII_DOTENV_PRECEDENCE")?, "from_process");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_local_dotenv_returns_error_when_permissions_are_enforced() -> std::io::Result<()>
    {
        use std::os::unix::fs::PermissionsExt;

        let dir = TestDir::new()?;
        let path = dir.0.join(".env");
        fs::write(&path, "IXIRSII_DOTENV_UNREADABLE=unexpected\n")?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000))?;
        let result = if fs::File::open(&path).is_err() {
            assert_local_dotenv_rejected(&dir)
        } else {
            Ok(())
        };
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        result
    }

    #[test]
    fn startup_configuration_checks_ports() -> std::io::Result<()> {
        let executable = std::env::current_exe()?;
        for port in ["", "0", "abc", "65536"] {
            let output = Command::new(&executable)
                .args(["--exact", "tests::invalid_port_child"])
                .env("IXIRSII_INVALID_PORT_FIXTURE", "1")
                .env("PORT", port)
                .output()?;
            assert!(
                output.status.success(),
                "isolated port configuration test failed: {}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }
        Ok(())
    }

    #[test]
    fn invalid_port_child() {
        if std::env::var_os("IXIRSII_INVALID_PORT_FIXTURE").is_some() {
            assert_eq!(
                configured_port()
                    .err()
                    .map(|error| error.to_string())
                    .as_deref(),
                Some("configuration error: PORT must be an integer from 1 to 65535"),
            );
        }
    }

    #[test]
    fn startup_configuration_checks_database_urls() {
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
