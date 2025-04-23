use std::{path::PathBuf, process::exit};

use config::Config;
use garde::Validate;
use pyre_build::build_info;
use pyre_fs::toml::FromToml;
use pyre_telemetry::{Info, Telemetry};
use tracing::error;
use uuid::Uuid;

mod auth;
mod config;
mod db;
mod error;
mod svc;

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> color_eyre::Result<()> {
    // Run Redis and Postgres in CI for tests using docker compose, check if it works

    // Graphql
    // Strong input validation around invariants
    // MTG card scheduled download, versioning
    // Cache layer for pg, card's data
    // WebTransport for any streaming
    // grafana tanka, ingress, grafana stack, terraform k8s cluster
    // k8s cluster pod resource monitoring
    // e2e tests, unit tests

    build_info!();
    color_eyre::install()?;

    let cfg;
    {
        let _g = Telemetry::stdout();

        (cfg, _) = Config::from_toml_path::<PathBuf>(None)
            .await
            .unwrap_or_else(|e| {
                error!(%e, "failed to load config");
                exit(1)
            });

        cfg.validate().unwrap_or_else(|e| {
            error!(%e, "failed to validate config");
            exit(1)
        });

        Telemetry::new(
            &cfg.telemetry,
            Info {
                id: Uuid::new_v4().into(),
                domain: "odrift".to_string(),
                meta: None,
            },
        )
        .init()
        .inspect_err(|e| {
            error!(%e, "failed to initialize telemetry");
            exit(1)
        })
        .expect("failed to initialize telemetry");
    }

    svc::start(cfg).await?;

    Ok(())
}
