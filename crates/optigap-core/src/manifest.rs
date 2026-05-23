use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferManifest {
    pub magic: String,
    pub version: u8,
    pub session_id: String,
    pub file_id: String,
    pub file_name: String,
    pub mime_type: Option<String>,
    pub original_file_size: u64,
    pub processed_file_size: u64,
    pub compression: String,
    pub encryption: String,
    pub chunk_size: u32,
    pub total_chunks: u32,
    pub original_sha256: String,
    pub processed_sha256: String,
    pub created_at: DateTime<Utc>,
}
