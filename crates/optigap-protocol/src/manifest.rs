use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestFrameV1 {
    pub magic: String,
    pub version: u8,
    pub frame_type: String,
    pub session_id: String,
    pub file_id: String,
    pub file_name: String,
    pub mime_type: Option<String>,
    pub original_file_size: u64,
    pub processed_file_size: u64,
    pub chunk_size: u32,
    pub total_chunks: u32,
    pub compression: String,
    pub encryption: String,
    pub original_sha256: String,
    pub processed_sha256: String,
}
