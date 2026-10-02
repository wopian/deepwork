//! Travel diagnostic and native visual fixture, with no hidden-geology orders.
use mine_core::{materials, Game};
fn main() {
    let arguments: Vec<_> = std::env::args().collect();
    if arguments.get(1).is_some_and(|value| value == "--profile") {
        let mut game: Game =
            serde_json::from_slice(&std::fs::read(&arguments[2]).unwrap()).unwrap();
        game.migrate().unwrap();
        let seconds: u64 = arguments[3].parse().unwrap();
        let mut states = std::collections::BTreeMap::<String, u64>::new();
        let mut construction = std::collections::BTreeMap::<String, u64>::new();
        let initial_depth = game.depth();
        let mut travelling = 0;
        let mut assigned = 0;
        for _ in 0..seconds {
            game.second(materials(), true);
            *states.entry(game.workings.status.clone()).or_default() += 1;
            let counts = game.movement.counts();
            travelling += counts.travelling as u64;
            assigned += counts.assigned as u64;
            if game.workings.status == "Waiting for supports" {
                let key = if game
                    .movement
                    .workers
                    .iter()
                    .any(|worker| worker.route == 0 && worker.activity == "building")
                {
                    if game.crew.engineers == 0 {
                        "basic_work"
                    } else {
                        "engineer_work"
                    }
                } else {
                    "awaiting_arrival"
                };
                *construction.entry(key.into()).or_default() += 1;
            }
        }
        println!(
            "{}",
            serde_json::json!({"seconds":seconds,"initial_depth":initial_depth,
            "final_depth":game.depth(),"states":states,"construction":construction,
            "travelling_worker_seconds":travelling,"assigned_worker_seconds":assigned})
        );
        return;
    }
    if arguments.get(1).is_some_and(|value| value == "--inspect") {
        let mut game: Game =
            serde_json::from_slice(&std::fs::read(&arguments[2]).unwrap()).unwrap();
        game.migrate().unwrap();
        game.workings.prepare_deposit_order_versioned(
            game.seed,
            game.profile,
            game.generator_version,
            materials(),
        );
        let floors: Vec<_> = game
            .workings
            .passages
            .iter()
            .filter(|p| !p.lift)
            .map(|p| p.feet)
            .collect();
        let remaining: Vec<_> = game
            .workings
            .target_cells
            .iter()
            .copied()
            .filter(|p| {
                !game.terrain.contains(p[0], p[1])
                    && game
                        .terrain
                        .known_material(p[0], p[1])
                        .is_some_and(|id| id > 1)
            })
            .collect();
        let eligible: Vec<_> = remaining
            .iter()
            .copied()
            .filter(|p| {
                p[1] >= mine_core::geometry::PIT_ROWS
                    && !floors.iter().any(|f| {
                        f[0] == p[0]
                            && f[1] < p[1]
                            && f[1]
                                >= p[1].saturating_sub(mine_core::workings::settings().pillar_width)
                    })
            })
            .collect();
        let unknown = game
            .workings
            .target_cells
            .iter()
            .filter(|p| !game.terrain.contains(p[0], p[1]) && !game.terrain.is_revealed(p[0], p[1]))
            .count();
        println!(
            "{}",
            serde_json::json!({"target":game.workings.target,"known_remaining":remaining.len(),
            "unknown_remaining":unknown,"eligible_remaining":eligible.len(),"eligible_sample":eligible.iter().take(8).collect::<Vec<_>>(),
            "purchase_blocker":game.pinned.as_ref().and_then(|id|game.purchase_blocker(id)),
            "depth_limit":game.equipment_depth_limit(),"crew":game.movement.counts()})
        );
        return;
    }
    if arguments.get(1).is_some_and(|value| value == "--resume") {
        let mut game: Game =
            serde_json::from_slice(&std::fs::read(&arguments[2]).unwrap()).unwrap();
        game.migrate().unwrap();
        let seconds: u64 = arguments
            .get(3)
            .map(|value| value.parse().unwrap())
            .unwrap_or(3600);
        game.last_saved = 1;
        game.advance_offline(1 + seconds * 2, materials());
        println!(
            "{}",
            serde_json::json!({"products":game.products,"cells":game.excavated,
            "depth":game.depth(),"status":game.workings.status,"crew":game.movement.counts()})
        );
        std::fs::write(
            "target/travel-resumed.json",
            serde_json::to_vec(&game).unwrap(),
        )
        .unwrap();
        return;
    }
    let retired = std::env::args().any(|v| v == "--retired");
    let mut game = Game::new(if retired { 46 } else { 42 }, 1);
    if retired {
        game.ranks.insert("metallurgy".into(), 3);
        game.heights.insert(256, 1200);
        game.steel_made = true;
        game.action(mine_core::Action {
            sequence: 1,
            kind: "retire".into(),
            target: String::new(),
            value: 0,
        })
        .unwrap();
        game.levels.insert("furnace".into(), 1);
        game.pinned = Some("steelworks".into());
        game.last_saved = 1;
        game.advance_offline(2401, materials());
        std::fs::write(
            "target/travel-retired.json",
            serde_json::to_vec(&game).unwrap(),
        )
        .unwrap();
        println!(
            "{}",
            serde_json::json!({"products":game.products,"status":game.workings.status,"crew":game.movement.counts()})
        );
        return;
    }
    game.workers = 16;
    game.housing = 32;
    for (id, level) in [
        ("shaft", 3),
        ("supports", 1),
        ("drill", 3),
        ("conveyor", 3),
        ("sorter", 5),
        ("power", 5),
        ("capacity", 5),
        ("survey", 1),
    ] {
        game.levels.insert(id.into(), level);
    }
    for _ in 0..600 {
        game.second(materials(), false);
    }
    std::fs::write(
        "target/travel-fixture.json",
        serde_json::to_vec(&game).unwrap(),
    )
    .unwrap();
    println!(
        "{}",
        serde_json::json!({"depth":game.depth(),"cells":game.excavated,"passages":game.workings.passages.len(),"status":game.workings.status,"crew":game.movement.counts()})
    );
}
