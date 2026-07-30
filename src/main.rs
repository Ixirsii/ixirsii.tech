//! Main executable.

use std::{net::SocketAddr, sync::Arc};

use axum::{
    Router,
    body::Body,
    extract::Path,
    http::{HeaderValue, StatusCode, header},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
    serve,
};
use axum_login::{AuthManagerLayerBuilder, login_required};
use dotenvy::{dotenv, var};
use include_dir::{Dir, include_dir};
use sqlx::{MySqlPool, mysql::MySqlPoolOptions};
use tokio::net::TcpListener;
use tower_sessions::{MemoryStore, SessionManagerLayer, cookie::SameSite};
use tracing::{error, info, instrument, warn};

pub use types::*;

mod auth;
mod csrf;
mod database;
mod observability;
mod page;
mod types;

pub mod static_assets {
    include!(concat!(env!("OUT_DIR"), "/static_assets.rs"));
}

/* ********************************************************************************************** *
 *                                           Constants                                            *
 * ********************************************************************************************** */

pub const ADMIN_ROUTE: &str = "/admin";
pub const LOGIN_ROUTE: &str = "/login";
static STATIC_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/src/static");

/* ********************************************************************************************** *
 *                                              Main                                              *
 * ********************************************************************************************** */

#[instrument]
#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    observability::init_tracing();

    let auth_security = Arc::new(auth::AuthSecurity::new(auth::AuthConfig::from_env()?));
    let db_pool: MySqlPool = get_db_pool().await?;
    sqlx::migrate!().run(&db_pool).await?;

    let port = var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(&addr).await?;
    info!("Listening on {addr}");

    serve(
        listener,
        get_routes(db_pool, auth_security).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

/* ********************************************************************************************** *
 *                                       Private functions                                        *
 * ********************************************************************************************** */

async fn get_db_pool() -> Result<MySqlPool> {
    let max_connections: u32 = var("DATABASE_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);
    let database_url: String = var("DATABASE_URL")?;

    match MySqlPoolOptions::new()
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

fn get_routes(db_pool: MySqlPool, auth_security: Arc<auth::AuthSecurity>) -> Router {
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
        .route_layer(login_required!(AppState, login_url = LOGIN_ROUTE));
    let allowed_origins: Vec<String> = match var("CSRF_ALLOWED_ORIGINS") {
        Ok(val) if !val.is_empty() => val
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => {
            warn!("CSRF_ALLOWED_ORIGINS not set; origin check disabled");
            Vec::new()
        }
    };
    let app_state = AppState {
        auth_security,
        db_pool: Arc::new(db_pool),
        allowed_origins,
    };
    let session_layer = SessionManagerLayer::new(MemoryStore::default())
        .with_secure(!cfg!(debug_assertions))
        .with_same_site(SameSite::Lax);
    let auth_layer = AuthManagerLayerBuilder::new(app_state.clone(), session_layer).build();

    let admin_routes = admin_routes.layer(middleware::from_fn_with_state(
        app_state.clone(),
        csrf::csrf_origin_check,
    ));

    let app_routes = Router::new()
        .nest(ADMIN_ROUTE, admin_routes)
        .route("/", get(page::home))
        .route("/about", get(page::about_page))
        .route("/blog", get(page::blog_index))
        .route("/blog/{slug}", get(page::blog_post))
        .route("/code", get(page::code_index))
        .route("/code/dotfiles", get(page::code_dotfiles))
        .route("/code/ixirsii-tech", get(page::code_ixirsii_tech))
        .route("/code/randstring", get(page::code_randstring))
        .route("/hall-of-fame", get(page::hall_of_fame_page))
        .route("/health", get(health_check))
        .route(LOGIN_ROUTE, get(page::login_page).post(page::login_submit))
        .route("/tags", get(page::tag_index))
        .route("/tags/search", get(page::tag_search))
        .route("/tags/{slug}", get(page::tag_posts))
        .fallback(page::not_found)
        .with_state(app_state)
        .layer(auth_layer);

    Router::new()
        .route("/.well-known/{*path}", get(well_known_asset))
        .route("/static/{*path}", get(static_asset))
        .merge(app_routes)
        .layer(middleware::from_fn(observability::trace_request))
}

async fn health_check() -> StatusCode {
    StatusCode::OK
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
