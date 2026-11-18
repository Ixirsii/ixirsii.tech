//! Request tracing instrumentation.
//!
//! No subscriber is installed, so tracing events and spans produce no output.

use axum::{extract::Request, middleware::Next, response::Response};
use tracing::{Instrument, Span, info_span};

/// Axum middleware that wraps each HTTP request in a tracing span.
///
/// Records:
/// - `http.method` — the HTTP verb
/// - `http.route` — the request path
/// - `http.status_code` — the response status
pub async fn trace_request(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();

    let span = info_span!(
        "http.request",
        http.method = %method,
        http.route = %uri.path(),
        http.status_code = tracing::field::Empty,
    );

    async {
        let response = next.run(request).await;
        let status = response.status();
        Span::current().record("http.status_code", status.as_u16());
        response
    }
    .instrument(span)
    .await
}
