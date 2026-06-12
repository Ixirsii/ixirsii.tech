//! Main executable.

use std::sync::Arc;

use axum::{
    Router, middleware,
    routing::{get, post},
    serve,
};
use axum_login::{AuthManagerLayerBuilder, login_required};
use dotenvy::{dotenv, var};
use sqlx::{MySqlPool, mysql::MySqlPoolOptions};
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use tower_sessions::{MemoryStore, SessionManagerLayer, cookie::SameSite};
use tracing::{error, info, instrument, warn};

pub use types::*;

mod csrf;
mod database;
mod observability;
mod page;
mod types;

/* ********************************************************************************************** *
 *                                           Constants                                            *
 * ********************************************************************************************** */

pub const ADMIN_ROUTE: &str = "/admin";
pub const LOGIN_ROUTE: &str = "/login";

/* ********************************************************************************************** *
 *                                              Main                                              *
 * ********************************************************************************************** */

#[instrument]
#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    observability::init_tracing();

    let db_pool: MySqlPool = get_db_pool().await?;
    sqlx::migrate!().run(&db_pool).await?;

    let port = var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(&addr).await?;
    info!("Listening on {addr}");

    serve(listener, get_routes(db_pool)).await?;

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
            let error = err.to_string();

            error!(error = error, "Error connecting to database");

            Err(Error::DatabaseConnectionError(error))
        }
    }
}

fn get_routes(db_pool: MySqlPool) -> Router {
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
        db_pool: Arc::new(db_pool),
        allowed_origins,
    };
    let session_layer = SessionManagerLayer::new(MemoryStore::default())
        .with_secure(!cfg!(debug_assertions))
        .with_same_site(SameSite::Lax);
    let auth_layer = AuthManagerLayerBuilder::new(app_state.clone(), session_layer).build();
    let static_service = ServeDir::new("./src/static");

    let admin_routes = admin_routes.layer(middleware::from_fn_with_state(
        app_state.clone(),
        csrf::csrf_origin_check,
    ));

    Router::new()
        .nest(ADMIN_ROUTE, admin_routes)
        .route("/", get(page::home))
        .route("/about", get(page::about_page))
        .route("/blog", get(page::blog_index))
        .route("/blog/{slug}", get(page::blog_post))
        .route("/code", get(page::code_index))
        .route("/code/{slug}", get(page::code_project))
        .route(LOGIN_ROUTE, get(page::login_page).post(page::login_submit))
        .route("/tags", get(page::tag_index))
        .route("/tags/search", get(page::tag_search))
        .route("/tags/{slug}", get(page::tag_posts))
        .nest_service("/static", static_service)
        .with_state(app_state)
        .layer(middleware::from_fn(observability::trace_request))
        .layer(auth_layer)
}
