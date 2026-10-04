//! Measure full idle recovery and a bounded comparison with online cargo physics.
use mine_core::{materials, Game};
use std::time::Instant;

fn mass(g: &Game) -> u64 {
    g.ore
        .values()
        .chain(g.hauled.values())
        .chain(g.concentrate.values())
        .chain(g.raw_stock.values())
        .chain(g.products.values())
        .chain(g.trace_feed.values())
        .chain(g.tailings.values())
        .sum::<u64>()
        + g.mining_fronts.iter().map(|f| f.stockpile).sum::<u64>()
        + g.transport.mass()
        + g.slag
        + g.depleted
        + g.sold_mass
        + g.disposed_mass
        + g.delivered_mass
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let mut initial: Game = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    initial.migrate().unwrap();
    initial.validate().unwrap();
    initial.last_saved = 1000;
    let seconds: u64 = args.get(2).map(|v| v.parse().unwrap()).unwrap_or(28_800);
    let mut idle = initial.clone();
    let started = Instant::now();
    let mut frames = 0;
    idle.advance_offline_observed(1000 + seconds, materials(), |_, _, _| frames += 1);
    let idle_ms = started.elapsed().as_secs_f64() * 1000.;
    idle.validate().unwrap();
    assert_eq!(
        mass(&idle) - mass(&initial),
        (idle.excavated - initial.excavated) * mine_core::geometry::CELL_MASS
    );
    // Compare identical starting states over 120 credited seconds, once.
    let comparison_seconds = 120;
    let mut physical = initial.clone();
    let started = Instant::now();
    for _ in 0..comparison_seconds * 20 {
        physical.tick(materials(), true);
    }
    let physical_ms = started.elapsed().as_secs_f64() * 1000.;
    let mut bulk = initial.clone();
    let started = Instant::now();
    bulk.advance_offline(1000 + comparison_seconds * 2, materials());
    let bulk_ms = started.elapsed().as_secs_f64() * 1000.;
    bulk.validate().unwrap();
    println!(
        "{}",
        serde_json::json!({
            "real_seconds":seconds, "credited_seconds":idle.offline.as_ref().unwrap().effective,
            "idle_compute_ms":idle_ms, "production_intervals":frames-1,
            "comparison_credited_seconds":comparison_seconds, "physical_compute_ms":physical_ms,
            "bulk_compute_ms":bulk_ms, "speedup":physical_ms / bulk_ms,
            "excavated":idle.excavated-initial.excavated, "credits":idle.offline.as_ref().unwrap().credits,
            "cargo_batches_before":initial.transport.segments.iter().map(|s|s.batches.len()).sum::<usize>(),
            "cargo_batches_after":idle.transport.segments.iter().map(|s|s.batches.len()).sum::<usize>(),
            "mass_conserved":true
        })
    );
}
