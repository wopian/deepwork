//! Deterministic underground acceptance fixture; no hidden ore is supplied to the planner.
use mine_core::{
    geometry::{PIT_ROWS, WIDTH},
    materials,
    terrain::Terrain,
    Game,
};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let seconds: u64 = args.get(1).and_then(|v| v.parse().ok()).unwrap_or(1800);
    let seed = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(42);
    let mut g = Game::new(seed, 1);
    g.policy = args.get(3).cloned().unwrap_or("vein".into());
    g.priorities = vec![3, 4, 19];
    g.workers = 32;
    g.housing = 64;
    for (k, n) in [
        ("shaft", 6),
        ("supports", 3),
        ("pump", 1),
        ("ventilation", 1),
        ("drill", 5),
        ("conveyor", 5),
        ("sorter", 10),
        ("capacity", 10),
        ("survey", 1),
        ("power", 10),
    ] {
        g.levels.insert(k.into(), n);
    }
    if args.get(5).is_some_and(|arg| arg == "starter") {
        g.workers = 4;
        g.housing = 8;
        g.profile = args.get(6).and_then(|arg| arg.parse().ok()).unwrap_or(0);
        g.levels.clear();
        for (id, level) in [
            ("shaft", 1),
            ("drill", 2),
            ("conveyor", 1),
            ("furnace", 1),
            ("steelworks", 1),
            ("power", 1),
        ] {
            g.levels.insert(id.into(), level);
        }
    }
    // Valid benched pit and ramp, bypass only the already-tested surface phase.
    g.heights = (0..WIDTH)
        .map(|x| {
            if x < 16 {
                0
            } else {
                (x - 15).min(PIT_ROWS).min(((WIDTH - 17 - x) / 24 + 1) * 24)
            }
        })
        .collect();
    g.terrain = Terrain::from_columns(&g.heights);
    g.excavated = g.heights.iter().map(|v| *v as u64).sum();
    g.disposed_mass = g.excavated * 1000;
    let cat = materials();
    for second in 0..seconds {
        g.second(&cat, false);
        if second % 300 == 299 {
            eprintln!(
                "{}s depth={} passages={} status={} search={:?} remaining={}",
                second + 1,
                g.depth(),
                g.workings.passages.len(),
                g.workings.status,
                g.workings.search.as_ref().map(|s| s.expanded),
                g.workings
                    .section
                    .as_ref()
                    .map(|s| s
                        .cells
                        .iter()
                        .filter(|p| !g.terrain.contains(p[0], p[1]))
                        .count())
                    .unwrap_or(0)
            );
        }
    }
    let routes = g
        .workings
        .passages
        .iter()
        .enumerate()
        .filter(|(i, n)| *i > 0 && n.lift)
        .count();
    println!(
        "{}",
        serde_json::json!({"seed":seed,"seconds":seconds,"depth":g.depth(),"passages":g.workings.passages.len(),"lifts":routes,"signals":g.workings.signals.len(),"cells":g.excavated,"status":g.workings.status})
    );
    if let Some(path) = args.get(4) {
        std::fs::write(path, serde_json::to_vec(&g).unwrap()).unwrap();
    }
    assert!(g.depth() > 60, "underground development must progress");
    assert!(g.workings.valid());
}
