//! Server telemetry identity and exported application log targets.

use nine_dev_site_lib::telemetry::Telemetry;

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
