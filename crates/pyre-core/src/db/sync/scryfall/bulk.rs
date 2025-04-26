use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BulkMetadataList {
    pub data: Vec<BulkMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BulkMetadata {
    pub id: String,
    #[serde(rename = "type")]
    pub bulk_type: String,
    pub updated_at: DateTime<Utc>,
    pub uri: String,
    pub name: String,
    pub description: String,
    pub size: u64,
    pub download_uri: String,
}

impl std::fmt::Display for BulkMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
