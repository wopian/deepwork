//! Compares fixed depth policy with intervention using discovered commodity prices.
use mine_core::{materials, Game};
use serde_json::json;
fn run(seed: u64, active: bool) -> serde_json::Value {
    let mut g = Game::new(seed, 2);
    g.profile = seed as usize % 3;
    g.workers = 24;
    g.housing = 24;
    g.pinned = None;
    for id in [
        "conveyor",
        "furnace",
        "sorter",
        "shaft",
        "supports",
        "pump",
        "ventilation",
        "power",
        "steelworks",
        "manufacturing",
        "chemical",
        "electrolytic",
        "trace",
        "minecart",
        "capacity",
    ] {
        g.levels
            .insert(id.into(), if id == "shaft" { 20 } else { 3 });
    }
    g.policy = "depth".into();
    let cat = materials();
    let initial = g.credits;
    for _ in 0..3600 {
        if active {
            let mut ranked: Vec<_> = g.discoveries.iter().copied().filter(|id| *id > 1).collect();
            ranked.sort_by_key(|id| std::cmp::Reverse(cat[*id].price));
            g.priorities = ranked.into_iter().take(3).collect();
            g.policy = "vein".into();
        }
        g.second(&cat, false);
    }
    json!({"seed":seed,"active":active,"credits":g.credits-initial,"excavated_quanta":g.excavated*mine_core::geometry::CELL_MASS,"depth":g.depth(),"transport_quanta":g.transport.mass(),"products":g.products})
}
fn main() {
    let mut rows = Vec::new();
    for seed in 42..45 {
        let idle = run(seed, false);
        let active = run(seed, true);
        let gain = (active["credits"].as_u64().unwrap() as f64
            / idle["credits"].as_u64().unwrap().max(1) as f64
            - 1.)
            * 100.;
        rows.push(json!({"idle":idle,"active":active,"credit_gain_percent":gain}));
    }
    println!("{}", serde_json::to_string_pretty(&json!({"foreground_seconds":3600,"comparison":"Identical fixed equipment; known commodity priorities and vein policy versus fixed depth policy. No cooldown abilities.","runs":rows})).unwrap());
}
