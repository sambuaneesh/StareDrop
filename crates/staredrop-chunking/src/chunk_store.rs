use std::collections::BTreeSet;

#[derive(Debug, Default)]
pub struct ChunkStore {
    received: BTreeSet<u32>,
    total_chunks: u32,
}

impl ChunkStore {
    pub fn new(total_chunks: u32) -> Self {
        Self {
            received: BTreeSet::new(),
            total_chunks,
        }
    }

    pub fn insert(&mut self, index: u32) -> bool {
        self.received.insert(index)
    }

    pub fn missing(&self) -> Vec<u32> {
        (0..self.total_chunks)
            .filter(|idx| !self.received.contains(idx))
            .collect()
    }
}
