//! A bounded RSS 2.0 feed of published blog posts.

use askama::Template;
use axum::{
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};

use crate::{AppState, database::list_published_posts, page::render_markdown};

#[derive(Template)]
#[template(path = "feed.xml")]
struct Feed {
    items: Vec<FeedItem>,
}

struct FeedItem {
    title: String,
    url: String,
    description: String,
    published_at: String,
    guid: String,
}

/// Serve the newest 20 published posts with stable IDs and canonical links.
///
/// # Errors
/// Returns 500 if querying posts or rendering XML fails.
pub async fn rss_feed(State(state): State<AppState>) -> Result<Response, StatusCode> {
    let posts = list_published_posts(&(*state.db_pool), 1, 20)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let items = posts
        .into_iter()
        .filter_map(|post| {
            let published_at = post.published_at?;
            let mut url = reqwest::Url::parse("https://ixirsii.tech/blog/").ok()?;
            url.path_segments_mut()
                .ok()?
                .pop_if_empty()
                .push(&post.slug);
            Some(FeedItem {
                title: xml_text(&post.title),
                url: url.to_string(),
                description: xml_text(&render_markdown(
                    post.preview.as_deref().unwrap_or(&post.body),
                )),
                published_at: published_at.to_rfc2822(),
                guid: format!("https://ixirsii.tech/posts/{}", post.id),
            })
        })
        .collect();
    let body = Feed { items }
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/rss+xml; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=300"),
        ],
        body,
    )
        .into_response())
}

// XML 1.0 disallows some control characters that PostgreSQL TEXT can store.
fn xml_text(text: &str) -> String {
    text.chars().filter(|c| matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rss_escapes_titles_and_html_as_xml() -> Result<(), askama::Error> {
        let feed = Feed {
            items: vec![FeedItem {
                title: xml_text("Rust & <friends>\u{1}"),
                url: "https://ixirsii.tech/blog/test".to_owned(),
                description: "<p>Text & markup</p>".to_owned(),
                published_at: "Wed, 23 Sep 2026 12:00:00 +0000".to_owned(),
                guid: "https://ixirsii.tech/posts/1".to_owned(),
            }],
        }
        .render()?;
        assert!(feed.contains("Rust &#38; &#60;friends&#62;"), "{feed}");
        assert!(
            feed.contains("&#60;p&#62;Text &#38; markup&#60;/p&#62;"),
            "{feed}"
        );
        assert!(!feed.contains('\u{1}'));
        Ok(())
    }
}
