//! Blog route handlers.

use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Html,
};
use std::collections::HashMap;

use crate::{
    AdminAuthSession, AppState,
    database::{count_published_posts, get_post_by_slug, list_published_posts},
    page::{PAGE_SIZE, PostView},
};

#[derive(Template)]
#[template(path = "blog_index.html")]
struct BlogIndexPage<'a> {
    title: &'a str,
    show_admin_link: bool,
    posts: &'a [PostView],
    current_page: u32,
    total_pages: u32,
}

#[derive(Template)]
#[template(path = "blog_post.html")]
struct BlogPostPage<'a> {
    title: &'a str,
    show_admin_link: bool,
    post: &'a PostView,
}

/// Renders the blog page with paginated post cards.
///
/// # Errors
/// Returns `500 Internal Server Error` if database queries or template rendering fails.
pub async fn blog_index(
    State(state): State<AppState>,
    auth_session: AdminAuthSession,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Html<String>, StatusCode> {
    let page = params
        .get("page")
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(1);
    let post_rows = list_published_posts(&(*state.db_pool), page, PAGE_SIZE)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let post_views: Vec<PostView> = post_rows.into_iter().map(PostView::from).collect();
    let total = count_published_posts(&(*state.db_pool))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let total_pages = total.div_ceil(PAGE_SIZE);
    let page_data = BlogIndexPage {
        title: "Blog | Ixirsii",
        show_admin_link: auth_session.user.is_some(),
        posts: &post_views,
        current_page: page,
        total_pages,
    };

    Ok(Html(
        page_data
            .render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

/// Renders an individual blog post.
///
/// # Errors
/// Returns `404 Not Found` if the slug doesn't match any post, or
/// `500 Internal Server Error` if template rendering fails.
pub async fn blog_post(
    State(state): State<AppState>,
    auth_session: AdminAuthSession,
    Path(slug): Path<String>,
) -> Result<Html<String>, StatusCode> {
    let Some(post) = get_post_by_slug(&(*state.db_pool), &slug)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    else {
        return Err(StatusCode::NOT_FOUND);
    };

    let page = BlogPostPage {
        title: &format!("{0} | Ixirsii", post.title),
        show_admin_link: auth_session.user.is_some(),
        post: &PostView::from(post),
    };

    Ok(Html(
        page.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}
