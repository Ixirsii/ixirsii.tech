//! Tag route handlers.

use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Html,
};
use serde::Deserialize;

use crate::{
    AdminAuthSession, AppState,
    database::{get_tag_by_slug, list_posts_by_tag, list_tags_with_published_count},
    page::PostView,
};

/* ********************************************************************************************** *
 *                                           Constants                                            *
 * ********************************************************************************************** */

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

/* ********************************************************************************************** *
 *                                             Types                                              *
 * ********************************************************************************************** */

struct TagCloudEntry {
    name: String,
    slug: String,
    count: i32,
    class: String,
}

#[derive(Template)]
#[template(path = "tags_index.html")]
struct TagsPage<'a> {
    title: &'a str,
    show_admin_link: bool,
    tags: &'a [TagCloudEntry],
}

#[derive(Template)]
#[template(path = "tags_cloud.html")]
struct TagsCloud<'a> {
    tags: &'a [TagCloudEntry],
}

#[derive(Template)]
#[template(path = "tag_posts.html")]
struct TagPostsPage<'a> {
    title: String,
    show_admin_link: bool,
    tag_name: String,
    posts: &'a [PostView],
}

#[derive(Deserialize)]
pub struct TagSearchParams {
    filter: Option<String>,
}

/// Renders the tag index page (tag cloud).
///
/// # Errors
/// Returns `500 Internal Server Error` if database queries or template rendering fails.
pub async fn tag_index(
    State(state): State<AppState>,
    auth_session: AdminAuthSession,
) -> Result<Html<String>, StatusCode> {
    let entries = get_tag_cloud(&state, None).await?;

    let page = TagsPage {
        title: "Tags | Ixirsii",
        show_admin_link: auth_session.user.is_some(),
        tags: &entries,
    };

    Ok(Html(
        page.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

/// Renders the tag cloud fragment for tag search.
///
/// # Errors
/// Returns `500 Internal Server Error` if database queries or template rendering fails.
pub async fn tag_search(
    State(state): State<AppState>,
    _auth_session: AdminAuthSession,
    Query(params): Query<TagSearchParams>,
) -> Result<Html<String>, StatusCode> {
    let entries = get_tag_cloud(&state, params.filter.as_deref()).await?;
    let cloud = TagsCloud { tags: &entries };

    Ok(Html(
        cloud
            .render()
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
    auth_session: AdminAuthSession,
    Path(slug): Path<String>,
) -> Result<Html<String>, StatusCode> {
    let Some(tag) = get_tag_by_slug(&(*state.db_pool), &slug)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    else {
        return Err(StatusCode::NOT_FOUND);
    };

    let post_rows = list_posts_by_tag(&(*state.db_pool), &slug)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let post_views: Vec<PostView> = post_rows.into_iter().map(PostView::from).collect();

    let title = format!("Tag: {}", tag.name);

    let page = TagPostsPage {
        title,
        show_admin_link: auth_session.user.is_some(),
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

async fn get_tag_cloud(
    state: &AppState,
    filter: Option<&str>,
) -> Result<Vec<TagCloudEntry>, StatusCode> {
    let filter = filter.unwrap_or_default().trim().to_lowercase();
    let tags: Vec<_> = list_tags_with_published_count(&(*state.db_pool))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .filter(|tag| tag.count > 0)
        .filter(|tag| filter.is_empty() || tag.name.to_lowercase().contains(&filter))
        .collect();

    let max_count = tags.iter().map(|tag| tag.count).max().unwrap_or(0);

    Ok(tags
        .into_iter()
        .enumerate()
        .map(|(i, tag)| {
            let size = get_tag_size(tag.count, max_count);
            let color = TAG_CLOUD_COLORS[i % TAG_CLOUD_COLORS.len()];
            let class = format!("{size} {color}");
            TagCloudEntry {
                name: tag.name,
                slug: tag.slug,
                count: tag.count,
                class,
            }
        })
        .collect())
}

fn get_tag_size(count: i32, max_count: i32) -> &'static str {
    if max_count == 0 {
        return "text-sm";
    }
    let ratio = f64::from(count) / f64::from(max_count);
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
