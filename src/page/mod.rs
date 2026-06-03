//! Page route handlers.

pub use about::about_page;
use ammonia::clean;
pub use blog::{blog_index, blog_post};
pub use code::{code_index, code_project};
pub use home::home;
use pulldown_cmark::{Parser, html};
pub use tag::{tag_index, tag_posts};

use crate::database::{Post, PostWithTagRow, Tag};

mod about;
mod blog;
mod code;
mod home;
mod tag;

pub struct PostView {
    slug: String,
    title: String,
    body: String,
    preview: String,
    published_at: String,
    updated_at: String,
    tags: Vec<TagView>,
}

pub struct TagView {
    name: String,
    slug: String,
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
            tags: Vec::new(),
        }
    }
}

impl From<Tag> for TagView {
    fn from(tag: Tag) -> Self {
        Self {
            name: tag.name,
            slug: tag.slug,
        }
    }
}

pub fn build_post_views(rows: Vec<PostWithTagRow>) -> Vec<PostView> {
    let mut post_views = Vec::new();
    let mut current_post_id = None;

    for row in rows {
        let PostWithTagRow {
            id,
            title,
            slug,
            body,
            preview,
            published_at,
            created_at,
            updated_at,
            tag_name,
            tag_slug,
        } = row;

        if current_post_id != Some(id) {
            current_post_id = Some(id);
            post_views.push(PostView::from(Post {
                id,
                title,
                slug,
                body,
                preview,
                published_at,
                created_at,
                updated_at,
            }));
        }

        if let (Some(name), Some(slug)) = (tag_name, tag_slug)
            && let Some(post_view) = post_views.last_mut()
        {
            post_view.tags.push(TagView { name, slug });
        }
    }

    post_views
}

pub fn render_markdown(text: &str) -> String {
    let parser = Parser::new(text);
    let mut output = String::new();
    html::push_html(&mut output, parser);
    clean(&output)
}
