use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferLog {
    pub session_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
}
