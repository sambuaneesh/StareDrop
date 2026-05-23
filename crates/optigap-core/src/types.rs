use serde::{Deserialize, Serialize};

use crate::{DecodedFrame, FramePayload, RenderedFrame, TransferProgress, error::Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub index: u32,
    pub total_chunks: u32,
    pub payload: Vec<u8>,
    pub crc32: u32,
}

#[derive(Debug, Clone, Default)]
pub struct SenderSessionState {
    pub next_index: usize,
    pub cycle_count: u64,
    pub total_frames: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedPayload {
    pub algorithm: String,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

pub trait VisualEncoder {
    fn encode_frame(&self, frame: &FramePayload) -> Result<RenderedFrame>;
}

pub trait VisualDecoder {
    fn decode_frame(&mut self, image: &CameraFrame) -> Result<Option<DecodedFrame>>;
}

pub trait Chunker {
    fn split(&self, data: &[u8]) -> Result<Vec<Chunk>>;
}

pub trait Reassembler {
    fn accept_chunk(&mut self, chunk: Chunk) -> Result<crate::ChunkAcceptResult>;
    fn progress(&self) -> TransferProgress;
    fn is_complete(&self) -> bool;
    fn reconstruct(&self) -> Result<Vec<u8>>;
}

pub trait ReliabilityStrategy {
    fn next_frames(&mut self, state: &SenderSessionState) -> Result<Vec<FramePayload>>;
    fn on_chunk_received(&mut self, chunk_index: u32);
    fn on_missing_chunks(&mut self, missing: &[u32]);
}

pub trait CryptoProvider {
    fn encrypt(&self, data: &[u8], password: &str) -> Result<EncryptedPayload>;
    fn decrypt(&self, payload: &EncryptedPayload, password: &str) -> Result<Vec<u8>>;
}

pub trait CompressionProvider {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>>;
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>>;
}
