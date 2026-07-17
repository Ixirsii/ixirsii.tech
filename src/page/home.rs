//! Home route handler.

use askama::Template;
use axum::{extract::State, response::Html};

use tracing::error;

use crate::{
    AdminAuthSession, AppState,
    database::list_published_posts,
    page::{PAGE_SIZE, PageError, PostView},
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
) -> Result<Html<String>, PageError> {
    let show_admin_link = auth_session.user.is_some();
    let posts: Vec<PostView> = list_published_posts(&(*state.db_pool), 1, PAGE_SIZE)
        .await
        .map_err(|_| PageError::internal_server_error(show_admin_link))?
        .into_iter()
        .map(PostView::from)
        .collect();
    let page = HomePage {
        title: "Home",
        show_admin_link,
        posts: &posts,
    };

    match page.render() {
        Ok(html) => Ok(Html(html)),
        Err(err) => {
            error!(error = %err, "home: template render failed");
            Err(PageError::internal_server_error(show_admin_link))
        }
    }
}
