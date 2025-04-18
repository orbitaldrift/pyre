use sqlx::{pool::PoolConnection, Postgres};
use tower_sessions_redis_store::fred::{
    prelude::{Client, ClientLike, Pool},
    types::{config::Config as RedisConfig, ShutdownFlags},
};
use tracing::info;

use crate::config::DbConfig;

pub struct AppState {
    pub config: DbConfig,
    pub redis_pool: Pool,
    pub sql_pool: sqlx::Pool<sqlx::Postgres>,
}

impl AppState {
    pub async fn new(config: DbConfig) -> color_eyre::Result<Self> {
        info!("creating app state");

        let redis_cfg = RedisConfig::from_url(config.redis.as_str())?;

        let redis_pool = Pool::new(
            redis_cfg,
            None,
            None,
            None,
            std::thread::available_parallelism()?.into(),
        )?;

        redis_pool.connect_pool();
        redis_pool.wait_for_connect().await?;

        info!("connected to redis");

        let sql_pool = sqlx::Pool::<sqlx::Postgres>::connect(&config.pg).await?;

        Ok(Self {
            config,
            redis_pool,
            sql_pool,
        })
    }

    pub fn redis(&self) -> &Client {
        self.redis_pool.next()
    }

    pub async fn sql(&self) -> Result<PoolConnection<Postgres>, sqlx::Error> {
        self.sql_pool.acquire().await
    }

    pub async fn shutdown(&self) {
        let _ = self.redis_pool.shutdown(Some(ShutdownFlags::Save)).await;
        self.sql_pool.close().await;
    }
}
