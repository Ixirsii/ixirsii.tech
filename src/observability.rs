//! Observability: request timing, tracing, and Datadog APM via OpenTelemetry.
//!
//! If `OTEL_EXPORTER_OTLP_ENDPOINT` is set, spans are exported to Datadog via
//! OTLP. Otherwise, only structured console logging is used.

use axum::{extract::Request, middleware::Next, response::Response};
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use tracing::{Instrument, Span, info_span};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Initialize the tracing subscriber.
///
/// Sets up a console logger (respecting `RUST_LOG`) and conditionally adds an
/// OpenTelemetry exporter for Datadog APM when `OTEL_EXPORTER_OTLP_ENDPOINT` is
/// configured.
pub fn init_tracing() -> crate::Result<()> {
    let fmt_layer = tracing_subscriber::fmt::layer().with_target(true);

    let env_filter = match std::env::var("RUST_LOG") {
        Ok(value) => tracing_subscriber::EnvFilter::try_new(value).map_err(|_| {
            crate::Error::Configuration("RUST_LOG must be a valid tracing filter".to_owned())
        })?,
        Err(std::env::VarError::NotPresent) => tracing_subscriber::EnvFilter::new("info"),
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(crate::Error::Configuration(
                "RUST_LOG must contain valid Unicode".to_owned(),
            ));
        }
    };

    let registry = tracing_subscriber::Registry::default()
        .with(env_filter)
        .with(fmt_layer);

    let Some(endpoint) = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .ok()
        .filter(|v| !v.is_empty())
    else {
        registry.init();
        return Ok(());
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
    Ok(())
}

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
