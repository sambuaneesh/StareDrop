use anyhow::{Context, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::manifest::ManifestFrameV1;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFrameV1 {
    pub magic: String,
    pub version: u8,
    pub frame_type: String,
    pub session_id: String,
    pub file_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub chunk_index: u32,
    pub total_chunks: u32,
    pub payload_base64: String,
    pub crc32: u32,
}

impl DataFrameV1 {
    pub fn payload_bytes(&self) -> Result<Vec<u8>> {
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

#[derive(Debug, Clone)]
pub enum ParsedFrameV1 {
    Text(JsonFrame),
    Manifest(ManifestFrameV1),
    Data(DataFrameV1),
}

impl JsonFrame {
    pub fn new_text_frame(session_id: &str, text: &str) -> Self {
        let payload = text.as_bytes();
        Self {
            magic: "STAREDROP".to_string(),
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
    if frame.magic != "STAREDROP" {
        bail!("invalid magic");
    }
    if frame.version != 1 {
        bail!("unsupported frame version {}", frame.version);
    }
    Ok(frame)
}

pub fn serialize_manifest_frame(frame: &ManifestFrameV1) -> Result<String> {
    Ok(serde_json::to_string(frame)?)
}

pub fn serialize_data_frame(frame: &DataFrameV1) -> Result<String> {
    Ok(serde_json::to_string(frame)?)
}

pub fn parse_frame_v1(raw: &str) -> Result<ParsedFrameV1> {
    let value: Value = serde_json::from_str(raw).context("invalid json frame")?;
    let magic = value
        .get("magic")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing frame magic"))?;
    if magic != "STAREDROP" {
        bail!("invalid magic");
    }
    let version = value
        .get("version")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("missing frame version"))?;
    if version != 1 {
        bail!("unsupported frame version {}", version);
    }
    let frame_type = value
        .get("frame_type")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing frame_type"))?;

    match frame_type {
        "TEXT" => Ok(ParsedFrameV1::Text(serde_json::from_value(value)?)),
        "MANIFEST" => Ok(ParsedFrameV1::Manifest(serde_json::from_value(value)?)),
        "DATA" => Ok(ParsedFrameV1::Data(serde_json::from_value(value)?)),
        other => bail!("unknown frame_type {}", other),
    }
}

#[cfg(test)]
mod tests {
    use crate::manifest::ManifestFrameV1;
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

    use super::{
        DataFrameV1, JsonFrame, ParsedFrameV1, deserialize_frame, parse_frame_v1,
        serialize_data_frame, serialize_frame,
    };

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

    #[test]
    fn data_frame_round_trip() {
        let payload = b"abc123";
        let frame = DataFrameV1 {
            magic: "STAREDROP".to_string(),
            version: 1,
            frame_type: "DATA".to_string(),
            session_id: "s".to_string(),
            file_id: "f".to_string(),
            file_name: "x".to_string(),
            file_size: 6,
            chunk_index: 0,
            total_chunks: 1,
            payload_base64: BASE64.encode(payload),
            crc32: crate::crc::crc32(payload),
        };
        let raw = serialize_data_frame(&frame).expect("serialize");
        let parsed = parse_frame_v1(&raw).expect("parse");
        match parsed {
            ParsedFrameV1::Data(data) => {
                assert_eq!(data.payload_bytes().expect("payload"), payload);
            }
            _ => panic!("expected data frame"),
        }
    }

    #[test]
    fn manifest_parse() {
        let manifest = ManifestFrameV1 {
            magic: "STAREDROP".to_string(),
            version: 1,
            frame_type: "MANIFEST".to_string(),
            session_id: "s".to_string(),
            file_id: "f".to_string(),
            file_name: "a.txt".to_string(),
            mime_type: Some("text/plain".to_string()),
            original_file_size: 10,
            processed_file_size: 10,
            chunk_size: 5,
            total_chunks: 2,
            compression: "none".to_string(),
            encryption: "none".to_string(),
            original_sha256: "aa".to_string(),
            processed_sha256: "aa".to_string(),
        };
        let raw = serde_json::to_string(&manifest).expect("serialize");
        let parsed = parse_frame_v1(&raw).expect("parse");
        match parsed {
            ParsedFrameV1::Manifest(m) => assert_eq!(m.file_name, "a.txt"),
            _ => panic!("expected manifest frame"),
        }
    }
}
