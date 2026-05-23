use std::collections::BTreeMap;

use optigap_core::{Chunk, ChunkAcceptResult, OptiGapError, Reassembler, Result, TransferProgress};

#[derive(Default)]
pub struct BasicReassembler {
    total_chunks: Option<u32>,
    chunks: BTreeMap<u32, Vec<u8>>,
    duplicate_chunks: usize,
    invalid_chunks: usize,
}

impl BasicReassembler {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Reassembler for BasicReassembler {
    fn accept_chunk(&mut self, chunk: Chunk) -> Result<ChunkAcceptResult> {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&chunk.payload);
        if hasher.finalize() != chunk.crc32 {
            self.invalid_chunks += 1;
            return Ok(ChunkAcceptResult::RejectedCrc);
        }

        match self.total_chunks {
            None => self.total_chunks = Some(chunk.total_chunks),
            Some(existing) if existing != chunk.total_chunks => {
                return Err(OptiGapError::InvalidData(format!(
                    "total chunk mismatch: expected {}, got {}",
                    existing, chunk.total_chunks
                )));
            }
            Some(_) => {}
        }

        if self.chunks.contains_key(&chunk.index) {
            self.duplicate_chunks += 1;
            return Ok(ChunkAcceptResult::Duplicate);
        }

        self.chunks.insert(chunk.index, chunk.payload);
        Ok(ChunkAcceptResult::Accepted)
    }

    fn progress(&self) -> TransferProgress {
        TransferProgress {
            received_chunks: self.chunks.len(),
            total_chunks: self.total_chunks.unwrap_or(0) as usize,
            duplicate_chunks: self.duplicate_chunks,
            invalid_chunks: self.invalid_chunks,
        }
    }

    fn is_complete(&self) -> bool {
        self.total_chunks
            .is_some_and(|total| self.chunks.len() == total as usize)
    }

    fn reconstruct(&self) -> Result<Vec<u8>> {
        if !self.is_complete() {
            return Err(OptiGapError::Failed("transfer not complete".to_string()));
        }
        let total = self.total_chunks.unwrap_or(0);
        let mut out = Vec::new();
        for idx in 0..total {
            let payload = self
                .chunks
                .get(&idx)
                .ok_or_else(|| OptiGapError::NotFound(format!("missing chunk {}", idx)))?;
            out.extend_from_slice(payload);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::BasicReassembler;
    use crate::chunker::FixedSizeChunker;
    use optigap_core::{Chunker, Reassembler};

    #[test]
    fn round_trip_reassembly() {
        let input = b"chunk test payload";
        let chunker = FixedSizeChunker::new(5);
        let chunks = chunker.split(input).expect("split");

        let mut reassembler = BasicReassembler::new();
        for chunk in chunks {
            reassembler.accept_chunk(chunk).expect("accept");
        }
        assert!(reassembler.is_complete());
        let out = reassembler.reconstruct().expect("reconstruct");
        assert_eq!(out, input);
    }
}
