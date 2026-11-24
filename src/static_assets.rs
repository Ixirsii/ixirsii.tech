//! Catalog and routes for embedded site and shared assets.

use axum::Router;
use include_dir::{Dir, File, include_dir};
use nine_dev_site_lib::{catalog::SiteCatalog, serving};

static PUBLIC: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/public");
pub(crate) static CATALOG: SiteCatalog = include!(concat!(env!("OUT_DIR"), "/public_assets.rs"));

pub(crate) fn routes() -> Router {
    serving::asset_router(&CATALOG, |source| {
        PUBLIC.get_file(source).map(File::contents)
    })
}

/// Return the immutable URL when a file also has a stable alias.
#[must_use]
pub fn static_url(source: &str) -> &'static str {
    CATALOG
        .by_source(source)
        .map_or("/static/missing", |entry| entry.url)
}

#[cfg(test)]
#[must_use]
pub fn source_path_for_request(url: &str) -> Option<&'static str> {
    CATALOG.immutable.by_url(url).map(|entry| entry.source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header},
    };
    use sha2::{Digest, Sha256};
    use std::collections::BTreeSet;
    use tower::ServiceExt;

    fn paths(dir: &Dir<'_>, out: &mut BTreeSet<String>) {
        for file in dir.files() {
            out.insert(file.path().to_string_lossy().replace('\\', "/"));
        }
        for child in dir.dirs() {
            paths(child, out);
        }
    }

    #[test]
    fn catalog_matches_embedded_assets() -> Result<(), Box<dyn std::error::Error>> {
        let mut embedded = BTreeSet::new();
        paths(&PUBLIC, &mut embedded);
        let mut declared = BTreeSet::new();
        let mut urls = BTreeSet::new();
        for entry in CATALOG
            .immutable
            .0
            .iter()
            .chain(CATALOG.aliases.iter().map(|a| &a.entry))
        {
            declared.insert(entry.source.to_owned());
            assert!(urls.insert(entry.url));
            assert!(nine_dev_site_lib::assets::lookup(entry.url).is_none());
            let bytes = PUBLIC
                .get_file(entry.source)
                .ok_or("missing asset")?
                .contents();
            let hash = format!(
                "{:016x}",
                u64::from_be_bytes(Sha256::digest(bytes)[..8].try_into()?)
            );
            assert_eq!(entry.hash, hash);
            if CATALOG.alias_by_url(entry.url).is_none() {
                assert_eq!(static_url(entry.source), entry.url);
                assert!(entry.url.contains(&format!(".{hash}.")));
                assert_eq!(source_path_for_request(entry.url), Some(entry.source));
                assert_eq!(
                    source_path_for_request(&entry.url.replace(&hash, "stale")),
                    None
                );
            }
        }
        assert_eq!(declared, embedded);
        Ok(())
    }

    #[tokio::test]
    async fn discovery_documents_revalidate_and_security_txt_is_stable_only()
    -> Result<(), Box<dyn std::error::Error>> {
        let app = routes();
        for source in [".well-known/security.txt", ".well-known/bimi/logo.svg"] {
            let url = format!("/{source}");
            let response = app
                .clone()
                .oneshot(Request::get(&url).body(Body::empty())?)
                .await?;
            assert_eq!(response.status(), StatusCode::OK);
            let etag = response.headers()[header::ETAG].to_str()?.to_owned();
            assert_eq!(
                response.headers()[header::CACHE_CONTROL],
                "public, max-age=3600"
            );
            let bytes = PUBLIC
                .get_file(source)
                .ok_or("missing document")?
                .contents();
            assert_eq!(
                to_bytes(response.into_body(), bytes.len()).await?.as_ref(),
                bytes
            );
            let cached = app
                .clone()
                .oneshot(
                    Request::head(&url)
                        .header(header::IF_NONE_MATCH, &etag)
                        .body(Body::empty())?,
                )
                .await?;
            assert_eq!(cached.status(), StatusCode::NOT_MODIFIED);
            assert_eq!(cached.headers()[header::ETAG], etag);
            assert_eq!(
                cached.headers()[header::CACHE_CONTROL],
                "public, max-age=3600"
            );
            assert!(to_bytes(cached.into_body(), 0).await?.is_empty());
        }
        let url = static_url(".well-known/security.txt");
        assert_eq!(url, "/.well-known/security.txt");
        let hash = CATALOG
            .alias_by_url(url)
            .ok_or("missing stable URL")?
            .entry
            .hash;
        let old = format!("/static/.well-known/security.{hash}.txt");
        let miss = app
            .clone()
            .oneshot(Request::get(old).body(Body::empty())?)
            .await?;
        assert_eq!(miss.status(), StatusCode::NOT_FOUND);
        assert!(!miss.headers().contains_key(header::CACHE_CONTROL));
        let bimi = static_url(".well-known/bimi/logo.svg");
        assert!(bimi.starts_with("/static/.well-known/bimi/"));
        assert_eq!(
            app.oneshot(Request::get(bimi).body(Body::empty())?)
                .await?
                .status(),
            StatusCode::OK
        );
        Ok(())
    }
}
