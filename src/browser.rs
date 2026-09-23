//! Shared browser response policy and install metadata.

use axum::{
    Json,
    extract::Request,
    http::{HeaderValue, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::static_assets::static_url;

pub async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("frame-ancestors 'none'"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    response
}

#[derive(Serialize)]
struct Manifest {
    id: &'static str,
    name: &'static str,
    short_name: &'static str,
    start_url: &'static str,
    scope: &'static str,
    display: &'static str,
    theme_color: &'static str,
    background_color: &'static str,
    icons: [Icon; 2],
}

#[derive(Serialize)]
struct Icon {
    src: &'static str,
    sizes: &'static str,
    #[serde(rename = "type")]
    mime: &'static str,
}

pub async fn manifest() -> Response {
    let manifest = Manifest {
        id: "/",
        name: "Ixirsii's website",
        short_name: "Ixirsii",
        start_url: "/",
        scope: "/",
        display: "standalone",
        theme_color: "#1e1e2e",
        background_color: "#1e1e2e",
        icons: [
            Icon {
                src: static_url("android-chrome-192x192.png"),
                sizes: "192x192",
                mime: "image/png",
            },
            Icon {
                src: static_url("android-chrome-512x512.png"),
                sizes: "512x512",
                mime: "image/png",
            },
        ],
    };
    // Revalidate this stable URL; referenced icon URLs remain immutable.
    (
        [
            (header::CONTENT_TYPE, "application/manifest+json"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        Json(manifest),
    )
        .into_response()
}
