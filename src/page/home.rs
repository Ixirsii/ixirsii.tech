//! Home route handler.

use askama::Template;
use axum::{extract::State, http::StatusCode, response::Html};

use tracing::error;

use crate::{
    AdminAuthSession, AppState,
    database::list_published_posts,
    page::{PAGE_SIZE, PostView},
};

#[derive(Template)]
#[template(path = "home.html")]
struct HomePage<'a> {
    title: &'a str,
    show_admin_link: bool,
    posts: &'a [PostView],
}

/// Renders the home page.
///
/// # Errors
/// Returns `500 Internal Server Error` if template rendering fails.
pub async fn home(
    State(state): State<AppState>,
    auth_session: AdminAuthSession,
) -> Result<Html<String>, StatusCode> {
    let posts: Vec<PostView> = match list_published_posts(&(*state.db_pool), 1, PAGE_SIZE).await {
        Ok(posts) => posts.into_iter().map(PostView::from).collect(),
        Err(err) => {
            error!(error = %err, "home: failed to fetch published posts");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    let page = HomePage {
        title: "Home | Ixirsii",
        show_admin_link: auth_session.user.is_some(),
        posts: &posts,
    };

    match page.render() {
        Ok(html) => Ok(Html(html)),
        Err(err) => {
            error!(error = %err, "home: template render failed");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
