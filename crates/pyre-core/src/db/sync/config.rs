use garde::Validate;
use pyre_fs::DefaultPathProvider;
use serde::{Deserialize, Serialize};

use super::Api;
use crate::{config::HasTelemetry, svc::state::DbConfig};

#[derive(Validate, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[garde(dive)]
    pub telemetry: pyre_telemetry::config::Config,
    #[garde(dive)]
    pub db: DbConfig,
    #[garde(dive)]
    pub sync: SyncConfig,
}

impl DefaultPathProvider for Config {
    const DEFAULT_FILENAME: &'static str = "config/pyre-dbsync.toml";
}

impl HasTelemetry for Config {
    fn telemetry(&self) -> &pyre_telemetry::config::Config {
        &self.telemetry
    }
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Validate, Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncConfig {
    #[garde(dive)]
    pub api: Api,

    #[garde(range(min = 30, max = 360))]
    pub freq: u64,
}
