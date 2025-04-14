use pyre_fs::DefaultPathProvider;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub telemetry: pyre_telemetry::config::Config,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerConfig {
    pub cert: String,
    pub key: String,
    pub addr: String,
}

impl DefaultPathProvider for Config {
    const DEFAULT_FILENAME: &'static str = "config/pyre.toml";
}
