//! Compare a real diagnostic save under offline advancement and exact fixed steps.
use mine_core::{materials, Game};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let input = args.get(1).expect("offline_replay INPUT REAL_SECONDS");
    let real_seconds: u64 = args.get(2).expect("elapsed real seconds").parse().unwrap();
    let mut offline: Game = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
    offline.terrain.rebuild().unwrap();
    offline.validate().unwrap();
    offline.last_saved = 1000;
    offline.offline = None;
    let initial_depth = offline.depth();
    let initial_passages = offline.workings.passages.len();
    let mut stepped = offline.clone();
    let started = std::time::Instant::now();
    offline.advance_offline(1000 + real_seconds, materials());
    let offline_ms = started.elapsed().as_millis();
    let report = offline.offline.take().unwrap();
    let started = std::time::Instant::now();
    for _ in 0..real_seconds.min(28_800) / 2 * 20 {
        stepped.tick(materials(), true);
    }
    let stepped_ms = started.elapsed().as_millis();
    stepped.last_saved = offline.last_saved;
    offline.validate().unwrap();
    stepped.validate().unwrap();
    let exact = serde_json::to_vec(&offline).unwrap() == serde_json::to_vec(&stepped).unwrap();
    println!(
        "{}",
        serde_json::json!({
            "initial_depth":initial_depth, "initial_passages":initial_passages,
            "real_seconds":real_seconds, "credited_seconds":report.effective,
            "offline_compute_ms":offline_ms, "stepped_compute_ms":stepped_ms,
            "serialized_state_equal_excluding_report_and_save_timestamp":exact,
            "credits":report.credits, "excavated":report.excavated,
        })
    );
    assert!(exact, "Offline advancement differs from exact fixed steps");
}
