use std::{path::PathBuf, process::exit, time::Duration};

use config::{toml::FromToml, Config};
use pyre_telemetry::{Info, Telemetry};
use tracing::{error, info};
use uuid::Uuid;

mod config;

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    {
        let _g = Telemetry::stdout();

        let (cfg, _) = Config::from_toml_path::<PathBuf>(None)
            .await
            .unwrap_or_else(|e| {
                error!("failed to load config: {e}");
                exit(1)
            });

        Telemetry::new(
            cfg.telemetry,
            Info {
                id: Uuid::new_v4().into(),
                domain: "odrift".to_string(),
                meta: None,
            },
        )
        .init()
        .inspect_err(|e| {
            error!("failed to initialize telemetry: {e}");
            exit(1)
        })
        .expect("failed to initialize telemetry");
    }

    compute(5, 10);

    tokio::time::sleep(Duration::from_secs(20)).await;
    Ok(())
}

#[tracing::instrument]
fn compute(a: i32, b: i32) -> i32 {
    info!(counter.computer = 1, "Computing");

    a + b
}
