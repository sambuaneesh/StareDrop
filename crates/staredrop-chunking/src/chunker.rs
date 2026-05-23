use staredrop_core::{Chunk, Chunker, StareDropError, Result};

pub struct FixedSizeChunker {
    pub chunk_size: usize,
}

impl FixedSizeChunker {
    pub fn new(chunk_size: usize) -> Self {
        Self { chunk_size }
    }
}

impl Chunker for FixedSizeChunker {
    fn split(&self, data: &[u8]) -> Result<Vec<Chunk>> {
        if self.chunk_size == 0 {
            return Err(StareDropError::InvalidData(
                "chunk size must be > 0".to_string(),
            ));
        }

        let total_chunks = data.len().div_ceil(self.chunk_size) as u32;
        let mut chunks = Vec::with_capacity(total_chunks as usize);
        for (index, payload) in data.chunks(self.chunk_size).enumerate() {
            let mut hasher = crc32fast::Hasher::new();
            hasher.update(payload);
            let crc32 = hasher.finalize();
            chunks.push(Chunk {
                index: index as u32,
                total_chunks,
                payload: payload.to_vec(),
                crc32,
            });
        }
        Ok(chunks)
    }
}

#[cfg(test)]
mod tests {
    use super::FixedSizeChunker;
    use staredrop_core::Chunker;

    #[test]
    fn split_exact_multiple() {
        let chunker = FixedSizeChunker::new(4);
        let chunks = chunker.split(b"ABCDEFGH").expect("split");
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].payload, b"ABCD");
        assert_eq!(chunks[1].payload, b"EFGH");
    }
}
