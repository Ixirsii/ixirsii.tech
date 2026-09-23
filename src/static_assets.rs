//! Fingerprinted URLs for embedded assets. Hashes are supplied by `build.rs`.

// Each entry is (source path, public URL). Add an entry here when adding an asset.
// Hash variable names use the exact path relative to src/static, including case.
// env! and concat! bake these URLs into the executable; no runtime environment is needed.
const STATIC_ASSETS: &[(&str, &str)] = &[
    (
        ".well-known/bimi/logo.svg",
        concat!(
            "/static/.well-known/bimi/logo.",
            env!("ASSET_HASH_.well-known/bimi/logo.svg"),
            ".svg"
        ),
    ),
    (
        ".well-known/security.txt",
        concat!(
            "/static/.well-known/security.",
            env!("ASSET_HASH_.well-known/security.txt"),
            ".txt"
        ),
    ),
    (
        "android-chrome-192x192.png",
        concat!(
            "/static/android-chrome-192x192.",
            env!("ASSET_HASH_android-chrome-192x192.png"),
            ".png"
        ),
    ),
    (
        "android-chrome-512x512.png",
        concat!(
            "/static/android-chrome-512x512.",
            env!("ASSET_HASH_android-chrome-512x512.png"),
            ".png"
        ),
    ),
    (
        "apple-touch-icon.png",
        concat!(
            "/static/apple-touch-icon.",
            env!("ASSET_HASH_apple-touch-icon.png"),
            ".png"
        ),
    ),
    (
        "favicon-16x16.png",
        concat!(
            "/static/favicon-16x16.",
            env!("ASSET_HASH_favicon-16x16.png"),
            ".png"
        ),
    ),
    (
        "favicon-32x32.png",
        concat!(
            "/static/favicon-32x32.",
            env!("ASSET_HASH_favicon-32x32.png"),
            ".png"
        ),
    ),
    (
        "favicon.ico",
        concat!("/static/favicon.", env!("ASSET_HASH_favicon.ico"), ".ico"),
    ),
    (
        "htmx.min.js",
        concat!("/static/htmx.min.", env!("ASSET_HASH_htmx.min.js"), ".js"),
    ),
    (
        "images/Ixirsii.webp",
        concat!(
            "/static/images/Ixirsii.",
            env!("ASSET_HASH_images/Ixirsii.webp"),
            ".webp"
        ),
    ),
    (
        "main.css",
        concat!("/static/main.", env!("ASSET_HASH_main.css"), ".css"),
    ),
    (
        "site.webmanifest",
        concat!(
            "/static/site.",
            env!("ASSET_HASH_site.webmanifest"),
            ".webmanifest"
        ),
    ),
];

/// Returns the fingerprinted URL, or `/static/missing` for an unknown asset.
#[must_use]
pub fn static_url(source_path: &str) -> &'static str {
    STATIC_ASSETS
        .iter()
        .find(|(source, _)| *source == source_path)
        .map_or("/static/missing", |(_, url)| *url)
}

/// Resolves a fingerprinted request path with the `/static/` prefix removed.
#[must_use]
pub fn source_path_for_request(request_path: &str) -> Option<&'static str> {
    STATIC_ASSETS.iter().find_map(|(source, url)| {
        (url.strip_prefix("/static/") == Some(request_path)).then_some(*source)
    })
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
        for &(source, url) in STATIC_ASSETS {
            let file = STATIC_DIR.get_file(source).ok_or("asset is not embedded")?;
            let digest = format!("{:x}", Sha256::digest(file.contents()));
            let fingerprint = &digest[..16];
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
            let request_path = url.strip_prefix("/static/").ok_or("missing URL prefix")?;
            assert_eq!(source_path_for_request(request_path), Some(source));

            let stale_path = request_path.replace(fingerprint, "stale");
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
            static_url("main.css"),
        ] {
            assert_eq!(source_path_for_request(path), None);
        }
    }
}
