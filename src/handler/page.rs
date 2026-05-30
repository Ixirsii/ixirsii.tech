use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Html,
};
use pulldown_cmark::{Parser, html};
use std::collections::HashMap;

use crate::{
    AppState,
    database::{
        Post, get_post_by_slug, get_posts_by_tag, get_tag_by_slug,
        get_tags_for_post, list_posts, list_posts_paginated, list_tags_with_counts,
    },
};

#[derive(Template)]
#[template(path = "about_page.html")]
struct AboutPage<'a> {
    title: &'a str,
}

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

#[derive(Template)]
#[template(path = "code_page.html")]
struct CodePage<'a> {
    title: &'a str,
}

#[derive(Template)]
#[template(path = "tags_index.html")]
struct TagsPage<'a> {
    title: &'a str,
    tags: &'a [TagCloudEntry],
}

#[derive(Template)]
#[template(path = "tag_posts.html")]
struct TagPostsPage<'a> {
    title: String,
    tag_name: String,
    posts: &'a [PostView],
}

#[derive(Template)]
#[template(path = "home.html")]
struct HomePage<'a> {
    title: &'a str,
    posts: &'a [PostView],
}

struct PostView {
    slug: String,
    title: String,
    body: String,
    preview: String,
    published_at: String,
    updated_at: String,
    tags_html: String,
}

struct TagCloudEntry {
    name: String,
    slug: String,
    count: i64,
    class: String,
}

impl From<Post> for PostView {
    fn from(post: Post) -> Self {
        Self {
            slug: post.slug,
            title: post.title,
            body: render_markdown(&post.body),
            preview: post
                .preview
                .map(|p| render_markdown(&p))
                .unwrap_or_default(),
            published_at: post
                .published_at
                .map(|d| d.format("%B %e, %Y").to_string())
                .unwrap_or_default(),
            updated_at: post
                .updated_at
                .map(|d| d.format("%B %e, %Y").to_string())
                .unwrap_or_default(),
            tags_html: String::new(),
        }
    }
}

/// Renders the about page.
///
/// # Errors
/// Returns `500 Internal Server Error` if template rendering fails.
pub async fn about_page(State(_state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let page = AboutPage { title: "About" };

    Ok(Html(
        page.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
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
        let tags_html = tags
            .iter()
            .map(|t| format!("<a href=\"/tags/{}\">{}</a>", t.slug, t.name))
            .collect::<Vec<_>>()
            .join(", ");
        let mut view = PostView::from(post);
        view.tags_html = tags_html;
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
    let tags_html = tags
        .iter()
        .map(|t| format!("<a href=\"/tags/{}\">{}</a>", t.slug, t.name))
        .collect::<Vec<_>>()
        .join(", ");
    let mut view = PostView::from(post);
    view.tags_html = tags_html;

    let page = BlogPostPage {
        title: &view.title,
        post: &view,
    };

    Ok(Html(
        page.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

/// Renders the code page.
///
/// # Errors
/// Returns `500 Internal Server Error` if template rendering fails.
pub async fn code_page(State(_state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let page = CodePage { title: "Code" };

    Ok(Html(
        page.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

/// Renders the tag index page (tag cloud).
///
/// # Errors
/// Returns `500 Internal Server Error` if database queries or template rendering fails.
pub async fn tags_index(
    State(state): State<AppState>,
) -> Result<Html<String>, StatusCode> {
    let tags = list_tags_with_counts(&state.db_pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let max_count = tags.iter().map(|t| t.count).max().unwrap_or(0);

    let entries: Vec<TagCloudEntry> = tags
        .into_iter()
        .enumerate()
        .map(|(i, tag)| {
            let size = tag_cloud_size(tag.count, max_count);
            let color = TAG_CLOUD_COLORS[i % TAG_CLOUD_COLORS.len()];
            let class = format!("{size} {color}");
            TagCloudEntry {
                name: tag.name,
                slug: tag.slug,
                count: tag.count,
                class,
            }
        })
        .collect();

    let page = TagsPage {
        title: "Tags",
        tags: &entries,
    };

    Ok(Html(
        page.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

/// Renders posts filtered by a tag.
///
/// # Errors
/// Returns `404 Not Found` if the tag slug doesn't match, or
/// `500 Internal Server Error` if database queries or template rendering fails.
pub async fn tag_posts(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Html<String>, StatusCode> {
    let Some(tag) = get_tag_by_slug(&state.db_pool, &slug)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    else {
        return Err(StatusCode::NOT_FOUND);
    };

    let posts = get_posts_by_tag(&state.db_pool, &slug)
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

    let title = format!("Tag: {}", tag.name);

    let page = TagPostsPage {
        title,
        tag_name: tag.name,
        posts: &post_views,
    };

    Ok(Html(
        page.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
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

fn render_markdown(text: &str) -> String {
    let parser = Parser::new(text);
    let mut output = String::new();
    html::push_html(&mut output, parser);
    output
}

const TAG_CLOUD_COLORS: [&str; 12] = [
    "text-ctp-pink",
    "text-ctp-mauve",
    "text-ctp-red",
    "text-ctp-maroon",
    "text-ctp-peach",
    "text-ctp-yellow",
    "text-ctp-green",
    "text-ctp-teal",
    "text-ctp-sky",
    "text-ctp-sapphire",
    "text-ctp-blue",
    "text-ctp-lavender",
];

#[allow(clippy::cast_precision_loss)]
fn tag_cloud_size(count: i64, max_count: i64) -> &'static str {
    if max_count == 0 {
        return "text-sm";
    }
    let ratio = count as f64 / max_count as f64;
    if ratio >= 0.9 {
        "text-4xl"
    } else if ratio >= 0.7 {
        "text-2xl"
    } else if ratio >= 0.5 {
        "text-xl"
    } else if ratio >= 0.2 {
        "text-base"
    } else {
        "text-sm"
    }
}
