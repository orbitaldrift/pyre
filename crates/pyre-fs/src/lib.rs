use std::path::PathBuf;

pub mod fs;
pub mod toml;

/// Trait for types that provide a default configuration path
pub trait DefaultPathProvider: Sized {
    /// The default filename for this configuration type
    const DEFAULT_FILENAME: &'static str;

    /// Default path resolution for this configuration
    #[must_use]
    fn default_path() -> PathBuf {
        Self::DEFAULT_FILENAME.into()
    }
}
