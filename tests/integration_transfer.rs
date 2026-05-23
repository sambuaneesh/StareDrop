use optigap_chunking::{chunker::FixedSizeChunker, reassembler::BasicReassembler};
use optigap_core::{Chunker, Reassembler};

#[test]
fn chunk_transfer_reconstructs_original_payload() {
    let payload = b"screen-to-camera data path smoke test";
    let chunker = FixedSizeChunker::new(7);
    let chunks = chunker.split(payload).expect("split");
    let mut reassembler = BasicReassembler::new();
    for chunk in chunks {
        reassembler.accept_chunk(chunk).expect("accept chunk");
    }
    assert!(reassembler.is_complete());
    let restored = reassembler.reconstruct().expect("reconstruct");
    assert_eq!(restored, payload);
}
