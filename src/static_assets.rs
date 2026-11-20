//! Fingerprinted URLs for embedded assets, supplied by `build.rs`.

macro_rules! asset {
    ($path:literal) => {
        ($path, env!(concat!("ASSET_URL_", $path)))
    };
}

// Paths are relative to src/static and case-sensitive. Tests check full coverage.
const STATIC_ASSETS: &[(&str, &str)] = &[
    asset!(".well-known/bimi/logo.svg"),
    asset!(".well-known/security.txt"),
    asset!("android-chrome-192x192.png"),
    asset!("android-chrome-512x512.png"),
    asset!("apple-touch-icon.png"),
    asset!("favicon-16x16.png"),
    asset!("favicon-32x32.png"),
    asset!("favicon.ico"),
    asset!("htmx.min.js"),
    asset!("images/Ixirsii.webp"),
    asset!("main.css"),
];

/// Returns the fingerprinted URL, or `/static/missing` for an unknown asset.
#[must_use]
pub fn static_url(source_path: &str) -> &'static str {
    STATIC_ASSETS
        .iter()
        .find(|(source, _)| *source == source_path)
        .map_or("/static/missing", |(_, url)| *url)
}

/// Resolves a canonical percent-encoded request path, including `/static/`.
#[must_use]
pub fn source_path_for_request(request_path: &str) -> Option<&'static str> {
    STATIC_ASSETS
        .iter()
        .find_map(|(source, url)| (*url == request_path).then_some(*source))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use include_dir::Dir;
    use sha2::{Digest, Sha256};

    use super::{STATIC_ASSETS, source_path_for_request, static_url};
    use crate::STATIC_DIR;

    fn embedded_paths(dir: &Dir<'_>, paths: &mut BTreeSet<String>) {
        for file in dir.files() {
            paths.insert(file.path().to_string_lossy().replace('\\', "/"));
        }
        for child in dir.dirs() {
            embedded_paths(child, paths);
        }
    }

    #[test]
    fn table_covers_every_embedded_asset_once() {
        let mut embedded = BTreeSet::new();
        embedded_paths(&STATIC_DIR, &mut embedded);
        let registered: BTreeSet<_> = STATIC_ASSETS
            .iter()
            .map(|(source, _)| (*source).to_owned())
            .collect();
        let urls: BTreeSet<_> = STATIC_ASSETS.iter().map(|(_, url)| *url).collect();

        assert_eq!(
            registered, embedded,
            "update the asset table when files change"
        );
        assert_eq!(
            registered.len(),
            STATIC_ASSETS.len(),
            "duplicate source paths"
        );
        assert_eq!(urls.len(), STATIC_ASSETS.len(), "duplicate public URLs");
    }

    #[test]
    fn urls_match_embedded_contents_and_resolve() -> Result<(), Box<dyn std::error::Error>> {
        const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

        for &(source, url) in STATIC_ASSETS {
            let file = STATIC_DIR.get_file(source).ok_or("asset is not embedded")?;
            let digest = Sha256::digest(file.contents());
            let mut fingerprint = String::with_capacity(12);
            for byte in &digest[..8] {
                fingerprint.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
                fingerprint.push(char::from(HEX_DIGITS[usize::from(byte & 0x0f)]));
            }
            let (directory, name) = source
                .rsplit_once('/')
                .map_or((String::new(), source), |(dir, name)| {
                    (format!("{dir}/"), name)
                });
            let fingerprinted_name = match name.rsplit_once('.') {
                Some((stem, extension)) => format!("{stem}.{fingerprint}.{extension}"),
                None => format!("{name}.{fingerprint}"),
            };
            assert_eq!(url, format!("/static/{directory}{fingerprinted_name}"));
            assert_eq!(static_url(source), url);
            assert_eq!(source_path_for_request(url), Some(source));

            let stale_path = url.replace(&fingerprint, "stale");
            assert_eq!(source_path_for_request(&stale_path), None);
            assert_eq!(source_path_for_request(source), None);
        }
        Ok(())
    }

    #[test]
    fn unknown_assets_do_not_resolve() {
        assert_eq!(static_url("missing.css"), "/static/missing");
        for path in [
            "",
            "missing",
            "missing.css",
            "../main.css",
            "/static/main.css",
        ] {
            assert_eq!(source_path_for_request(path), None);
        }
    }
}
