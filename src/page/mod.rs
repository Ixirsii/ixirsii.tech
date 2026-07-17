//! Page route handlers.

use ammonia::clean;
use pulldown_cmark::{Parser, html};
use rand::Rng;

use crate::{
    ADMIN_ROUTE,
    database::{Post, Tag},
};

pub use about::*;
pub use admin::*;
pub use admin_posts::*;
pub use admin_tags::*;
pub use blog::*;
pub use code::*;
pub use error::*;
pub use hall_of_fame::*;
pub use home::*;
pub use tag::*;

mod about;
mod admin;
mod admin_posts;
mod admin_tags;
mod blog;
mod code;
mod error;
mod hall_of_fame;
mod home;
mod tag;

/* ********************************************************************************************** *
 *                                           Constants                                            *
 * ********************************************************************************************** */

pub const PAGE_SIZE: u32 = 8;

const TOTAL_TAGLINE_WEIGHT: u32 = 100;

const TAGLINES: &[(&str, u32)] = &[
    // Hyperbole
    ("150% hyperbole", 7),
    ("It's fine. Statistically.", 6),
    ("The alot is better than you at everything", 6),
    // Nostalgia
    ("Bump", 6),
    ("Loading... please wait", 6),
    ("Please sign my guestbook", 6),
    ("Powered by phpBB® Forum Software © phpBB Group", 6),
    ("Signature exceeds maximum length", 7),
    // Programming
    ("Rewrite it in Rust", 6),
    ("\"This isn't in scope.\" — Mangement", 6),
    ("TODO: Write a better tagline", 6),
    // UwU
    ("Can't stop the psyop", 7),
    ("My coffee maker has two-factor authentication", 6),
    ("Security through sleep deprivation", 6),
    ("Threat model: vibes", 7),
    ("Yuma said it was \"probably fine.\"", 6),
];

const fn total_tagline_weight(taglines: &[(&str, u32)]) -> u32 {
    let mut total = 0;
    let mut index = 0;

    while index < taglines.len() {
        total += taglines[index].1;
        index += 1;
    }

    total
}

const _: () = assert!(TOTAL_TAGLINE_WEIGHT == total_tagline_weight(TAGLINES));

/* ********************************************************************************************** *
 *                                           Post view                                            *
 * ********************************************************************************************** */

pub struct PostView {
    slug: String,
    title: String,
    body: String,
    preview: String,
    published_at: String,
    updated_at: String,
    tags: Vec<TagView>,
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
            tags: post.tags.into_iter().map(TagView::from).collect(),
        }
    }
}

/* ********************************************************************************************** *
 *                                            Tag view                                            *
 * ********************************************************************************************** */

pub struct TagView {
    name: String,
    slug: String,
}

impl From<Tag> for TagView {
    fn from(tag: Tag) -> Self {
        Self {
            name: tag.name,
            slug: tag.slug,
        }
    }
}

/* ********************************************************************************************** *
 *                                           Functions                                            *
 * ********************************************************************************************** */

pub fn random_tagline() -> &'static str {
    let Some((last, candidates)) = TAGLINES.split_last() else {
        return "";
    };

    let mut roll = rand::rng().random_range(0..TOTAL_TAGLINE_WEIGHT);

    for &(tagline, weight) in candidates {
        if roll < weight {
            return tagline;
        }

        roll -= weight;
    }

    last.0
}

pub fn redirect_target(next: Option<&str>) -> &str {
    match next {
        Some(path) if path.starts_with('/') && !path.starts_with("//") => path,
        _ => ADMIN_ROUTE,
    }
}

pub fn render_markdown(text: &str) -> String {
    let parser = Parser::new(text);
    let mut output = String::new();
    html::push_html(&mut output, parser);
    clean(&output)
}

pub fn slugify(text: &str) -> String {
    let mut slug = String::new();
    let mut last_was_separator = false;

    for character in text.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            last_was_separator = false;
        } else if !last_was_separator && !slug.is_empty() {
            slug.push('-');
            last_was_separator = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    slug
}
