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
        if ["iron", "coke", "lime"].contains(&g.contracts[i].product.as_str())
            && (g.level("shaft") == 0 || !g.steel_made)
        {
            continue;
        }
        if g.contracts[i].complete {
            act(g, "new_contract", "", i as u64);
        } else {
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
        "trace",
        "ventilation",
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
        g.pinned = None;
    }
    // Select production by available modules. Reservations use the same public controls as UI.
    for recipe in recipes() {
        if g.level(&recipe.building) > 0 && !g.enabled_recipes.contains(&recipe.id) {
            act(g, "recipe", &recipe.id, 0);
        }
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
    // Keep mandatory purchases ahead of optional rate spending.
    if g.level("shaft") > 0 {
        let target_drill = (g.depth() / 300 + 1).min(5) * 10;
        if g.level("drill") < target_drill && g.credits > g.cost("drill") * 3 {
            act(g, "buy", "drill", 0);
        }
        if g.workers < 32 && g.credits > g.cost("worker") * 3 {
            if g.workers >= g.housing {
                act(g, "buy", "housing", 0);
            } else {
                act(g, "buy", "worker", 0);
            }
        }
        if g.depth() + 20 >= 300 * (1 + g.level("shaft")) {
            act(g, "buy", "shaft", 0);
        }
        for id in [
            "power", "recovery", "capacity", "conveyor", "furnace", "sorter",
        ] {
            if g.level(id) < 5 && g.credits > g.cost(id) * 5 {
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
    for visit in 0..days * 2 {
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
        eprintln!(
            "seed={seed} strategy={style} mode={mode} visit={} depth={} next={:?} credits={}",
            visit + 1,
            g.depth(),
            g.pinned,
            g.credits
        );
    }
    json!({"seed":seed,"strategy":style,"mode":mode,"complete":g.megaproject,"events":events,"sites":g.site,"depth":g.depth(),"credits":g.credits,"next_upgrade":g.pinned,"blockers":g.stages.iter().map(|f|&f.blocker).collect::<Vec<_>>(),"save_bytes":serde_json::to_vec(&g).unwrap().len()})
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
    let mut runs = Vec::new();
    for index in 0..seeds {
        let style = ["bulk", "precision", "reclamation"][index as usize % 3];
        runs.push(run(42 + index, style, days, mode));
    }
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
        serde_json::to_string_pretty(&json!({"days":days,"runs":runs,"milestones":medians}))
            .unwrap()
    );
}
