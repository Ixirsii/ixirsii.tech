//! Page route handlers.

pub use about::about_page;
pub use blog::{blog_index, blog_post};
pub use code::{code_index, code_project};
pub use home::home;
use pulldown_cmark::{Parser, html};
pub use tag::{tag_index, tag_posts};

use crate::database::Post;

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
    tags_html: String,
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

pub fn render_markdown(text: &str) -> String {
    let parser = Parser::new(text);
    let mut output = String::new();
    html::push_html(&mut output, parser);
    output
}
