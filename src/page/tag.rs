//! Tag route handlers.

use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Html,
};

use crate::{
    AppState,
    database::{get_posts_by_tag, get_tag_by_slug, list_tags_with_counts},
    page::{PostView, build_post_views},
};

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

struct TagCloudEntry {
    name: String,
    slug: String,
    count: i64,
    class: String,
}

#[allow(clippy::cast_precision_loss)]
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

/// Renders the tag index page (tag cloud).
///
/// # Errors
/// Returns `500 Internal Server Error` if database queries or template rendering fails.
pub async fn tag_index(State(state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let tags: Vec<_> = list_tags_with_counts(&state.db_pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .filter(|tag| tag.count > 0)
        .collect();

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

    let post_rows = get_posts_by_tag(&state.db_pool, &slug)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let post_views: Vec<PostView> = build_post_views(post_rows);

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

/* ********************************************************************************************** *
 *                                       Private functions                                        *
 * ********************************************************************************************** */

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
