//! Deterministic opening acceptance. No grants, debug digging, or hidden ore targeting.
use mine_core::{materials, Action, Game};
fn main() {
    let cat = materials();
    let count = std::env::args()
        .nth(1)
        .map(|s| s.parse::<u64>().unwrap())
        .unwrap_or(30);
    let mode = std::env::args().nth(2).unwrap_or("guided".into());
    assert!(["guided", "adverse", "released"].contains(&mode.as_str()));
    let first = std::env::args()
        .nth(3)
        .map(|s| s.parse::<u64>().unwrap())
        .unwrap_or(0);
    let mut failed = false;
    for seed in first..first + count {
        let mut g = Game::new(seed, 1);
        let buy = |g: &mut Game, id: &str| {
            g.action(Action {
                sequence: g.last_sequence + 1,
                kind: "buy".into(),
                target: id.into(),
                value: 0,
            })
            .is_ok()
        };
        if mode == "adverse" {
            buy(&mut g, "housing");
            g.action(Action {
                sequence: g.last_sequence + 1,
                kind: "pin".into(),
                target: "furnace".into(),
                value: 0,
            })
            .unwrap();
        } else {
            buy(&mut g, "worker");
        }
        let mut conveyor = None;
        let mut shaft = None;
        let mut first_sale = None;
        let mut first_iron = None;
        let mut furnace = None;
        let mut branch = None;
        let mut iron_contract = None;
        let mut last_iron_gain = 0;
        let mut last_iron_stock = 0;
        let mut exploring = false;
        let limit = std::env::args()
            .nth(4)
            .map(|s| s.parse::<u64>().unwrap())
            .unwrap_or(match mode.as_str() {
                "guided" => 600,
                "released" => 3600,
                _ => 1800,
            });
        for second in 1..=limit {
            // The guide pins the furnace; its actual quote controls the purchase.
            if g.level("furnace") == 0 && g.purchase_blocker("furnace").is_none() {
                if g.action(Action {
                    sequence: g.last_sequence + 1,
                    kind: "buy".into(),
                    target: "furnace".into(),
                    value: 0,
                })
                .is_ok()
                {
                    furnace = Some(second);
                }
            }
            if g.level("furnace") > 0 && g.level("conveyor") == 0 && buy(&mut g, "conveyor") {
                conveyor = Some(second);
            }
            // A player may release the first iron to earn credits, then pin the
            // lift later. Recovery must use remaining reachable feed, not grants.
            if mode == "released" && iron_contract.is_none() {
                if g.action(Action {
                    sequence: g.last_sequence + 1,
                    kind: "contract".into(),
                    target: String::new(),
                    value: 0,
                })
                .is_ok()
                {
                    iron_contract = Some(second);
                }
            }
            let saving_for_lift =
                mode != "released" || iron_contract.is_some_and(|sold| second >= sold + 120);
            if !saving_for_lift && g.level("furnace") > 0 {
                if let Some(target) = g.pinned.clone() {
                    g.action(Action {
                        sequence: g.last_sequence + 1,
                        kind: "pin".into(),
                        target,
                        value: 0,
                    })
                    .unwrap();
                }
            }
            if g.level("furnace") > 0 && g.pinned.as_deref() != Some("shaft") && saving_for_lift {
                g.action(Action {
                    sequence: g.last_sequence + 1,
                    kind: "pin".into(),
                    target: "shaft".into(),
                    value: 0,
                })
                .unwrap();
            }
            if saving_for_lift
                && g.level("conveyor") > 0
                && g.level("shaft") == 0
                && buy(&mut g, "shaft")
            {
                shaft = Some(second);
            }
            if mode == "released" && saving_for_lift && second >= 600 && shaft.is_none() {
                // Respond to the visible missing-iron quote using public controls.
                // Explore again only when the chosen vein stops delivering iron.
                if !exploring && second >= 1800 && second - last_iron_gain >= 300 {
                    exploring = true;
                    g.action(Action {
                        sequence: g.last_sequence + 1,
                        kind: "clear_vein".into(),
                        target: String::new(),
                        value: 0,
                    })
                    .unwrap();
                }
                for (kind, target, value) in [
                    ("priority", "", 3),
                    ("policy", if exploring { "depth" } else { "vein" }, 0),
                ] {
                    if kind == "priority" && g.priorities.contains(&3) {
                        continue;
                    }
                    g.action(Action {
                        sequence: g.last_sequence + 1,
                        kind: kind.into(),
                        target: target.into(),
                        value,
                    })
                    .unwrap();
                }
                if g.workers < 6 {
                    buy(&mut g, "worker");
                }
                if g.level("survey") < 2 {
                    buy(&mut g, "survey");
                }
                if exploring && g.level("drill") < 2 {
                    buy(&mut g, "drill");
                }
                if !exploring
                    && second % 30 == 0
                    && g.workings
                        .order_view(&g.terrain)
                        .is_none_or(|v| v.known_cells == 0)
                {
                    let at = g.work_route().first().copied().unwrap_or([256, 0]);
                    let target = g
                        .terrain
                        .visible
                        .iter()
                        .flat_map(|(&chunk, visible)| {
                            let (x, y) = mine_core::geometry::chunk_origin(chunk);
                            visible.iter().enumerate().filter_map(move |(index, &id)| {
                                (id == 3).then_some([x + index as i64 % 64, y + index as i64 / 64])
                            })
                        })
                        .filter(|p| !g.terrain.contains(p[0], p[1]))
                        .min_by_key(|p| p[0].abs_diff(at[0]) + p[1].abs_diff(at[1]));
                    if let Some(p) = target {
                        g.action(Action {
                            sequence: g.last_sequence + 1,
                            kind: "target_vein".into(),
                            target: format!("{},{}", p[0], p[1]),
                            value: 0,
                        })
                        .unwrap();
                    }
                }
            }
            g.second(&cat, false);
            let iron_stock = g.products.get("iron").copied().unwrap_or(0);
            if iron_stock > last_iron_stock {
                last_iron_gain = second;
            }
            last_iron_stock = iron_stock;
            if g.sold_mass > 0 {
                first_sale.get_or_insert(second);
            }
            if g.collection.contains("iron") {
                first_iron.get_or_insert(second);
            }
            // An angled access drive alone is not an underground branch.
            let mut children = vec![0; g.workings.passages.len()];
            for passage in g.workings.passages.iter().skip(1) {
                children[passage.parent] += 1;
            }
            if children
                .iter()
                .enumerate()
                .any(|(i, &count)| count >= 2 && g.workings.passages[i].feet[1] > 0)
            {
                branch.get_or_insert(second);
            }
            if mode == "released" && shaft.is_some() && branch.is_some() {
                break;
            }
        }
        let pass = first_sale.is_some_and(|s| s <= 60)
            && first_iron.is_some_and(|s| s <= if mode == "guided" { 300 } else { 1800 })
            && branch.is_some()
            && (mode == "guided" || shaft.is_some())
            && (mode != "released" || iron_contract.is_some());
        failed |= !pass;
        if !pass {
            std::fs::create_dir_all("target").unwrap();
            std::fs::write(
                format!("target/opening-{mode}-{seed}.json"),
                serde_json::to_vec(&g).unwrap(),
            )
            .unwrap();
        }
        println!(
            "{}",
            serde_json::json!({"seed":seed,"mode":mode,"limit_seconds":limit,"iron_contract":iron_contract,"pass":pass,"conveyor":conveyor,"shaft":shaft,"sale":first_sale,"furnace":furnace,"iron":first_iron,"branch":branch,"cells":g.excavated,"depth":g.depth(),"credits":g.credits,"status":g.workings.status,"stages":g.stages})
        );
    }
    if failed {
        std::process::exit(1);
    }
}
