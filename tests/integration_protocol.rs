use staredrop_protocol::frame_json::{JsonFrame, deserialize_frame, serialize_frame};

#[test]
fn phase1_json_frame_roundtrip() {
    let frame = JsonFrame::new_text_frame("integration-session", "hello world");
    let json = serialize_frame(&frame).expect("serialize");
    let parsed = deserialize_frame(&json).expect("deserialize");
    let payload = parsed.decode_payload().expect("payload");
    assert_eq!(payload, b"hello world");
}
