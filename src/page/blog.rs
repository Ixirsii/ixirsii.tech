//! Blog route handlers.

use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Html,
};
use std::collections::HashMap;

use crate::{
    AppState,
    database::{get_post_by_slug, get_tags_for_post, list_posts_paginated},
    page::{PostView, TagView},
};

#[derive(Template)]
#[template(path = "blog_index.html")]
struct BlogIndexPage<'a> {
    title: &'a str,
    posts: &'a [PostView],
    current_page: u32,
    total_pages: u32,
}

#[derive(Template)]
#[template(path = "blog_post.html")]
struct BlogPostPage<'a> {
    title: &'a str,
    post: &'a PostView,
}

/// Renders the blog page with paginated post cards.
///
/// # Errors
/// Returns `500 Internal Server Error` if database queries or template rendering fails.
pub async fn blog_index(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Html<String>, StatusCode> {
    const PAGE_SIZE: u32 = 8;
    let page = params
        .get("page")
        .and_then(|p| p.parse::<u32>().ok())
        .filter(|&p| p > 0)
        .unwrap_or(1);

    let (posts, total) = list_posts_paginated(&state.db_pool, page, PAGE_SIZE)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let total_pages = total.div_ceil(PAGE_SIZE);

    let mut post_views: Vec<PostView> = Vec::with_capacity(posts.len());
    for post in posts {
        let tags = get_tags_for_post(&state.db_pool, post.id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let mut view = PostView::from(post);
        view.tags = tags.into_iter().map(TagView::from).collect();
        post_views.push(view);
    }

    let page_data = BlogIndexPage {
        title: "Blog",
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
    Path(slug): Path<String>,
) -> Result<Html<String>, StatusCode> {
    let Some(post) = get_post_by_slug(&state.db_pool, &slug)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    else {
        return Err(StatusCode::NOT_FOUND);
    };

    let tags = get_tags_for_post(&state.db_pool, post.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut view = PostView::from(post);
    view.tags = tags.into_iter().map(TagView::from).collect();

    let page = BlogPostPage {
        title: &view.title,
        post: &view,
    };

    Ok(Html(
        page.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}
