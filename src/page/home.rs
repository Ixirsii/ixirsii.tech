//! Home route handler.

use askama::Template;
use axum::{extract::State, http::StatusCode, response::Html};

use crate::{
    AppState,
    database::{get_tags_for_post, list_posts},
    page::PostView,
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
        let tags_html = tags
            .iter()
            .map(|t| format!("<a href=\"/tags/{}\">{}</a>", t.slug, t.name))
            .collect::<Vec<_>>()
            .join(", ");
        let mut view = PostView::from(post);
        view.tags_html = tags_html;
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
