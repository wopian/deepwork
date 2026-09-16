//! Deterministic acceptance harness. Uses public gameplay commands and real offline advancement.
use mine_core::{materials, pacing, recipes, requirements, Action, Game};
use serde_json::{json, Value};
use std::collections::BTreeMap;
fn act(g: &mut Game, kind: &str, target: &str, value: u64) -> bool {
    g.action(Action {
        sequence: g.last_sequence + 1,
        kind: kind.into(),
        target: target.into(),
        value,
    })
    .is_ok()
}
fn strategy(g: &mut Game, style: &str, attentive: bool) {
    if g.workers == 3 {
        act(g, "buy", "worker", 0);
    }
    for i in 0..3 {
        if g.level("pump") == 0 {
            continue;
        }
        let c = &g.contracts[i];
        let committed = g
            .pinned
            .as_ref()
            .and_then(|id| requirements().iter().find(|u| u.id == *id))
            .and_then(|u| u.inputs.get(&c.product))
            .copied()
            .unwrap_or(0);
        let recipe_hold = if recipes()
            .iter()
            .any(|r| g.level(&r.building) > 0 && r.inputs.contains_key(&c.product))
        {
            8 * mine_core::geometry::UNITS
        } else {
            0
        };
        if c.complete {
            act(g, "new_contract", "", i as u64);
        } else if g.products.get(&c.product).copied().unwrap_or(0)
            >= c.amount + committed.max(recipe_hold)
        {
            act(g, "contract", "", i as u64);
        }
    }
    if attentive {
        for i in 0..3 {
            act(g, "ability", "", i);
        }
    }
    let branch = match style {
        "bulk" => "excavation",
        "precision" => "metallurgy",
        _ => "reclamation",
    };
    let branch = if g.ranks.get(branch).copied().unwrap_or(0) >= 3
        && g.ranks.get("metallurgy").copied().unwrap_or(0) < 6
    {
        "metallurgy"
    } else {
        branch
    };
    act(g, "research", branch, 0);
    if g.specialisation.is_none() {
        act(g, "specialise", style, 0);
    }
    // One early retirement tests retained research, site selection and fresh local economy.
    if g.site == 1 && g.steel_made && g.depth() >= 300 {
        act(
            g,
            "retire",
            "",
            match style {
                "bulk" => 0,
                "precision" => 1,
                _ => 2,
            },
        );
    }
    let order = [
        "conveyor",
        "furnace",
        "steelworks",
        "shaft",
        "supports",
        "minecart",
        "power",
        "manufacturing",
        "chemical",
        "pump",
        "electrolytic",
        "ventilation",
        "trace",
        "train",
        "reclaimer",
        "slagcrusher",
        "water_recovery",
        "heat_recovery",
    ];
    let next = order
        .iter()
        .find(|id| g.level(id) == 0 && requirements().iter().any(|u| u.id == **id));
    if let Some(&id) = next {
        g.pinned = Some(id.into());
        if act(g, "buy", id, 0) {
            g.pinned = order
                .iter()
                .find(|next| g.level(next) == 0 && requirements().iter().any(|u| u.id == ***next))
                .map(|s| s.to_string());
        }
    } else {
        g.pinned = if g.depth() < 3900 {
            Some("shaft".into())
        } else {
            None
        };
    }
    // Select only recipes needed for the next module, then headquarters components.
    let mut needed = std::collections::BTreeSet::new();
    let mut pending: Vec<String> = g
        .pinned
        .as_ref()
        .filter(|_| order.iter().any(|id| g.level(id) == 0))
        .and_then(|id| requirements().iter().find(|u| u.id == *id))
        .map(|u| u.inputs.keys().cloned().collect())
        .unwrap_or_else(|| {
            vec![
                "advanced_structure".into(),
                "precision_controls".into(),
                "magnets".into(),
                "batteries".into(),
            ]
        });
    while let Some(product) = pending.pop() {
        if let Some(recipe) = recipes().iter().find(|r| r.output == product) {
            if needed.insert(recipe.id.clone()) {
                pending.extend(recipe.inputs.keys().cloned());
            }
        }
    }
    for recipe in recipes() {
        if ["steel", "aluminium"].contains(&recipe.id.as_str()) {
            let wanted = recipe.id == "aluminium"
                || order.iter().any(|id| g.level(id) == 0)
                || g.products.get("advanced_structure").copied().unwrap_or(0)
                    < 10 * mine_core::geometry::UNITS;
            if g.paused_recipes.contains(&recipe.id) == wanted {
                act(g, "recipe", &recipe.id, 0);
            }
            continue;
        }
        let wanted = needed.contains(&recipe.id) && g.level(&recipe.building) > 0;
        if g.enabled_recipes.contains(&recipe.id) != wanted {
            act(g, "recipe", &recipe.id, 0);
        }
    }
    for (product, units) in [
        ("iron", 16),
        ("coke", 16),
        ("lime", 16),
        ("steel", 16),
        ("silica", 8),
        ("copper", 8),
        ("insulation", 4),
        ("alumina", 8),
        ("aluminium", 4),
        ("wiring", 2),
        ("graphite", 8),
        ("lithium_carbonate", 4),
        ("nickel", 4),
        ("cobalt", 4),
        ("borate", 4),
        ("ferrovanadium", 4),
    ] {
        act(g, "reserve", product, units * mine_core::geometry::UNITS);
    }
    for product in [
        "advanced_structure",
        "precision_controls",
        "magnets",
        "batteries",
    ] {
        act(g, "reserve", product, 10 * mine_core::geometry::UNITS);
    }
    act(g, "megaproject", "", 0);
    // Recover finite tailings when an exhausted site cannot fund its next module.
    if g.level("reclaimer") == 0
        && g.level("furnace") > 0
        && g.level("shaft") > 0
        && g.tailings.values().sum::<u64>() >= 100 * mine_core::geometry::UNITS
    {
        act(g, "buy", "reclaimer", 0);
    }
    // Keep mandatory purchases ahead of optional rate spending.
    let spendable = |g: &Game| {
        g.credits
            .saturating_sub(g.pinned.as_ref().map(|id| g.cost(id)).unwrap_or(0))
    };
    if g.level("shaft") > 0 {
        let target_drill = (g.depth() / 300 + 1).min(5) * 10;
        if g.level("drill") < target_drill && spendable(g) > g.cost("drill") * 3 {
            act(g, "buy", "drill", 0);
        }
        if g.workers < 32 && spendable(g) > g.cost("worker") * 3 {
            if g.workers >= g.housing {
                act(g, "buy", "housing", 0);
            } else {
                act(g, "buy", "worker", 0);
            }
        }
        if g.depth() + 20 >= 300 * (1 + g.level("shaft")) && g.credits >= g.cost("shaft") {
            act(g, "buy", "shaft", 0);
        }
        for id in [
            "power", "recovery", "capacity", "conveyor", "furnace", "sorter",
        ] {
            if g.level(id) < 5 && spendable(g) > g.cost(id) * 5 {
                act(g, "buy", id, 0);
            }
        }
    }
    let policy = match style {
        "bulk" => "bulk",
        "precision" => "vein",
        _ => "depth",
    };
    let policy = if g.depth() + 4 >= 300 * (1 + g.level("shaft")) {
        "bulk"
    } else {
        policy
    };
    act(g, "policy", policy, 0);
}
fn record(g: &Game, wall: u64, events: &mut BTreeMap<String, u64>) {
    for (name, reached) in [
        ("worker", g.workers > 3),
        ("conveyor", g.level("conveyor") > 0),
        ("furnace", g.collection.contains("iron")),
        ("shaft", g.level("shaft") > 0),
        ("specialisation", g.specialisation.is_some()),
        ("retirement", g.site > 1),
        ("power", g.level("power") > 0),
        ("chemical", g.level("chemical") > 0),
        ("precision", g.collection.contains("precision_controls")),
        ("rare_earth", g.collection.contains("magnets")),
        ("headquarters", g.megaproject),
    ] {
        if reached {
            events.entry(name.into()).or_insert(wall);
        }
    }
}
fn run(seed: u64, style: &str, days: u64, mode: &str) -> Value {
    let mut g = Game::new(seed, 1);
    g.profile = (seed % 3) as usize;
    let cat = materials();
    let mut events = BTreeMap::new();
    let mut wall = 0;
    let mut idle_visits = 0;
    for visit in 0..days * 2 {
        if idle_visits >= 2 && g.depth() >= 300 && g.steel_made {
            act(
                &mut g,
                "retire",
                "",
                match style {
                    "bulk" => 0,
                    "precision" => 1,
                    _ => 2,
                },
            );
            idle_visits = 0;
        }
        let before = (g.site, g.excavated, g.credits, g.levels.clone());
        for second in 0..720 {
            if second % 5 == 0 {
                strategy(&mut g, style, mode == "attentive");
            }
            g.second(&cat, false);
            wall += 1;
            record(&g, wall, &mut events);
            if g.megaproject {
                break;
            }
        }
        if g.megaproject {
            break;
        }
        let gap = 12 * 3600 - 720;
        if mode == "continuous" {
            for second in 0..gap {
                if second % 30 == 0 {
                    strategy(&mut g, style, true);
                }
                g.second(&cat, false);
                wall += 1;
                record(&g, wall, &mut events);
                if g.megaproject {
                    break;
                }
            }
        } else {
            g.last_saved = 1_000_000 + wall;
            g.advance_offline(g.last_saved + gap, &cat);
            wall += gap;
            record(&g, wall, &mut events);
        }
        idle_visits = if before == (g.site, g.excavated, g.credits, g.levels.clone()) {
            idle_visits + 1
        } else {
            0
        };
        std::fs::write(format!("target/campaign-{seed}.json"), serde_json::to_vec_pretty(&json!({"seed":seed,"visit":visit+1,"site":g.site,"depth":g.depth(),"next":g.pinned,"products":g.products,"trace":g.trace_feed,"levels":g.levels,"recipes":g.enabled_recipes,"credits":g.credits,"shaft_blocker":g.purchase_blocker("shaft")})).unwrap()).unwrap();
        eprintln!(
            "seed={seed} strategy={style} mode={mode} visit={} depth={} next={:?} credits={}",
            visit + 1,
            g.depth(),
            g.pinned,
            g.credits
        );
    }
    json!({"seed":seed,"strategy":style,"mode":mode,"complete":g.megaproject,"events":events,"sites":g.site,"depth":g.depth(),"credits":g.credits,"next_upgrade":g.pinned,"purchase_blocker":g.pinned.as_ref().and_then(|id|g.purchase_blocker(id)),"products":g.products,"levels":g.levels,"blockers":g.stages.iter().map(|f|&f.blocker).collect::<Vec<_>>(),"save_bytes":serde_json::to_vec(&g).unwrap().len()})
}
fn main() {
    mine_core::content::validate().unwrap();
    let args: Vec<_> = std::env::args().collect();
    let seeds = args
        .get(1)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(30);
    let days = args
        .get(2)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(42);
    let mode = args.get(3).map(String::as_str).unwrap_or("scheduled");
    assert!(["scheduled", "attentive", "continuous"].contains(&mode));
    let workers = args
        .get(4)
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(4)
        .clamp(1, 8);
    std::fs::write("target/campaign-progress.json", "[]").unwrap();
    let next = std::sync::atomic::AtomicU64::new(0);
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut runs = Vec::new();
    std::thread::scope(|scope| {
        for _ in 0..workers {
            let sender = sender.clone();
            let next = &next;
            scope.spawn(move || loop {
                let index = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if index >= seeds {
                    break;
                }
                let style = ["bulk", "precision", "reclamation"][index as usize % 3];
                sender.send(run(42 + index, style, days, mode)).unwrap();
            });
        }
        drop(sender);
        for report in receiver {
            eprintln!(
                "Completed seed {}: headquarters={}",
                report["seed"], report["complete"]
            );
            runs.push(report);
            std::fs::write(
                "target/campaign-progress.json",
                serde_json::to_vec_pretty(&runs).unwrap(),
            )
            .unwrap();
        }
    });
    runs.sort_by_key(|r| r["seed"].as_u64());
    let mut medians = BTreeMap::new();
    for (name, window) in &pacing::get().windows {
        let mut times: Vec<_> = runs
            .iter()
            .filter_map(|r| r["events"][name].as_u64())
            .collect();
        times.sort_unstable();
        let median = times.get(times.len() / 2).copied();
        medians.insert(name,json!({"median_seconds":median,"reached":times.len(),"target_seconds":window,"in_window":median.is_some_and(|v|v>=window[0]&&v<=window[1])}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"save_version":mine_core::VERSION,"generator_version":mine_core::geometry::GENERATOR_VERSION,"days":days,"runs":runs,"milestones":medians}))
            .unwrap()
    );
}
