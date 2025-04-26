use garde::Validate;
use scryfall::ScryfallSync;
use serde::{Deserialize, Serialize};
use tracing::info;

pub mod config;
pub mod scryfall;

#[derive(Validate, Debug, strum::Display, Clone, Serialize, Deserialize)]
pub enum Api {
    Scryfall {
        #[garde(url)]
        url: String,
        #[garde(ascii, length(min = 1))]
        path: String,
    },
}

impl Default for Api {
    fn default() -> Self {
        Self::Scryfall {
            url: "https://api.scryfall.com".to_string(),
            path: "bulk-data".to_string(),
        }
    }
}

pub async fn start(
    cfg: config::Config,
    shutdown: tokio::sync::broadcast::Receiver<()>,
) -> color_eyre::Result<()> {
    info!(cfg = %cfg, "creating dbsync");

    match cfg.sync.api.clone() {
        Api::Scryfall { url, path } => ScryfallSync::new(cfg, url, path, shutdown).start().await,
    }
}

#[cfg(test)]
mod tests {
    use pyre_cli::shutdown::Shutdown;
    use pyre_telemetry::Telemetry;

    use crate::db::sync::{config::Config, start};

    #[tokio::test]
    async fn test_scryfall_sync() {
        let _t = Telemetry::default().init_scoped();
        let shutdown = Shutdown::new_with_all_signals().install();

        let cfg = Config::default();
        start(cfg, shutdown.subscribe()).await.unwrap();
    }
}
