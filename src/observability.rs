//! Server telemetry and request labels.

use axum::{Router, middleware};
use nine_dev_site_lib::telemetry::{RequestMetrics, Telemetry, observe};

use crate::{Error, Result};

pub const LIFECYCLE_TARGET: &str = "ixirsii_tech::lifecycle";

pub fn init() -> Result<Telemetry> {
    // Lower-level database errors may contain query data. Export only bounded
    // request events (the library default) and reviewed application targets.
    Telemetry::init(
        "ixirsii.tech",
        env!("CARGO_PKG_VERSION"),
        &[LIFECYCLE_TARGET, "ixirsii_tech::auth"],
    )
    .map_err(|error| Error::Configuration(error.to_string()))
}

pub fn with_requests(router: Router, requests: RequestMetrics) -> Router {
    router.layer(middleware::from_fn_with_state(
        requests.with_route_override(|matched, _| {
            matched
                .and_then(|url| crate::static_assets::STABLE.by_url(url))
                .map(|_| "static")
        }),
        observe,
    ))
}
