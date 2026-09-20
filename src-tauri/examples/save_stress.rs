//! Accelerated first-mine save growth check. Keeps one site active for 24 hours.
#[allow(dead_code)]
#[path = "../src/persistence.rs"]
mod persistence;

use mine_core::{materials, Action, Game};

fn buy(game: &mut Game, target: &str) -> bool {
    game.action(Action {
        sequence: game.last_sequence + 1,
        kind: "buy".into(),
        target: target.into(),
        value: 0,
    })
    .is_ok()
}

fn main() {
    let cat = materials();
    let mut game = Game::new(0xD33F_570E, 1);
    let plan = [
        "worker",
        "furnace",
        "conveyor",
        "sorter",
        "steelworks",
        "shaft",
        "supports",
        "minecart",
        "power",
        "manufacturing",
        "chemical",
        "electrolytic",
        "trace",
        "capacity",
        "drill",
    ];
    let mut target = 0;
    let mut maximum_bytes = 0;
    let mut maximum_routes = 0;
    for second in 1..=24 * 60 * 60 {
        if let Some(upgrade) = plan.get(target) {
            game.pinned = Some((*upgrade).into());
            if buy(&mut game, upgrade) {
                target += 1;
            }
        } else if second % 300 == 0 {
            if game.workers >= game.housing {
                buy(&mut game, "housing");
            } else if game.workers < 48 {
                buy(&mut game, "worker");
            } else if game.level("capacity") < 20 {
                buy(&mut game, "capacity");
            } else if game.level("drill") < 20 {
                buy(&mut game, "drill");
            }
        }
        if second % 900 == 0 {
            let policy = ["vein", "depth", "bulk"][(second / 900) as usize % 3];
            let _ = game.action(Action {
                sequence: game.last_sequence + 1,
                kind: "policy".into(),
                target: policy.into(),
                value: 0,
            });
        }
        game.second(&cat, false);
        if second % 600 == 0 || second == 6 * 60 * 60 || second == 24 * 60 * 60 {
            game.validate().unwrap();
            let bytes = persistence::encode_container(&game).unwrap().len();
            maximum_bytes = maximum_bytes.max(bytes);
            maximum_routes = maximum_routes.max(game.transport.routes.len());
            if second == 6 * 60 * 60 || second == 24 * 60 * 60 {
                println!(
                    "{}",
                    serde_json::json!({
                        "hours": second / 3600,
                        "save_bytes": bytes,
                        "routes": game.transport.routes.len(),
                        "max_save_bytes": maximum_bytes,
                        "max_routes": maximum_routes,
                        "depth": game.depth(),
                        "excavated": game.excavated,
                        "cargo": game.transport.mass(),
                    })
                );
            }
            assert!(bytes < 2 * 1024 * 1024, "save exceeded 2 MiB");
        }
    }
}
