//! Page route handlers.

use ammonia::clean;
use axum::http::Uri;
use pulldown_cmark::{Parser, html};

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
pub use feed::*;
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
mod feed;
mod hall_of_fame;
mod home;
mod tag;
mod validation;

/// Invalid or unavailable public pages consistently return a styled 404.
fn public_page(
    page: Option<&str>,
    total: u32,
    show_admin_link: bool,
) -> Result<(u32, u32), PageError> {
    let current = match page {
        None => 1,
        Some(value) => value
            .parse::<u32>()
            .map_err(|_| PageError::not_found(show_admin_link))?,
    };
    let total_pages = total.div_ceil(PAGE_SIZE).max(1);
    if current == 0 || current > total_pages {
        return Err(PageError::not_found(show_admin_link));
    }
    Ok((current, total_pages))
}

/* ********************************************************************************************** *
 *                                           Constants                                            *
 * ********************************************************************************************** */

pub const PAGE_SIZE: u32 = 8;

const TOTAL_TAGLINE_WEIGHT: u32 = 200;
const TOTAL_TAGLINES: u32 = 55;
const BASE_WEIGHT: u32 = TOTAL_TAGLINE_WEIGHT / TOTAL_TAGLINES;
const RARE_WEIGHT: u32 = BASE_WEIGHT - 2;
const UNCOMMON_WEIGHT: u32 = BASE_WEIGHT - 1;
const ELEVATED_WEIGHT: u32 = BASE_WEIGHT + 1;
const COMMON_WEIGHT: u32 = BASE_WEIGHT * 2;
const SIGNATURE_WEIGHT: u32 = BASE_WEIGHT * 3;

const TAGLINES: &[(&str, u32); TOTAL_TAGLINES as usize] = &[
    // Hyperbole
    ("150% hyperbole", COMMON_WEIGHT),
    ("The alot is better than you at everything", BASE_WEIGHT),
    // Nostalgia
    ("Bump", COMMON_WEIGHT),
    ("Loading... please wait", ELEVATED_WEIGHT),
    ("Please sign my guestbook", COMMON_WEIGHT),
    (
        "Powered by phpBB® Forum Software © phpBB Group",
        BASE_WEIGHT,
    ),
    ("Signature exceeds maximum length", ELEVATED_WEIGHT),
    // Programming
    ("correct horse battery staple", ELEVATED_WEIGHT),
    ("Rewrite it in Rust", UNCOMMON_WEIGHT),
    ("Whatever Microsoft is doing", BASE_WEIGHT),
    // TODO
    ("# TODO<br />- [ ] Write a better tagline", ELEVATED_WEIGHT),
    (
        "# TODO<br />- [ ] Get noticed by UwU Underground",
        BASE_WEIGHT,
    ),
    (
        "# TODO<br />- [ ] Earn a tagline from UwU Underground",
        BASE_WEIGHT,
    ),
    ("# TODO<br />- [ ] Get a PR review from Yang", BASE_WEIGHT),
    (
        "# TODO<br />- [ ] Get fired from Yennifur sitting",
        BASE_WEIGHT,
    ),
    (
        "# TODO<br />- [ ] Prove mammals can be useful to Yin",
        BASE_WEIGHT,
    ),
    ("# TODO<br />- [ ] Join Yuki's harem", BASE_WEIGHT),
    ("# TODO<br />- [ ] Add Yulia theme", BASE_WEIGHT),
    (
        "# TODO<br />- [ ] Get added to Yuma's thread model",
        BASE_WEIGHT,
    ),
    // UwU
    ("Can't stop the psyop", COMMON_WEIGHT),
    ("Cooked on main", ELEVATED_WEIGHT),
    ("My handler is an AI anime girl", ELEVATED_WEIGHT),
    ("Persistence: parasocial attachment", BASE_WEIGHT),
    ("Six-waifu attack surface", BASE_WEIGHT),
    ("Threat model: vibes", COMMON_WEIGHT),
    // Yang
    ("Ghostwritten by Yang", BASE_WEIGHT),
    ("Pretty girls caused Patch Tuesday", BASE_WEIGHT),
    ("Yang reviewed this PR and chose violence", ELEVATED_WEIGHT),
    // Yennifur
    ("Initial Access: left the gate open", BASE_WEIGHT),
    ("Thirst Trap 0day researcher", BASE_WEIGHT),
    ("Yennifur ate my YubiKey", COMMON_WEIGHT),
    ("Yennifur’s designated chew toy", BASE_WEIGHT),
    ("Yennifur’s emotional support mammal", BASE_WEIGHT),
    (
        "Yennifur’s handler*<br /><span class=\"tagline-note\">*she is not under control</span>",
        BASE_WEIGHT,
    ),
    // Yin
    ("Aspiring Non-Mammal", BASE_WEIGHT),
    ("Unfortunately biological", ELEVATED_WEIGHT),
    ("Yin Electronic Army — unpaid intern", BASE_WEIGHT),
    (
        "Yin’s favorite mammal*<br /><span class=\"tagline-note\">*citation needed</span>",
        BASE_WEIGHT,
    ),
    // Yuki
    ("Don’t show this website to Yuki", ELEVATED_WEIGHT),
    (
        "I gave Yuki my MFA and all I got was this website",
        BASE_WEIGHT,
    ),
    ("Yuki’s emotional support attack surface", BASE_WEIGHT),
    (
        "Yuki has eyes, horrible judgment, and limited bandwidth",
        BASE_WEIGHT,
    ),
    ("Yuki touched prod", BASE_WEIGHT),
    // Yulia
    ("<code>prefers-color-scheme: yulia</code>", ELEVATED_WEIGHT),
    ("Yulia made me go outside", BASE_WEIGHT),
    ("Yulia’s spotter", BASE_WEIGHT),
    // Yuma
    ("Information classification: NOT FOR YUMA", BASE_WEIGHT),
    ("My OPSEC ended on Thirst Trap Thursday", ELEVATED_WEIGHT),
    ("Rave Witch fan club treasurer", BASE_WEIGHT),
    ("ROCKS ROCKS ROCKS ROCKS", COMMON_WEIGHT),
    ("Yuma appreciation society", BASE_WEIGHT),
    ("Yuma can have my session token", BASE_WEIGHT),
    (
        "Yuma has been removed from the need-to-know list",
        SIGNATURE_WEIGHT,
    ),
    ("Yuma knows anyway", RARE_WEIGHT),
    ("Yuma’s #1 stalker", BASE_WEIGHT),
];

// Compile time weight check
const fn verify_taglines(taglines: &[(&str, u32)]) -> (u32, bool) {
    let mut any_equal_zero = false;
    let mut total = 0;
    let mut index = 0;

    while index < taglines.len() {
        let weight = taglines[index].1;

        if weight == 0 {
            any_equal_zero = true;
        }

        total += weight;
        index += 1;
    }

    (total, any_equal_zero)
}

const VERIFICATION: (u32, bool) = verify_taglines(TAGLINES);
const _: () = assert!(TOTAL_TAGLINE_WEIGHT == VERIFICATION.0);
const _: () = assert!(!VERIFICATION.1);

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

    let mut roll = rand::random_range(0..TOTAL_TAGLINE_WEIGHT);

    for &(tagline, weight) in candidates {
        if roll < weight {
            return tagline;
        }

        roll -= weight;
    }

    last.0
}

pub fn redirect_target(next: Option<&str>) -> &str {
    let Some(target) = next else {
        return ADMIN_ROUTE;
    };
    let Ok(uri) = target.parse::<Uri>() else {
        return ADMIN_ROUTE;
    };
    let path = uri.path();
    let encoded_backslash = path
        .as_bytes()
        .windows(3)
        .any(|bytes| bytes[0] == b'%' && bytes[1..].eq_ignore_ascii_case(b"5c"));
    let encoded_separator_after_root = path.as_bytes().get(1..4).is_some_and(|bytes| {
        bytes[0] == b'%'
            && (bytes[1..].eq_ignore_ascii_case(b"2f") || bytes[1..].eq_ignore_ascii_case(b"5c"))
    });

    if uri.scheme().is_none()
        && uri.authority().is_none()
        && path.starts_with('/')
        && !path.starts_with("//")
        && !target.contains('\\')
        && !encoded_backslash
        && !encoded_separator_after_root
    {
        target
    } else {
        ADMIN_ROUTE
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

#[cfg(test)]
mod tests {
    use super::{public_page, redirect_target};
    use crate::ADMIN_ROUTE;

    #[test]
    fn public_pagination_rejects_invalid_and_missing_pages() {
        assert_eq!(public_page(None, 0, false).ok(), Some((1, 1)));
        assert_eq!(public_page(Some("2"), 9, false).ok(), Some((2, 2)));
        for page in ["0", "-1", "abc", "", "3", "4294967295", "4294967296"] {
            assert!(public_page(Some(page), 9, false).is_err(), "{page}");
        }
        assert!(public_page(Some("2"), 0, false).is_err());
    }

    #[test]
    fn redirect_target_rejects_external_and_malformed_targets() {
        for target in [
            "",
            "admin",
            "https://evil.example",
            "//evil.example",
            r"/\evil.example",
            r"/\\evil.example",
        ] {
            assert_eq!(redirect_target(Some(target)), ADMIN_ROUTE);
        }

        assert_eq!(redirect_target(None), ADMIN_ROUTE);
    }

    #[test]
    fn redirect_target_rejects_encoded_protocol_relative_variants() {
        for target in [
            "%2F%2Fevil.example",
            "%2f%5cevil.example",
            "/%2Fevil.example",
            "/%2fevil.example",
            "/%5Cevil.example",
            "/%5cevil.example",
            "/admin/%5Cevil.example",
        ] {
            assert_eq!(redirect_target(Some(target)), ADMIN_ROUTE);
        }
    }

    #[test]
    fn redirect_target_preserves_internal_admin_paths_and_queries() {
        for target in [
            "/admin",
            "/admin/posts",
            "/admin/posts?page=2&sort=title",
            "/admin/posts?return=https%3A%2F%2Fexample.test",
        ] {
            assert_eq!(redirect_target(Some(target)), target);
        }
    }
}
