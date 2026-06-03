//! Home route handler.

use askama::Template;
use axum::{extract::State, http::StatusCode, response::Html};

use crate::{
    AppState,
    database::list_posts,
    page::{PostView, build_post_views},
};

#[derive(Template)]
#[template(path = "home.html")]
struct HomePage<'a> {
    title: &'a str,
    posts: &'a [PostView],
}

/// Renders the home page.
///
/// # Errors
/// Returns `500 Internal Server Error` if template rendering fails.
pub async fn home(State(state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let post_rows = list_posts(&state.db_pool, 10)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let post_views: Vec<PostView> = build_post_views(post_rows);

    let page = HomePage {
        title: "Home",
        posts: &post_views,
    };

    Ok(Html(
        page.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}
