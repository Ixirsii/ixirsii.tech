//! Observability: request timing, tracing, and Datadog APM via OpenTelemetry.
//!
//! If `OTEL_EXPORTER_OTLP_ENDPOINT` is set, spans are exported to Datadog via
//! OTLP. Otherwise, only structured console logging is used.

use std::time::Instant;

use axum::{extract::Request, middleware::Next, response::Response};
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use tracing::{Instrument, Span, info, info_span};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Initialize the tracing subscriber.
///
/// Sets up a console logger (respecting `RUST_LOG`) and conditionally adds an
/// OpenTelemetry exporter for Datadog APM when `OTEL_EXPORTER_OTLP_ENDPOINT` is
/// configured.
pub fn init_tracing() {
    let fmt_layer = tracing_subscriber::fmt::layer().with_target(true);

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let registry = tracing_subscriber::Registry::default()
        .with(env_filter)
        .with(fmt_layer);

    let Some(endpoint) = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .ok()
        .filter(|v| !v.is_empty())
    else {
        registry.init();
        return;
    };

    match opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(endpoint)
        .build()
    {
        Ok(exporter) => {
            let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
                .with_batch_exporter(exporter)
                .build();
            let tracer = provider.tracer("ixirsii-tech");
            let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
            registry.with(otel_layer).init();
        }
        Err(err) => {
            eprintln!(
                "WARN: Failed to build OTLP exporter — falling back to console logging: {err}"
            );
            registry.init();
        }
    }
}

/// Axum middleware that wraps each HTTP request in a tracing span.
///
/// Records:
/// - `http.method` — the HTTP verb
/// - `http.route` — the request path
/// - `http.status_code` — the response status
/// - `duration_ms` — total request-to-response time in milliseconds
pub async fn trace_request(request: Request, next: Next) -> Response {
    let start = Instant::now();
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
        info!(
            duration_ms = start.elapsed().as_secs_f64() * 1000.0,
            "request completed"
        );
        response
    }
    .instrument(span)
    .await
}
