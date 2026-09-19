//! Measure native save encoding and verify an exported or raw diagnostic save.
#[allow(dead_code)]
#[path = "../src/persistence.rs"]
mod persistence;

fn main() {
    let path = std::env::args().nth(1).expect("save_roundtrip INPUT.json");
    let raw = std::fs::read_to_string(path).unwrap();
    let game = persistence::decode(&raw).unwrap();
    let started = std::time::Instant::now();
    let compact = persistence::encode(&game).unwrap();
    let encode_ms = started.elapsed().as_millis();
    let started = std::time::Instant::now();
    let restored = persistence::decode(&compact).unwrap();
    let decode_ms = started.elapsed().as_millis();
    let equal = serde_json::to_vec(&game).unwrap() == serde_json::to_vec(&restored).unwrap();
    println!("{}", serde_json::json!({
        "depth": game.depth(), "passages": game.workings.passages.len(),
        "source_bytes": raw.len(), "save_bytes": compact.len(),
        "encode_ms": encode_ms, "decode_ms": decode_ms, "exact_state": equal,
    }));
    assert!(equal, "Native save round-trip changed authoritative state");
}
