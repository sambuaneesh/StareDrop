use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkAcceptResult {
    Accepted,
    Duplicate,
    RejectedCrc,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransferProgress {
    pub received_chunks: usize,
    pub total_chunks: usize,
    pub duplicate_chunks: usize,
    pub invalid_chunks: usize,
}

impl TransferProgress {
    pub fn ratio(&self) -> f32 {
        if self.total_chunks == 0 {
            return 0.0;
        }
        self.received_chunks as f32 / self.total_chunks as f32
    }
}
