//! Home route handler.

use askama::Template;
use axum::{extract::State, http::StatusCode, response::Html};

use crate::{
    AppState,
    database::{get_tags_for_post, list_posts},
    page::{PostView, TagView},
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
    let posts = list_posts(&state.db_pool, 10)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut post_views: Vec<PostView> = Vec::with_capacity(posts.len());
    for post in posts {
        let tags = get_tags_for_post(&state.db_pool, post.id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let mut view = PostView::from(post);
        view.tags = tags.into_iter().map(TagView::from).collect();
        post_views.push(view);
    }

    let page = HomePage {
        title: "Home",
        posts: &post_views,
    };

    Ok(Html(
        page.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}
