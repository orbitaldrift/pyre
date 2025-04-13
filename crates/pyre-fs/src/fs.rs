use std::path::PathBuf;

use async_trait::async_trait;
use tokio::io::AsyncReadExt;
use tracing::info;

#[async_trait]
pub trait FileReadExt {
    async fn read_all(self) -> Result<Vec<u8>, tokio::io::Error>;
}

#[async_trait]
impl FileReadExt for PathBuf {
    #[tracing::instrument]
    async fn read_all(self) -> Result<Vec<u8>, tokio::io::Error> {
        info!(path = %self.display(), "reading file");

        let mut buf = vec![];
        tokio::fs::File::open(self)
            .await?
            .read_to_end(&mut buf)
            .await?;
        Ok(buf)
    }
}
