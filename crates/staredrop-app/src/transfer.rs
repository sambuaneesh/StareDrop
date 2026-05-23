use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use staredrop_chunking::{chunker::FixedSizeChunker, reassembler::BasicReassembler};
use staredrop_core::{ChunkAcceptResult, Chunker, Reassembler};
use staredrop_crypto::hash::sha256_hex;
use staredrop_protocol::{
    frame_json::{
        DataFrameV1, JsonFrame, ParsedFrameV1, parse_frame_v1, serialize_data_frame,
        serialize_frame, serialize_manifest_frame,
    },
    manifest::ManifestFrameV1,
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum SenderPlan {
    Text {
        frame_text: String,
    },
    File {
        manifest: ManifestFrameV1,
        frames: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub struct SenderBuildOptions {
    pub chunk_size: usize,
}

pub fn build_text_sender_plan(payload_text: &str) -> Result<SenderPlan> {
    let frame = JsonFrame::new_text_frame(&Uuid::new_v4().to_string(), payload_text);
    let frame_text = serialize_frame(&frame)?;
    Ok(SenderPlan::Text { frame_text })
}

pub fn build_file_sender_plan(path: &Path, options: SenderBuildOptions) -> Result<SenderPlan> {
    if options.chunk_size == 0 {
        bail!("chunk-size must be > 0");
    }

    let file_bytes =
        fs::read(path).with_context(|| format!("failed to read input file {}", path.display()))?;
    let chunker = FixedSizeChunker::new(options.chunk_size);
    let chunks = chunker.split(&file_bytes)?;

    let session_id = Uuid::new_v4().to_string();
    let file_id = Uuid::new_v4().to_string();
    let file_name = sanitize_file_name(
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("payload.bin"),
    );
    let file_size = file_bytes.len() as u64;
    let original_sha256 = sha256_hex(&file_bytes);
    let processed_sha256 = original_sha256.clone();

    let manifest = ManifestFrameV1 {
        magic: "STAREDROP".to_string(),
        version: 1,
        frame_type: "MANIFEST".to_string(),
        session_id: session_id.clone(),
        file_id: file_id.clone(),
        file_name: file_name.clone(),
        mime_type: None,
        original_file_size: file_size,
        processed_file_size: file_size,
        chunk_size: options.chunk_size as u32,
        total_chunks: chunks.len() as u32,
        compression: "none".to_string(),
        encryption: "none".to_string(),
        original_sha256,
        processed_sha256,
    };

    let mut frames = Vec::with_capacity(chunks.len() + 1);
    frames.push(serialize_manifest_frame(&manifest)?);
    for chunk in chunks {
        let frame = DataFrameV1 {
            magic: "STAREDROP".to_string(),
            version: 1,
            frame_type: "DATA".to_string(),
            session_id: session_id.clone(),
            file_id: file_id.clone(),
            file_name: file_name.clone(),
            file_size,
            chunk_index: chunk.index,
            total_chunks: chunk.total_chunks,
            payload_base64: BASE64.encode(&chunk.payload),
            crc32: chunk.crc32,
        };
        frames.push(serialize_data_frame(&frame)?);
    }

    Ok(SenderPlan::File { manifest, frames })
}

pub struct ReceiverSession {
    manifest: Option<ManifestFrameV1>,
    reassembler: BasicReassembler,
    session_id: Option<String>,
    file_id: Option<String>,
    pub duplicate_chunks: u64,
    pub invalid_chunks: u64,
    pub accepted_chunks: u64,
    pub completed_path: Option<PathBuf>,
    pub status: String,
}

impl ReceiverSession {
    pub fn new() -> Self {
        Self {
            manifest: None,
            reassembler: BasicReassembler::new(),
            session_id: None,
            file_id: None,
            duplicate_chunks: 0,
            invalid_chunks: 0,
            accepted_chunks: 0,
            completed_path: None,
            status: "Waiting for manifest/data frames".to_string(),
        }
    }

    pub fn progress(&self) -> staredrop_core::TransferProgress {
        self.reassembler.progress()
    }

    pub fn on_decoded_text(
        &mut self,
        text: &str,
        output_spec: &OutputSpec,
        auto_save: bool,
    ) -> Result<Option<PathBuf>> {
        let parsed = match parse_frame_v1(text) {
            Ok(p) => p,
            Err(_) => {
                self.status = "Decoded non-protocol QR text".to_string();
                return Ok(None);
            }
        };

        match parsed {
            ParsedFrameV1::Text(frame) => {
                let bytes = frame.decode_payload()?;
                self.status = format!("TEXT frame: {}", String::from_utf8_lossy(&bytes));
                Ok(None)
            }
            ParsedFrameV1::Manifest(manifest) => {
                self.session_id = Some(manifest.session_id.clone());
                self.file_id = Some(manifest.file_id.clone());
                self.manifest = Some(manifest);
                self.status = "Manifest received".to_string();
                Ok(None)
            }
            ParsedFrameV1::Data(data) => {
                if let Some(expected_session) = &self.session_id
                    && &data.session_id != expected_session
                {
                    self.status = "Ignoring DATA frame from different session".to_string();
                    return Ok(None);
                }
                let payload = data.payload_bytes()?;
                let chunk = staredrop_core::Chunk {
                    index: data.chunk_index,
                    total_chunks: data.total_chunks,
                    payload,
                    crc32: data.crc32,
                };
                match self.reassembler.accept_chunk(chunk)? {
                    ChunkAcceptResult::Accepted => {
                        self.accepted_chunks += 1;
                    }
                    ChunkAcceptResult::Duplicate => {
                        self.duplicate_chunks += 1;
                    }
                    ChunkAcceptResult::RejectedCrc => {
                        self.invalid_chunks += 1;
                    }
                }
                let progress = self.reassembler.progress();
                self.status = format!(
                    "DATA frame: {}/{} chunks",
                    progress.received_chunks, progress.total_chunks
                );

                if auto_save && self.reassembler.is_complete() {
                    let path = self.try_finalize(output_spec)?;
                    return Ok(Some(path));
                }
                Ok(None)
            }
        }
    }

    pub fn try_finalize(&mut self, output_spec: &OutputSpec) -> Result<PathBuf> {
        if self.completed_path.is_some() {
            return Ok(self.completed_path.clone().expect("checked is_some"));
        }

        if !self.reassembler.is_complete() {
            bail!("cannot finalize before all chunks are received");
        }
        let manifest = self
            .manifest
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("manifest not received"))?;
        let bytes = self.reassembler.reconstruct()?;

        let actual_sha = sha256_hex(&bytes);
        if actual_sha != manifest.processed_sha256 {
            bail!(
                "sha256 mismatch: expected {}, got {}",
                manifest.processed_sha256,
                actual_sha
            );
        }

        let out_path = output_spec.resolve_path(&manifest.file_name)?;
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed creating parent directory {}", parent.display())
            })?;
        }
        fs::write(&out_path, &bytes)
            .with_context(|| format!("failed writing output file {}", out_path.display()))?;
        self.completed_path = Some(out_path.clone());
        self.status = format!("Saved {}", out_path.display());
        Ok(out_path)
    }
}

#[derive(Debug, Clone)]
pub enum OutputSpec {
    ExactFile(PathBuf),
    Directory(PathBuf),
}

impl OutputSpec {
    pub fn resolve_path(&self, file_name: &str) -> Result<PathBuf> {
        match self {
            OutputSpec::ExactFile(path) => {
                if path.exists() {
                    bail!("output file already exists: {}", path.display());
                }
                Ok(path.clone())
            }
            OutputSpec::Directory(dir) => {
                let file_name = sanitize_file_name(file_name);
                let mut candidate = dir.join(&file_name);
                if !candidate.exists() {
                    return Ok(candidate);
                }

                for idx in 1..10_000_u32 {
                    let next = append_numeric_suffix(dir, &file_name, idx);
                    if !next.exists() {
                        candidate = next;
                        break;
                    }
                }
                Ok(candidate)
            }
        }
    }
}

fn sanitize_file_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "output.bin".to_string()
    } else {
        out
    }
}

fn append_numeric_suffix(dir: &Path, base_name: &str, idx: u32) -> PathBuf {
    if let Some((stem, ext)) = base_name.rsplit_once('.') {
        if !stem.is_empty() && !ext.is_empty() {
            return dir.join(format!("{stem}.{idx}.{ext}"));
        }
    }
    dir.join(format!("{base_name}.{idx}"))
}

#[cfg(test)]
mod tests {
    use super::{
        OutputSpec, ReceiverSession, SenderBuildOptions, SenderPlan, build_file_sender_plan,
        build_text_sender_plan,
    };
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn text_sender_plan_builds() {
        let plan = build_text_sender_plan("hello").expect("build");
        match plan {
            SenderPlan::Text { frame_text } => {
                assert!(frame_text.contains("\"frame_type\":\"TEXT\""))
            }
            _ => panic!("expected text plan"),
        }
    }

    #[test]
    fn file_sender_and_receiver_round_trip() {
        let tmp = tempdir().expect("tmp");
        let input_path = tmp.path().join("input.bin");
        let input_bytes = b"phase2-test-payload-1234567890";
        fs::write(&input_path, input_bytes).expect("write input");

        let plan = build_file_sender_plan(&input_path, SenderBuildOptions { chunk_size: 8 })
            .expect("build file plan");
        let SenderPlan::File { frames, .. } = plan else {
            panic!("expected file plan");
        };

        let out_dir = tmp.path().join("out");
        let output_spec = OutputSpec::Directory(out_dir.clone());
        let mut rx = ReceiverSession::new();
        let mut saved = None;
        for frame in frames {
            if let Some(path) = rx
                .on_decoded_text(&frame, &output_spec, true)
                .expect("accept frame")
            {
                saved = Some(path);
                break;
            }
        }
        let out_path = saved.expect("saved output");
        let output_bytes = fs::read(out_path).expect("read output");
        assert_eq!(output_bytes, input_bytes);
    }
}
