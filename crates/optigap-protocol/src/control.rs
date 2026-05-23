use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ControlType {
    MissingChunks,
    Pause,
    Resume,
    SlowDown,
    SpeedUp,
    TransferComplete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlFrameV1 {
    pub magic: String,
    pub version: u8,
    pub frame_type: String,
    pub session_id: String,
    pub control_type: ControlType,
    pub missing_chunks: Vec<u32>,
}
