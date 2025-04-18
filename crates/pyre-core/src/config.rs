use std::{net::SocketAddr, path::PathBuf};

use axum::http::HeaderValue;
use pyre_fs::DefaultPathProvider;
use serde::{Deserialize, Serialize};
use tower_http::cors::AllowOrigin;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Origins(pub Vec<String>);

impl From<Origins> for AllowOrigin {
    fn from(origins: Origins) -> Self {
        let origins = origins
            .0
            .into_iter()
            .map(|origin| {
                HeaderValue::from_str(&origin)
                    .unwrap_or_else(|_| panic!("Invalid origin header value: {origin}"))
            })
            .collect::<Vec<_>>();

        AllowOrigin::list(origins)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub http: HttpConfig,
    pub session: SessionConfig,
    pub db: DbConfig,
    pub telemetry: pyre_telemetry::config::Config,
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Config {{ server: {:?}, telemetry: {:?} }}",
            self.server, self.telemetry
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub addr: SocketAddr,
    pub cert: PathBuf,
    pub key: PathBuf,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            addr: "127.0.0.1:4433".parse().unwrap(),
            cert: "etc/localhost/local.cert".into(),
            key: "etc/localhost/local.key".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    pub timeout: u64,
    pub max_conns: usize,
    pub max_body: usize,
    pub origins: Origins,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            timeout: 10,
            max_conns: 512,
            max_body: 4096,
            origins: Origins(vec!["*".to_string()]),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub session_days: i64,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self { session_days: 7 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbConfig {
    pub redis: String,
    pub pg: String,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            redis: "redis://localhost:6379".to_string(),
            pg: "postgres://postgres:postgres@localhost:5432/pyre".to_string(),
        }
    }
}

impl DefaultPathProvider for Config {
    const DEFAULT_FILENAME: &'static str = "config/pyre.toml";
}
