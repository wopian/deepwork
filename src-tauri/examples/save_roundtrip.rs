//! Measure native save encoding and verify an exported or raw diagnostic save.
#[allow(dead_code)]
#[path = "../src/persistence.rs"]
mod persistence;

fn main() {
    let path = std::env::args().nth(1).expect("save_roundtrip INPUT");
    let raw = std::fs::read(path).unwrap();
    let game = persistence::decode_bytes(&raw).unwrap();
    let started = std::time::Instant::now();
    let compact = persistence::encode_container(&game).unwrap();
    let encode_ms = started.elapsed().as_millis();
    let started = std::time::Instant::now();
    let restored = persistence::decode_bytes(&compact).unwrap();
    let decode_ms = started.elapsed().as_millis();
    let equal = serde_json::to_vec(&game).unwrap() == serde_json::to_vec(&restored).unwrap();
    println!(
        "{}",
        serde_json::json!({
            "depth": game.depth(), "passages": game.workings.passages.len(),
            "source_bytes": raw.len(), "save_bytes": compact.len(),
            "payload_bytes": persistence::encode(&game).unwrap().len(),
            "routes": game.transport.routes.len(),
            "encode_ms": encode_ms, "decode_ms": decode_ms, "exact_state": equal,
        })
    );
    assert!(equal, "Native save round-trip changed authoritative state");
}
