use std::{path::PathBuf, process::exit};

use clap::Parser;
use config::HasTelemetry;
use db::sync;
use garde::Validate;
use opentelemetry::KeyValue;
use pyre_build::build_info;
use pyre_cli::shutdown::Shutdown;
use pyre_fs::{toml::FromToml, DefaultPathProvider};
use pyre_telemetry::{Info, Telemetry};
use serde::de::DeserializeOwned;
use tracing::error;
use uuid::Uuid;

mod auth;
mod config;
mod db;
mod error;
mod svc;

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[derive(Debug, Default, Clone, Copy, strum::Display, clap::ValueEnum)]
enum CliMode {
    #[default]
    Server,
    DbSync,
}

/// Pyre CLI: A command line interface for Pyre
/// There are two modes: server and dbsync
/// The server mode starts the Pyre server
/// The dbsync mode syncs the database with scryfall
#[derive(Parser, Debug, Default)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Application mode
    #[arg(short, long, default_value_t = CliMode::Server)]
    mode: CliMode,

    /// Path to the config file
    #[arg(short, long)]
    config: Option<PathBuf>,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> color_eyre::Result<()> {
    // Turnstile support on /redirect for auth

    // Download and index in SQL scryfall database only in english
    // Graphql + Moka for in memory cache for GraphQL read only queries
    // Strong input validation around invariants
    // MTG card scheduled download and upload to SQL, versioning
    // WebTransport for streaming
    // grafana tanka, ingress, grafana stack, terraform k8s cluster
    // k8s cluster pod resource monitoring, self hosted grafana stack
    build_info!();
    color_eyre::install()?;

    let cli = Cli::try_parse().unwrap_or_default();

    let shutdown = Shutdown::new_with_all_signals().install();

    match cli.mode {
        CliMode::Server => {
            let cfg = load_config::<config::Config>(cli).await?;
            svc::start(cfg, shutdown.subscribe()).await?;
        }
        CliMode::DbSync => {
            let cfg = load_config::<sync::config::Config>(cli).await?;
            sync::start(cfg, shutdown.subscribe()).await?;
        }
    }

    Ok(())
}

async fn load_config<T>(cli: Cli) -> color_eyre::Result<T>
where
    T: HasTelemetry + DefaultPathProvider + DeserializeOwned + FromToml + Validate,
    T::Context: Default,
{
    let cfg;
    {
        let _g = Telemetry::stdout();

        (cfg, _) = T::from_toml_path(cli.config).await.unwrap_or_else(|e| {
            error!(%e, "failed to load config");
            exit(1)
        });

        cfg.validate().unwrap_or_else(|e| {
            error!(%e, "failed to validate config");
            exit(1)
        });

        Telemetry::new(
            cfg.telemetry(),
            Info {
                id: Uuid::new_v4().into(),
                domain: "odrift".to_string(),
                meta: Some(vec![KeyValue::new(
                    "mode".to_string(),
                    cli.mode.to_string(),
                )]),
            },
        )
        .init()
        .inspect_err(|e| {
            error!(%e, "failed to initialize telemetry");
            exit(1)
        })
        .expect("failed to initialize telemetry");
    }

    Ok(cfg)
}
