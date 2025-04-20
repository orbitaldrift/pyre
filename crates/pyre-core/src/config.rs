use std::path::PathBuf;

use garde::Validate;
use pyre_fs::DefaultPathProvider;
use serde::{Deserialize, Serialize};

use crate::{
    auth::provider,
    svc::{server::HttpConfig, state::DbConfig, SessionConfig},
};

#[derive(Validate, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[garde(dive)]
    pub server: ServerConfig,
    #[garde(dive)]
    pub http: HttpConfig,
    #[garde(dive)]
    pub session: SessionConfig,
    #[garde(dive)]
    pub db: DbConfig,
    #[garde(dive)]
    pub discord: provider::discord::Config,
    #[garde(dive)]
    pub telemetry: pyre_telemetry::config::Config,
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Validate, Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[garde(custom(|value: &String, (): &()| {
        if value.parse::<std::net::SocketAddr>().is_ok() {
            return Ok(())
        }
        Err(garde::Error::new("invalid socket address"))
    }))]
    pub addr: String,
    #[garde(skip)]
    pub cert: PathBuf,
    #[garde(skip)]
    pub key: PathBuf,
    #[garde(skip)]
    pub secret: PathBuf,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            addr: "127.0.0.1:4433".parse().unwrap(),
            cert: "etc/localhost/local.cert".into(),
            key: "etc/localhost/local.key".into(),
            secret: ".master.key".into(),
        }
    }
}

impl DefaultPathProvider for Config {
    const DEFAULT_FILENAME: &'static str = "config/pyre.toml";
}
