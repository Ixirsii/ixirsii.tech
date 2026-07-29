//! Tag route handlers.

use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::Html,
};
use serde::Deserialize;

use crate::{
    AdminAuthSession, AppState,
    database::{get_tag_by_slug, list_posts_by_tag, list_tags_with_published_count},
    page::{PageError, PostView},
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
const TAG_CLOUD_COLOR_COUNT: u64 = 12;

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
) -> Result<Html<String>, PageError> {
    let show_admin_link = auth_session.user.is_some();
    let entries = get_tag_cloud(&state, None, show_admin_link).await?;

    let page = TagsPage {
        title: "Tags",
        show_admin_link,
        tags: &entries,
    };

    Ok(Html(page.render().map_err(|_| {
        PageError::internal_server_error(show_admin_link)
    })?))
}

/// Renders the tag cloud fragment for tag search.
///
/// # Errors
/// Returns `500 Internal Server Error` if database queries or template rendering fails.
pub async fn tag_search(
    State(state): State<AppState>,
    auth_session: AdminAuthSession,
    Query(params): Query<TagSearchParams>,
) -> Result<Html<String>, PageError> {
    let show_admin_link = auth_session.user.is_some();
    let entries = get_tag_cloud(&state, params.filter.as_deref(), show_admin_link).await?;
    let cloud = TagsCloud { tags: &entries };

    Ok(Html(cloud.render().map_err(|_| {
        PageError::internal_server_error(show_admin_link)
    })?))
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
) -> Result<Html<String>, PageError> {
    let show_admin_link = auth_session.user.is_some();
    let Some(tag) = get_tag_by_slug(&(*state.db_pool), &slug)
        .await
        .map_err(|_| PageError::internal_server_error(show_admin_link))?
    else {
        return Err(PageError::not_found(show_admin_link));
    };

    let post_rows = list_posts_by_tag(&(*state.db_pool), &slug)
        .await
        .map_err(|_| PageError::internal_server_error(show_admin_link))?;
    let post_views: Vec<PostView> = post_rows.into_iter().map(PostView::from).collect();

    let title = format!("Tag: {}", tag.name);

    let page = TagPostsPage {
        title,
        show_admin_link,
        tag_name: tag.name,
        posts: &post_views,
    };

    Ok(Html(page.render().map_err(|_| {
        PageError::internal_server_error(show_admin_link)
    })?))
}

/* ********************************************************************************************** *
 *                                       Private functions                                        *
 * ********************************************************************************************** */

async fn get_tag_cloud(
    state: &AppState,
    filter: Option<&str>,
    show_admin_link: bool,
) -> Result<Vec<TagCloudEntry>, PageError> {
    let filter = filter.unwrap_or_default().trim().to_lowercase();
    let tags: Vec<_> = list_tags_with_published_count(&(*state.db_pool))
        .await
        .map_err(|_| PageError::internal_server_error(show_admin_link))?
        .into_iter()
        .filter(|tag| tag.count > 0)
        .filter(|tag| filter.is_empty() || tag.name.to_lowercase().contains(&filter))
        .collect();

    let max_count = tags.iter().map(|tag| tag.count).max().unwrap_or(0);

    Ok(tags
        .into_iter()
        .map(|tag| {
            let size = get_tag_size(tag.count, max_count);
            let color_index = usize::try_from(tag.id % TAG_CLOUD_COLOR_COUNT).unwrap_or_default();
            let color = TAG_CLOUD_COLORS[color_index];
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
