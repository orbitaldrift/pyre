use std::{collections::HashMap, sync::Arc, time::Duration};

use garde::Validate;
use secstr::SecStr;
use serde::{Deserialize, Serialize};
use tower_sessions_redis_store::fred::{
    prelude::{ClientLike, Pool},
    types::{config::Config as RedisConfig, ShutdownFlags},
};
use tracing::info;

use crate::{
    auth::{
        self,
        provider::{ConfiguredClient, ProviderKind},
    },
    config::Config,
};

#[derive(Validate, Debug, Clone, Serialize, Deserialize)]
pub struct DbConfig {
    #[garde(prefix("redis://"))]
    pub redis: String,
    #[garde(prefix("postgres://"))]
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

/// Application state
/// Very cheap to clone.
#[derive(Debug, Clone)]
pub struct AppState {
    pub config: Config,

    pub secret: SecStr,

    pub redis_pool: Pool,
    pub sql_pool: sqlx::Pool<sqlx::Postgres>,

    pub oauth2_clients: HashMap<ProviderKind, Arc<ConfiguredClient>>,
    pub http_client: reqwest::Client,
}

impl AppState {
    /// Creates a new application state
    ///
    /// # Panics
    /// If the application secret cannot be read from the file.
    pub async fn new(config: Config) -> color_eyre::Result<Self> {
        info!(%config, "initializing app state");

        let redis_cfg = RedisConfig::from_url(config.db.redis.as_str())?;

        let redis_pool = Pool::new(
            redis_cfg,
            None,
            None,
            None,
            std::thread::available_parallelism()?.into(),
        )?;

        redis_pool.connect_pool();
        redis_pool.wait_for_connect().await?;

        let sql_pool = sqlx::Pool::<sqlx::Postgres>::connect(&config.db.pg).await?;

        let mut oauth2_clients = HashMap::new();
        oauth2_clients.insert(
            ProviderKind::Discord,
            Arc::new(auth::provider::discord::new_client(config.discord.clone()).await),
        );

        let http_client = reqwest::Client::builder()
            .zstd(true)
            .connect_timeout(Duration::from_secs(config.http.timeout))
            .timeout(Duration::from_secs(config.http.timeout))
            .http2_prior_knowledge()
            .build()?;

        let secret = tokio::fs::read(config.server.secret.clone())
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "failed to read secret from {}",
                    config.server.secret.display()
                )
            });

        assert_eq!(secret.len(), 64, "secret must be 64 bytes");

        let secret = SecStr::from(secret);

        Ok(Self {
            config,
            secret,
            redis_pool,
            sql_pool,
            oauth2_clients,
            http_client,
        })
    }

    pub fn get_oauth_client(&self, kind: ProviderKind) -> Result<Arc<ConfiguredClient>, String> {
        self.oauth2_clients
            .get(&kind)
            .ok_or(format!("client {kind:?} not found"))
            .cloned()
    }

    pub async fn shutdown(&self) {
        let _ = self.redis_pool.shutdown(Some(ShutdownFlags::Save)).await;
        self.sql_pool.close().await;
    }
}
