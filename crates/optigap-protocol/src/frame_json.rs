use anyhow::{Context, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonFrame {
    pub magic: String,
    pub version: u8,
    pub frame_type: String,
    pub session_id: String,
    pub file_id: String,
    pub payload_base64: String,
    pub crc32: u32,
}

impl JsonFrame {
    pub fn new_text_frame(session_id: &str, text: &str) -> Self {
        let payload = text.as_bytes();
        Self {
            magic: "OPTIGAP".to_string(),
            version: 1,
            frame_type: "TEXT".to_string(),
            session_id: session_id.to_string(),
            file_id: "phase1-text".to_string(),
            payload_base64: BASE64.encode(payload),
            crc32: crate::crc::crc32(payload),
        }
    }

    pub fn decode_payload(&self) -> Result<Vec<u8>> {
        let payload = BASE64
            .decode(self.payload_base64.as_bytes())
            .context("invalid base64 payload")?;
        let actual_crc = crate::crc::crc32(&payload);
        if actual_crc != self.crc32 {
            bail!("crc mismatch: expected {}, got {}", self.crc32, actual_crc);
        }
        Ok(payload)
    }
}

pub fn serialize_frame(frame: &JsonFrame) -> Result<String> {
    Ok(serde_json::to_string(frame)?)
}

pub fn deserialize_frame(json: &str) -> Result<JsonFrame> {
    let frame: JsonFrame = serde_json::from_str(json)?;
    if frame.magic != "OPTIGAP" {
        bail!("invalid magic");
    }
    if frame.version != 1 {
        bail!("unsupported frame version {}", frame.version);
    }
    Ok(frame)
}

#[cfg(test)]
mod tests {
    use super::{JsonFrame, deserialize_frame, serialize_frame};

    #[test]
    fn text_frame_round_trip() {
        let frame = JsonFrame::new_text_frame("s-1", "hello world");
        let raw = serialize_frame(&frame).expect("serialize");
        let parsed = deserialize_frame(&raw).expect("parse");
        let payload = parsed.decode_payload().expect("decode payload");
        assert_eq!(payload, b"hello world");
    }

    #[test]
    fn bad_json_is_rejected() {
        let parsed = deserialize_frame("{\"magic\":5}");
        assert!(parsed.is_err());
    }
}
