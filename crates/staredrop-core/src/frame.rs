use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FrameType {
    Manifest,
    Data,
    Control,
    End,
    Text,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FramePayload {
    pub magic: String,
    pub version: u8,
    pub frame_type: FrameType,
    pub session_id: String,
    pub file_id: String,
    pub payload_bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct RenderedFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedFrame {
    pub frame_type: FrameType,
    pub session_id: String,
    pub payload_utf8: String,
}
