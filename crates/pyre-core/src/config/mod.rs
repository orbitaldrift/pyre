use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub mod toml;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub telemetry: pyre_telemetry::config::Config,
}

impl DefaultPathProvider for Config {
    const DEFAULT_FILENAME: &'static str = "config/pyre.toml";
}

/// Trait for types that provide a default configuration path
pub trait DefaultPathProvider: Sized {
    /// The default filename for this configuration type
    const DEFAULT_FILENAME: &'static str;

    /// Default path resolution for this configuration
    fn default_path() -> PathBuf {
        Self::DEFAULT_FILENAME.into()
    }
}
