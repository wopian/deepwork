//! Signed-world compatibility fingerprint for profiling generator-preserving changes.
fn main() {
    let started = std::time::Instant::now();
    let mut hash = 0xcbf29ce484222325_u64;
    let mut samples = 0;
    for seed in [42, 314159] {
        for profile in 0..3 {
            for y in (0..16000).step_by(31) {
                for x in (-1024..1024).step_by(29) {
                    let id =
                        mine_core::geology::sample(seed, profile, x, y, mine_core::materials());
                    hash = (hash ^ id as u64).wrapping_mul(0x100000001b3);
                    samples += 1;
                }
            }
        }
    }
    println!(
        "{}",
        serde_json::json!({
            "generator": mine_core::geometry::GENERATOR_VERSION,
            "samples": samples,
            "fingerprint": format!("{hash:016x}"),
            "compute_ms": started.elapsed().as_millis()
        })
    );
}
