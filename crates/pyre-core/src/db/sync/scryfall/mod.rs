use std::time::Duration;

use bulk::{BulkMetadata, BulkMetadataList};
use hyper::{header::ACCEPT, HeaderMap, StatusCode};
use serde::de::DeserializeOwned;
use sqlx::{Connection, PgConnection};
use tracing::{error, info};

use super::config;

pub mod bulk;
pub mod card;
pub mod db_card;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to get scryfall bulk data list: {0}")]
    Fetch(#[from] reqwest::Error),

    #[error("error from scryfall api: ({status}) {response}")]
    Scryfall {
        status: StatusCode,
        response: String,
    },
}

#[derive(Debug)]
pub struct ScryfallSync {
    pub cfg: config::Config,
    pub url: String,
    pub path: String,
    pub shutdown: tokio::sync::broadcast::Receiver<()>,
}

impl ScryfallSync {
    pub fn new(
        cfg: config::Config,
        url: String,
        path: String,
        shutdown: tokio::sync::broadcast::Receiver<()>,
    ) -> Self {
        Self {
            cfg,
            url,
            path,
            shutdown,
        }
    }

    pub async fn start(&mut self) -> color_eyre::Result<()> {
        let url = format!("{}/{}", self.url, self.path);
        let user_agent: String =
            format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

        let _sql = PgConnection::connect(&self.cfg.db.pg).await?;

        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, "application/json".parse().unwrap());

        let client = reqwest::Client::builder()
            .http2_prior_knowledge()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(3))
            .user_agent(user_agent)
            .default_headers(headers)
            .build()
            .unwrap();

        let bulk = Self::get_bulk(&client, &url).await?;
        info!(%bulk, "downloaded bulk metadata");

        // Filter MTGO only cards (with games[] with mtgo only)
        // Populate symbology table using /symbology
        // Card faces should always be present even with a single card face (card_faces null or empty)
        // Sometimes card faces is present, but the images are in the original card object, so move them over

        // loop {
        //     tokio::select! {
        //         _ = self.shutdown.recv() => {
        //             info!("scryfall sync shutting down");
        //             break;
        //         }
        //         () = tokio::time::sleep(tokio::time::Duration::from_secs(self.cfg.sync.freq)) => {
        //             info!("scryfall sync running");

        //         }
        //     }
        // }

        Ok(())
    }

    async fn get_bulk(client: &reqwest::Client, url: &str) -> color_eyre::Result<BulkMetadata> {
        let bulk_data = Self::download::<BulkMetadataList>(client, url).await?;
        Ok(bulk_data
            .data
            .into_iter()
            .find(|b| b.bulk_type == "default_cards")
            .expect("default_cards bulk data not found"))
    }

    async fn download<T>(client: &reqwest::Client, url: &str) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let res = client.get(url).send().await?;

        if res.status() != 200 {
            return Err(Error::Scryfall {
                status: res.status(),
                response: res.text().await?,
            });
        }

        Ok(res.json::<T>().await?)
    }
}
