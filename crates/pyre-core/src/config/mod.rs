use pyre_fs::DefaultPathProvider;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub telemetry: pyre_telemetry::config::Config,
}

impl DefaultPathProvider for Config {
    const DEFAULT_FILENAME: &'static str = "config/pyre.toml";
}
