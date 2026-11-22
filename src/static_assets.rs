//! Catalog and routes for embedded site and shared assets.

use axum::{
    extract::Path,
    http::{StatusCode, Uri},
    response::{IntoResponse, Response},
};
use include_dir::{Dir, include_dir};
use nine_dev_site_lib::{
    assets,
    catalog::{Catalog, Entry},
    serving::{self, IMMUTABLE, content_type},
};

static STATIC_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/public");
const WELL_KNOWN_CACHE: &str = "public, max-age=3600";

macro_rules! asset {
    ($path:literal) => {
        asset!($path, env!(concat!("ASSET_URL_", $path)))
    };
    ($path:literal, $url:expr) => {
        Entry {
            source: $path,
            url: $url,
            hash: env!(concat!("ASSET_HASH_", $path)),
        }
    };
}

// Paths are relative to public and case-sensitive. Tests check full coverage.
const STATIC_ASSETS: Catalog = Catalog(&[
    asset!(".well-known/bimi/logo.svg"),
    asset!(".well-known/security.txt"),
    asset!("admin-confirm.js"),
    asset!("android-chrome-192x192.png"),
    asset!("android-chrome-512x512.png"),
    asset!("apple-touch-icon.png"),
    // BEGIN dotfiles screenshots
    asset!("dotfiles/screenshots/common/neovim.png"),
    asset!("dotfiles/screenshots/freebsd/desktop.png"),
    asset!("dotfiles/screenshots/freebsd/waybar.png"),
    asset!("dotfiles/screenshots/macos/desktop.png"),
    asset!("dotfiles/screenshots/macos/sketchybar.png"),
    // END dotfiles screenshots
    asset!("favicon-16x16.png"),
    asset!("favicon-32x32.png"),
    asset!("favicon.ico"),
    asset!("fonts/mulish-OFL.txt"),
    asset!("fonts/mulish-italic.woff2"),
    asset!("fonts/mulish-normal.woff2"),
    asset!("fonts/sourcecodepro-OFL.txt"),
    asset!("fonts/sourcecodepro-italic.woff2"),
    asset!("fonts/sourcecodepro-normal.woff2"),
    asset!("images/Ixirsii.webp"),
    asset!("main.css"),
    asset!("post-dates.js"),
]);

// Keep the stable discovery URLs alongside the immutable versions.
const WELL_KNOWN_ALIASES: Catalog = Catalog(&[
    asset!(".well-known/bimi/logo.svg", "/.well-known/bimi/logo.svg"),
    asset!(".well-known/security.txt", "/.well-known/security.txt"),
]);

/// Returns a site-owned fingerprinted URL, or `/static/missing` for an unknown asset.
#[must_use]
pub fn static_url(source_path: &str) -> &'static str {
    STATIC_ASSETS
        .by_source(source_path)
        .map_or("/static/missing", |entry| entry.url)
}

/// Resolves a canonical percent-encoded request path for a site-owned `/static/` asset.
#[must_use]
pub fn source_path_for_request(request_path: &str) -> Option<&'static str> {
    STATIC_ASSETS.by_url(request_path).map(|entry| entry.source)
}

pub(crate) async fn static_asset(uri: Uri) -> Response {
    if let Some(asset) = assets::lookup(uri.path()) {
        return serving::response(asset.bytes, asset.content_type, IMMUTABLE);
    }
    serve_embedded(source_path_for_request(uri.path()), IMMUTABLE)
}

pub(crate) async fn well_known_asset(Path(path): Path<String>) -> Response {
    let url = format!("/.well-known/{path}");
    let source = WELL_KNOWN_ALIASES.by_url(&url).map(|entry| entry.source);
    serve_embedded(source, WELL_KNOWN_CACHE)
}

fn serve_embedded(source: Option<&str>, cache: &'static str) -> Response {
    let Some((source, file)) =
        source.and_then(|source| STATIC_DIR.get_file(source).map(|file| (source, file)))
    else {
        return StatusCode::NOT_FOUND.into_response();
    };
    serving::response(file.contents(), content_type(source), cache)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use axum::{body::to_bytes, http::header};
    use nine_dev_site_lib::assets::htmx;
    use sha2::{Digest, Sha256};

    use super::{
        Dir, IMMUTABLE, Path, STATIC_ASSETS, STATIC_DIR, StatusCode, WELL_KNOWN_ALIASES,
        WELL_KNOWN_CACHE, assets, source_path_for_request, static_asset, static_url,
        well_known_asset,
    };

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
            .0
            .iter()
            .map(|entry| entry.source.to_owned())
            .collect();
        let urls: BTreeSet<_> = STATIC_ASSETS
            .0
            .iter()
            .chain(WELL_KNOWN_ALIASES.0)
            .map(|entry| entry.url)
            .collect();

        assert_eq!(
            registered, embedded,
            "update the asset table when files change"
        );
        assert_eq!(
            registered.len(),
            STATIC_ASSETS.0.len(),
            "duplicate source paths"
        );
        assert_eq!(
            urls.len(),
            STATIC_ASSETS.0.len() + WELL_KNOWN_ALIASES.0.len(),
            "duplicate public URLs"
        );
        assert!(
            urls.iter().all(|url| assets::lookup(url).is_none()),
            "site asset URL collides with a shared asset"
        );
        let embedded_discovery_urls: BTreeSet<_> = embedded
            .iter()
            .filter(|source| source.starts_with(".well-known/"))
            .map(|source| format!("/{source}"))
            .collect();
        let alias_urls: BTreeSet<_> = WELL_KNOWN_ALIASES
            .0
            .iter()
            .map(|alias| alias.url.to_owned())
            .collect();
        assert_eq!(embedded_discovery_urls, alias_urls);
    }

    #[test]
    fn urls_match_embedded_contents_and_resolve() -> Result<(), Box<dyn std::error::Error>> {
        const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

        for entry in STATIC_ASSETS.0 {
            let (source, url) = (entry.source, entry.url);
            let file = STATIC_DIR.get_file(source).ok_or("asset is not embedded")?;
            let digest = Sha256::digest(file.contents());
            let mut fingerprint = String::with_capacity(16);
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
            assert_eq!(entry.hash, fingerprint);
            assert_eq!(url, format!("/static/{directory}{fingerprinted_name}"));
            assert_eq!(static_url(source), url);
            assert_eq!(source_path_for_request(url), Some(source));

            let stale_path = url.replace(&fingerprint, "stale");
            assert_eq!(source_path_for_request(&stale_path), None);
            assert_eq!(source_path_for_request(source), None);
        }
        Ok(())
    }

    #[tokio::test]
    async fn well_known_aliases_preserve_bytes_and_cache_policy()
    -> Result<(), Box<dyn std::error::Error>> {
        for alias in WELL_KNOWN_ALIASES.0 {
            let immutable = STATIC_ASSETS
                .by_source(alias.source)
                .ok_or("alias lacks a fingerprinted entry")?;
            assert_eq!(alias.hash, immutable.hash);
            assert_eq!(source_path_for_request(alias.url), None);
            let path = alias
                .url
                .strip_prefix("/.well-known/")
                .ok_or("invalid alias path")?;
            let response = well_known_asset(Path(path.to_owned())).await;
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.headers()[header::CACHE_CONTROL], WELL_KNOWN_CACHE);
            let body = to_bytes(response.into_body(), usize::MAX).await?;
            let file = STATIC_DIR
                .get_file(alias.source)
                .ok_or("missing alias bytes")?;
            assert_eq!(body.as_ref(), file.contents());
        }
        for path in ["missing.txt", "../main.css", "bimi/../security.txt"] {
            let response = well_known_asset(Path(path.to_owned())).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn shared_htmx_keeps_its_immutable_url() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(htmx::JS.url, "/static/htmx.min.e484d9171a9db30a.js");
        let response = static_asset(htmx::JS.url.parse()?).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CACHE_CONTROL], IMMUTABLE);
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            htmx::JS.content_type
        );
        let body = to_bytes(response.into_body(), usize::MAX).await?;
        assert_eq!(body.as_ref(), htmx::JS.bytes);
        for path in [
            "/static/htmx.min.js",
            "/static/htmx.min.0000000000000000.js",
        ] {
            assert_eq!(
                static_asset(path.parse()?).await.status(),
                StatusCode::NOT_FOUND
            );
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
