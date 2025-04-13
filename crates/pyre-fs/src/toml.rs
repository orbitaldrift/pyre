use std::{
    future::Future,
    path::{Path, PathBuf},
};

use serde::de::DeserializeOwned;
use tracing::{error, info};

use super::DefaultPathProvider;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("file not found: {0}")]
    FileNotFound(PathBuf),

    #[error("file does not contain valid utf8")]
    InvalidFileUtf8,

    #[error("failed to parse toml: {0}")]
    Parse(#[from] toml::de::Error),
}

pub trait FromToml: Sized {
    /// Reads a TOML file from the specified path, or the default path if none is provided.
    /// Returns the parsed configuration and the path used.
    #[tracing::instrument]
    fn from_toml_path<P>(path: Option<P>) -> impl Future<Output = Result<(Self, P), Error>> + Send
    where
        P: std::fmt::Debug + AsRef<Path> + From<PathBuf> + Send,
        Self: DeserializeOwned + DefaultPathProvider,
    {
        async {
            let path = match path {
                Some(path) => path,
                None => P::from(Self::default_path()),
            };

            info!(file = %path.as_ref().display(), "reading toml");

            let file_contents = tokio::fs::read(path.as_ref())
                .await
                .map_err(|_| Error::FileNotFound(path.as_ref().into()))?;

            let result = toml::from_str(
                &String::from_utf8(file_contents).map_err(|_| Error::InvalidFileUtf8)?,
            )?;
            Ok((result, path))
        }
    }
}

impl<T> FromToml for T where T: DeserializeOwned + DefaultPathProvider {}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Debug, Serialize, Deserialize)]
    struct TestConfig {
        foo: String,
        bar: i32,
    }

    impl DefaultPathProvider for TestConfig {
        const DEFAULT_FILENAME: &'static str = "test/config.toml";
    }

    #[tokio::test]
    async fn test_from_toml_path() {
        let config = TestConfig::from_toml_path::<PathBuf>(None).await;
        assert!(config.is_ok());
        assert!(!config.unwrap().0.foo.is_empty());
    }
}
