//! Signed-world compatibility fingerprint for profiling generator-preserving changes.
fn main() {
    let started = std::time::Instant::now();
    let mut hash = 0xcbf29ce484222325_u64;
    let mut samples = 0;
    let version = std::env::args()
        .nth(1)
        .map(|v| v.parse::<u32>().unwrap())
        .unwrap_or(5);
    for seed in [42, 314159] {
        for profile in 0..3 {
            for y in (0..16000).step_by(31) {
                for x in (-1024..1024).step_by(29) {
                    let id = mine_core::geology::sample_versioned(
                        seed,
                        profile,
                        version,
                        x,
                        y,
                        mine_core::materials(),
                    );
                    hash = (hash ^ id as u64).wrapping_mul(0x100000001b3);
                    samples += 1;
                }
            }
        }
    }
    println!(
        "{}",
        serde_json::json!({
            "generator": version,
            "samples": samples,
            "fingerprint": format!("{hash:016x}"),
            "compute_ms": started.elapsed().as_millis()
        })
    );
}
