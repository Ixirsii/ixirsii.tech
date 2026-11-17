//! Shared limits and URL-safe slugs for admin forms.

pub const POST_TITLE_MAX: usize = 255;
pub const POST_SLUG_MAX: usize = 255;
pub const POST_PREVIEW_MAX: usize = 512;
pub const TAG_TEXT_MAX: usize = 127;
// An application limit, in bytes, to bound Markdown rendering and storage.
pub const POST_BODY_MAX_BYTES: usize = 65_535;

pub fn text_error(value: &str, label: &str, max_chars: usize) -> Option<String> {
    if value.contains('\0') {
        Some(format!("{label} cannot contain null characters."))
    } else if value.chars().count() > max_chars {
        Some(format!("{label} must be at most {max_chars} characters."))
    } else {
        None
    }
}

pub fn slug_error(value: &str, max_chars: usize) -> Option<String> {
    text_error(value, "Slug", max_chars).or_else(|| {
        (!value.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        }))
        .then(|| "Use lowercase letters and numbers separated by single hyphens.".to_owned())
    })
}

pub fn body_error(value: &str) -> Option<String> {
    if value.trim().is_empty() {
        Some("Body is required.".to_owned())
    } else if value.contains('\0') {
        Some("Body cannot contain null characters.".to_owned())
    } else if value.len() > POST_BODY_MAX_BYTES {
        Some(format!(
            "Body must be at most {POST_BODY_MAX_BYTES} bytes in UTF-8."
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_are_single_canonical_path_segments() {
        for slug in ["post", "post-123", "2026", "a-b-c", &"a".repeat(255)] {
            assert!(slug_error(slug, POST_SLUG_MAX).is_none(), "{slug}");
        }
        for slug in [
            "",
            "-a",
            "a-",
            "a--b",
            "A",
            "é",
            "a b",
            " a",
            "a/",
            "a\\b",
            "a?b",
            "a#b",
            "a%20b",
            ".",
            "..",
            &"a".repeat(256),
        ] {
            assert!(slug_error(slug, POST_SLUG_MAX).is_some(), "{slug}");
        }
    }

    #[test]
    fn postgres_varchar_limits_count_unicode_characters() {
        assert!(text_error(&"🦀".repeat(255), "Title", POST_TITLE_MAX).is_none());
        assert!(text_error(&"🦀".repeat(256), "Title", POST_TITLE_MAX).is_some());
        assert!(text_error(&"é".repeat(512), "Preview", POST_PREVIEW_MAX).is_none());
        assert!(text_error(&"é".repeat(513), "Preview", POST_PREVIEW_MAX).is_some());
        assert!(text_error(&"é".repeat(127), "Name", TAG_TEXT_MAX).is_none());
        assert!(text_error(&"é".repeat(128), "Name", TAG_TEXT_MAX).is_some());
        assert!(text_error("a\0b", "Name", TAG_TEXT_MAX).is_some());
    }

    #[test]
    fn markdown_limit_counts_utf8_bytes() {
        assert!(body_error(&"a".repeat(POST_BODY_MAX_BYTES)).is_none());
        assert!(body_error(&"a".repeat(POST_BODY_MAX_BYTES + 1)).is_some());
        assert!(body_error(&"🦀".repeat(16_383)).is_none());
        assert!(body_error(&"🦀".repeat(16_384)).is_some());
    }
}
