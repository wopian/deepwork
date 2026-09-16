//! Isolates tactical ability benefit from purchase/recipe strategy and offline time.
use mine_core::{materials, Action, Game};
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
            for ability in 0..3 {
                let _ = g.action(Action {
                    sequence: g.last_sequence + 1,
                    kind: "ability".into(),
                    target: String::new(),
                    value: ability,
                });
            }
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
    println!("{}", serde_json::to_string_pretty(&json!({"foreground_seconds":3600,"comparison":"Identical fixed equipment; only tactical ability use differs. This does not measure complete strategy or scheduled campaign advantage.","runs":rows})).unwrap());
}
