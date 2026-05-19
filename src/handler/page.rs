use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Html,
};
use pulldown_cmark::{Parser, html};

use crate::{
    AppState,
    database::{Post, get_post_by_slug, get_tags_for_post, list_posts},
};

#[derive(Template)]
#[template(path = "about.html")]
struct AboutPage<'a> {
    title: &'a str,
}

#[derive(Template)]
#[template(path = "blog.html")]
struct BlogIndexPage<'a> {
    title: &'a str,
}

#[derive(Template)]
#[template(path = "post.html")]
struct BlogPostPage<'a> {
    title: &'a str,
    post: &'a PostView,
}

#[derive(Template)]
#[template(path = "code.html")]
struct CodePage<'a> {
    title: &'a str,
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
    published_at: String,
    updated_at: String,
    tags_html: String,
}

impl From<Post> for PostView {
    fn from(post: Post) -> Self {
        Self {
            slug: post.slug,
            title: post.title,
            body: render_markdown(&post.body),
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

/// Renders the blog page.
///
/// # Errors
/// Returns `500 Internal Server Error` if template rendering fails.
pub async fn blog_index(State(_state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let page = BlogIndexPage { title: "Blog" };

    Ok(Html(
        page.render()
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
