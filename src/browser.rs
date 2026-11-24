//! Shared browser response policy.

use axum::{
    extract::Request,
    http::{HeaderValue, header},
    middleware::Next,
    response::Response,
};

pub async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; style-src 'self'; font-src 'self'; \
             img-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'none'; \
             frame-ancestors 'none'; form-action 'self'",
        ),
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
