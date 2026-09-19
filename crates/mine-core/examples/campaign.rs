//! Deterministic acceptance harness. Uses public gameplay commands and real offline advancement.
use mine_core::{materials, pacing, recipes, requirements, Action, Game};
use serde_json::{json, Value};
use std::collections::BTreeMap;
#[derive(serde::Serialize, serde::Deserialize)]
struct Checkpoint {
    seed: u64,
    style: String,
    mode: String,
    content: u64,
    visit: u64,
    wall: u64,
    idle_visits: u64,
    events: BTreeMap<String, u64>,
    stalls: Vec<Value>,
    game: Game,
}
fn content_fingerprint() -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for text in [
        // Never resume an older decision policy as if it were the same strategy.
        include_str!("campaign.rs"),
        include_str!("../../../content/pacing.json"),
        include_str!("../../../content/upgrades.json"),
        include_str!("../../../content/materials.json"),
        include_str!("../../../content/recipes.json"),
        include_str!("../../../content/deposits.json"),
        include_str!("../../../content/mining.json"),
        include_str!("../../../content/sites.json"),
        include_str!("../../../content/traces.json"),
    ] {
        for byte in text.bytes().chain(std::iter::once(0)) {
            hash = (hash ^ byte as u64).wrapping_mul(0x100000001b3);
        }
    }
    hash
}
impl Checkpoint {
    fn validate(&mut self, seed: u64, style: &str, mode: &str) -> Result<(), String> {
        if self.seed != seed
            || self.style != style
            || self.mode != mode
            || self.content != content_fingerprint()
            || self.wall != self.visit * 43200
            || mode == "continuous"
            || self.game.last_saved != 1_000_000 + self.wall
            || self.events.values().any(|time| *time > self.wall)
        {
            return Err("Campaign checkpoint does not match run parameters/content".into());
        }
        self.game.terrain.rebuild()?;
        self.game.validate()
    }
}
const BUILD_ORDER: &[&str] = &[
    "furnace",
    "wheelbarrow",
    "conveyor",
    "sorter",
    "survey",
    "capacity",
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
    "recovery",
    "reclaimer",
    "slagcrusher",
];
fn act(g: &mut Game, kind: &str, target: &str, value: u64) -> bool {
    g.action(Action {
        sequence: g.last_sequence + 1,
        kind: kind.into(),
        target: target.into(),
        value,
    })
    .is_ok()
}
// Use public recipe/catalogue knowledge only. Terrain and hidden geology are never inspected.
fn pinned_feeds(g: &Game) -> Vec<usize> {
    let requirement = g
        .pinned
        .as_ref()
        .and_then(|id| requirements().iter().find(|u| u.id == *id));
    let mut pending: Vec<(String, u64)> = requirement
        .map(|u| u.inputs.iter().map(|(p, n)| (p.clone(), *n)).collect())
        .unwrap_or_default();
    if BUILD_ORDER.iter().all(|id| g.level(id) > 0) {
        // A pinned lift must not hide the feeds for the commissioned industry.
        pending.extend(
            [
                "advanced_structure",
                "precision_controls",
                "magnets",
                "batteries",
            ]
            .into_iter()
            .map(|p| (p.into(), 10 * mine_core::geometry::UNITS)),
        );
    }
    let cat = materials();
    let mut visited = std::collections::BTreeSet::new();
    let mut feeds = std::collections::BTreeSet::new();
    while let Some((product, target)) = pending.pop() {
        let available = if product.ends_with("_residue") {
            g.trace_feed.get(&product)
        } else {
            g.products.get(&product)
        }
        .copied()
        .unwrap_or(0);
        if available >= target || !visited.insert(product.clone()) {
            continue;
        }
        if let Some(recipe) = recipes().iter().find(|r| r.output == product) {
            let batches = (target - available).div_ceil(recipe.inputs.values().sum());
            for (input, quantity) in &recipe.inputs {
                let committed = requirement
                    .and_then(|u| u.inputs.get(input))
                    .copied()
                    .unwrap_or(0)
                    .max(
                        if input == "iron"
                            && g.pinned.is_some()
                            && g.level("shaft") > 0
                            && g.level("shaft") < 50
                        {
                            8 * mine_core::geometry::UNITS
                        } else {
                            0
                        },
                    );
                pending.push((input.clone(), batches * quantity + committed));
            }
        } else if let Some(output) = product.strip_suffix("_residue") {
            let sources: Vec<_> = mine_core::traces()
                .iter()
                .filter(|rule| rule.output == output)
                .collect();
            if let Some(source) = sources
                .iter()
                .find(|rule| g.discoveries.contains(&rule.feed))
                .or_else(|| sources.first())
            {
                feeds.insert(source.feed);
            }
        } else {
            let matches = |m: &&mine_core::Material| {
                m.product == product || product == "alumina" && m.name == "Bauxite"
            };
            if let Some(material) = cat
                .iter()
                .filter(matches)
                .find(|m| g.discoveries.contains(&m.id))
                .or_else(|| cat.iter().find(matches))
            {
                feeds.insert(material.id);
            }
        }
    }
    feeds.into_iter().take(3).collect()
}

fn strategy(g: &mut Game, style: &str, attentive: bool) {
    if g.workers == 3 {
        act(g, "buy", "worker", 0);
    }
    for i in 0..3 {
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
            >= c.amount
                + committed
                    .max(recipe_hold)
                    .max(g.reserve.get(&c.product).copied().unwrap_or(0))
        {
            act(g, "contract", "", i as u64);
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
    let mut research_goal = BUILD_ORDER
        .iter()
        .find(|id| g.level(id) == 0)
        .and_then(|id| requirements().iter().find(|u| u.id == **id))
        .map(|u| u.research_points)
        .unwrap_or(pacing::get().headquarters_research);
    let endgame_known = [
        "advanced_structure",
        "precision_controls",
        "magnets",
        "batteries",
    ]
    .iter()
    .all(|p| g.collection.contains(*p));
    if endgame_known {
        research_goal = pacing::get().headquarters_research;
    }
    if g.site > 1
        && g.depth() >= 300
        && g.steel_made
        && g.research_invested() < research_goal
        // Commission the first electrolysis line before dismantling the site.
        // Otherwise a five-second strategy cadence can skip its first output
        // and falsely report precision materials weeks after they were usable.
        && (g.level("electrolytic") == 0 || g.collection.contains("aluminium"))
        && (BUILD_ORDER.iter().any(|id| g.level(id) == 0) || endgame_known)
    {
        act(g, "retire", "", (g.site % 3) as u64);
        return;
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
    let order = BUILD_ORDER;
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
            // Completed component lines must stop consuming shared feeds such
            // as iron needed for a lift. Re-enable them if reserved stock falls.
            [
                "advanced_structure",
                "precision_controls",
                "magnets",
                "batteries",
            ]
            .into_iter()
            .filter(|product| {
                g.products.get(*product).copied().unwrap_or(0) < 10 * mine_core::geometry::UNITS
            })
            .map(str::to_string)
            .collect()
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
    let established = g.site > 1 || g.level("shaft") > 0;
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
        act(
            g,
            "reserve",
            product,
            if established {
                units * mine_core::geometry::UNITS
            } else {
                0
            },
        );
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
        if g.level("survey") < 2 && spendable(g) > g.cost("survey") * 2 {
            act(g, "buy", "survey", 0);
        }
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
        if g.depth() + 20 >= g.lift_depth_limit() && g.credits >= g.cost("shaft") {
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
    let feeds = pinned_feeds(g);
    for id in g.priorities.clone() {
        if !feeds.contains(&id) {
            act(g, "priority", "", id as u64);
        }
    }
    for id in feeds {
        if !g.priorities.contains(&id) {
            act(g, "priority", "", id as u64);
        }
    }
    let sampled_required_feed = g.priorities.iter().any(|id| {
        g.terrain
            .ore_frontiers
            .get(id)
            .is_some_and(|faces| !faces.is_empty())
    });
    let equipment_limit = g.equipment_depth_limit();
    let material_shortage = g
        .pinned
        .as_ref()
        .and_then(|id| requirements().iter().find(|u| u.id == *id))
        .is_some_and(|u| {
            u.inputs
                .iter()
                .any(|(p, n)| g.products.get(p).copied().unwrap_or(0) < *n)
        });
    // Styles choose specialisation and headquarters investment. All competent
    // strategies can develop retirement access or follow essential construction feed.
    let selected_required = g
        .workings
        .target
        .and_then(|p| g.terrain.known_material(p[0], p[1]))
        .is_some_and(|id| g.priorities.contains(&id));
    let policy = if selected_required {
        "vein"
    } else if g.level("shaft") > 0 && g.depth() < 300 && g.research_invested() < research_goal {
        "depth"
    } else if g.level("shaft") > 0 && material_shortage && !g.priorities.is_empty() {
        if sampled_required_feed || g.depth() + 4 >= equipment_limit {
            "vein"
        } else {
            "depth"
        }
    } else {
        match style {
            "bulk" => "bulk",
            "precision" => "vein",
            _ => "depth",
        }
    };
    let policy = if !selected_required && g.depth() + 4 >= g.lift_depth_limit() {
        "bulk"
    } else {
        policy
    };
    act(g, "policy", policy, 0);
    if g.ticks % if attentive { 100 } else { 600 } == 0 {
        direct_known_feed(
            g,
            &pinned_feeds(g),
            equipment_limit as i64 * mine_core::geometry::CELLS_PER_METRE,
        );
    }
}
/// Choose from the same revealed cells exposed by IPC, including surveyed ore
/// which is not yet an excavation face. Never inspect generated hidden geology.
fn direct_known_feed(g: &mut Game, feeds: &[usize], depth_limit: i64) {
    if let Some(anchor) = g.workings.target {
        let wanted = g
            .terrain
            .known_material(anchor[0], anchor[1])
            .is_some_and(|id| feeds.contains(&id));
        let remaining = g
            .workings
            .order_view(&g.terrain)
            .is_some_and(|v| v.known_cells > 0);
        if wanted && remaining {
            return;
        }
        act(g, "clear_vein", "", 0);
    }
    if feeds.is_empty() {
        return;
    }
    let at = g.work_route().first().copied().unwrap_or([256, 0]);
    let mut best = None;
    for (&chunk, visible) in &g.terrain.visible {
        let (cx, cy) = mine_core::geometry::chunk_origin(chunk);
        if cy >= depth_limit {
            continue;
        }
        for (index, &id) in visible.iter().enumerate() {
            if !feeds.contains(&(id as usize)) {
                continue;
            }
            let p = [cx + index as i64 % 64, cy + index as i64 / 64];
            if p[1] >= depth_limit || g.terrain.contains(p[0], p[1]) {
                continue;
            }
            let distance = p[0].abs_diff(at[0]) + p[1].abs_diff(at[1]);
            if best.is_none_or(|(score, _)| distance < score) {
                best = Some((distance, p));
            }
        }
    }
    if let Some((_, p)) = best {
        act(g, "target_vein", &format!("{},{}", p[0], p[1]), 0);
    }
}
fn record(g: &Game, wall: u64, events: &mut BTreeMap<String, u64>) {
    for (name, reached) in [
        ("worker", g.workers > 3),
        ("conveyor", g.level("conveyor") > 0),
        ("furnace", g.collection.contains("iron")),
        ("shaft", g.level("shaft") > 0),
        ("specialisation", g.milestones.contains("specialisation")),
        (
            "tactics",
            g.site > 1 || g.depth() >= pacing::get().tactics_depth,
        ),
        ("retirement", g.site > 1),
        ("power", g.level("power") > 0),
        ("chemical", g.level("chemical") > 0),
        ("electrolysis_built", g.level("electrolytic") > 0),
        ("precision", g.collection.contains("aluminium")),
        (
            "precision_controls",
            g.collection.contains("precision_controls"),
        ),
        ("rare_earth", g.collection.contains("magnets")),
        ("headquarters", g.megaproject),
    ] {
        if reached {
            events.entry(name.into()).or_insert(wall);
        }
    }
}
/// A strategy should retire an unproductive site even while selling junk rock.
/// Track public progress toward construction, not unrelated gross income.
struct Advancement {
    site: u32,
    depth: u32,
    levels: BTreeMap<String, u32>,
    collection: usize,
    research: u64,
    funding: u64,
    stocks: BTreeMap<String, u64>,
}
impl Advancement {
    fn read(game: &Game) -> Self {
        // Public catalogue tiers define a reasonable search band. Continuing
        // below it is not progress toward a missing construction ingredient.
        // A selected, revealed deeper source can extend that search explicitly.
        let feeds = pinned_feeds(game);
        let mut search_depth = feeds
            .iter()
            .map(|id| [100, 300, 700, 1500, 3000, 6000][materials()[*id].tier.min(5) as usize])
            .max()
            .unwrap_or(300)
            .max(300);
        if let Some(target) = game.workings.target {
            if game
                .terrain
                .known_material(target[0], target[1])
                .is_some_and(|id| feeds.contains(&id))
            {
                search_depth = search_depth.max(mine_core::geometry::depth(target[1]));
            }
        }
        let mut pending: Vec<String> = game
            .pinned
            .as_ref()
            .and_then(|id| requirements().iter().find(|u| u.id == *id))
            .map(|u| u.inputs.keys().cloned().collect())
            .unwrap_or_else(|| {
                [
                    "advanced_structure",
                    "precision_controls",
                    "magnets",
                    "batteries",
                ]
                .into_iter()
                .map(String::from)
                .collect()
            });
        let mut stocks = BTreeMap::new();
        if game.pinned.is_some() && BUILD_ORDER.iter().all(|id| game.level(id) > 0) {
            pending.extend(
                [
                    "advanced_structure",
                    "precision_controls",
                    "magnets",
                    "batteries",
                ]
                .into_iter()
                .map(String::from),
            );
        }
        while let Some(product) = pending.pop() {
            if stocks.contains_key(&product) {
                continue;
            }
            stocks.insert(
                product.clone(),
                (if product.ends_with("_residue") {
                    game.trace_feed.get(&product)
                } else {
                    game.products.get(&product)
                })
                .copied()
                .unwrap_or(0)
                .min(16 * mine_core::geometry::UNITS),
            );
            if let Some(recipe) = recipes().iter().find(|r| r.output == product) {
                pending.extend(recipe.inputs.keys().cloned());
            }
        }
        Self {
            site: game.site,
            depth: game.depth().min(search_depth),
            levels: game.levels.clone(),
            collection: game.collection.len(),
            research: game.research_invested(),
            funding: game
                .pinned
                .as_ref()
                .map(|id| game.credits.min(game.cost(id)))
                .unwrap_or(0),
            stocks,
        }
    }
    fn advanced_to(&self, next: &Self) -> bool {
        next.site != self.site
            || next.depth > self.depth
            || next.collection > self.collection
            || next.research > self.research
            || next.funding > self.funding
            || next
                .levels
                .iter()
                .any(|(id, level)| *level > self.levels.get(id).copied().unwrap_or(0))
            || next
                .stocks
                .iter()
                .any(|(id, quantity)| *quantity > self.stocks.get(id).copied().unwrap_or(0))
    }
}
fn run(seed: u64, style: &str, days: u64, mode: &str, resume: Option<&std::path::Path>) -> Value {
    let run_started = std::time::Instant::now();
    let mut g = Game::new(seed, 1);
    g.profile = (seed % 3) as usize;
    let cat = materials();
    let mut events = BTreeMap::new();
    let mut wall = 0;
    let mut idle_visits = 0;
    let mut stalls = Vec::new();
    let mut resumed_visit = 0;
    if let Some(path) = resume
        .map(|dir| dir.join(format!("campaign-checkpoint-{seed}.json")))
        .filter(|p| p.exists())
    {
        let mut checkpoint: Checkpoint =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        checkpoint.validate(seed, style, mode).unwrap();
        assert!(
            checkpoint.visit < days * 2,
            "Checkpoint exceeds observation window"
        );
        resumed_visit = checkpoint.visit;
        wall = checkpoint.wall;
        idle_visits = checkpoint.idle_visits;
        events = checkpoint.events;
        stalls = checkpoint.stalls;
        g = checkpoint.game;
        eprintln!("Resumed seed={seed} after visit={resumed_visit}");
    }
    for visit in resumed_visit..days * 2 {
        let visit_started = std::time::Instant::now();
        if idle_visits >= 2 && g.depth() >= 300 && g.steel_made {
            stalls.push(json!({"visit":visit,"site":g.site,"depth":g.depth(),"products":g.products,"paused_recipes":g.paused_recipes,"next":g.pinned,"ranks":g.ranks}));
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
        let before = Advancement::read(&g);
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
        idle_visits = if !before.advanced_to(&Advancement::read(&g)) {
            idle_visits + 1
        } else {
            0
        };
        if idle_visits == 2 {
            std::fs::write(
                format!("target/campaign-stalled-{seed}.json"),
                serde_json::to_vec(&g).unwrap(),
            )
            .unwrap();
        }
        std::fs::write(
            format!("target/campaign-state-{seed}.json"),
            serde_json::to_vec(&g).unwrap(),
        )
        .unwrap();
        std::fs::write(format!("target/campaign-{seed}.json"), serde_json::to_vec_pretty(&json!({"seed":seed,"visit":visit+1,"events":events,"site":g.site,"depth":g.depth(),"next":g.pinned,"products":g.products,"trace":g.trace_feed,"levels":g.levels,"recipes":g.enabled_recipes,"paused_recipes":g.paused_recipes,"credits":g.credits,"ranks":g.ranks,"research":g.research,"invested":g.research_invested(),"shaft_blocker":g.purchase_blocker("shaft")})).unwrap()).unwrap();
        let checkpoint = Checkpoint {
            seed,
            style: style.into(),
            mode: mode.into(),
            content: content_fingerprint(),
            visit: visit + 1,
            wall,
            idle_visits,
            events: events.clone(),
            stalls: stalls.clone(),
            game: g.clone(),
        };
        let temporary = format!("target/campaign-checkpoint-{seed}.tmp");
        std::fs::write(&temporary, serde_json::to_vec(&checkpoint).unwrap()).unwrap();
        std::fs::rename(temporary, format!("target/campaign-checkpoint-{seed}.json")).unwrap();
        if std::env::var_os("DEEPWORK_CAMPAIGN_DIAGNOSTIC").is_some() {
            let _ = std::fs::write(
                format!("target/campaign-debug-{seed}.json"),
                serde_json::to_vec(&g).unwrap(),
            );
            eprintln!(
                "status={} nodes={} blockers={:?}",
                g.workings.status,
                g.workings.passages.len(),
                g.stages
                    .iter()
                    .map(|s| s.blocker.as_str())
                    .collect::<Vec<_>>()
            );
        }
        eprintln!(
            "seed={seed} strategy={style} mode={mode} visit={} depth={} next={:?} credits={} compute_seconds={}",
            visit + 1,
            g.depth(),
            g.pinned,
            g.credits,
            visit_started.elapsed().as_secs()
        );
    }
    if !g.megaproject {
        std::fs::write(
            format!("target/campaign-failed-{seed}.json"),
            serde_json::to_vec(&g).unwrap(),
        )
        .unwrap();
    }
    json!({"seed":seed,"content_fingerprint":content_fingerprint().to_string(),"resumed_visit":resumed_visit,"compute_seconds":run_started.elapsed().as_secs(),"strategy":style,"mode":mode,"stalls":stalls,"complete":g.megaproject,"events":events,"sites":g.site,"depth":g.depth(),"credits":g.credits,"next_upgrade":g.pinned,"purchase_blocker":g.pinned.as_ref().and_then(|id|g.purchase_blocker(id)),"products":g.products,"levels":g.levels,"blockers":g.stages.iter().map(|f|&f.blocker).collect::<Vec<_>>(),"expanded_state_bytes":serde_json::to_vec(&g).unwrap().len()})
}
fn median_seconds(sorted: &[u64]) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    Some((sorted[(sorted.len() - 1) / 2] as f64 + sorted[sorted.len() / 2] as f64) / 2.0)
}

fn progress_path() -> std::path::PathBuf {
    std::path::PathBuf::from("target/campaign-progress.json")
}

fn load_progress(
    path: &std::path::Path,
    seed_start: u64,
    seeds: u64,
    mode: &str,
) -> Result<Vec<Value>, String> {
    let backup = path.with_extension("json.bak");
    let bytes = std::fs::read(path)
        .or_else(|_| std::fs::read(&backup))
        .map_err(|error| format!("Could not read campaign progress: {error}"))?;
    let runs: Vec<Value> = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Could not decode campaign progress: {error}"))?;
    let mut completed = std::collections::BTreeSet::new();
    for run in &runs {
        let seed = run["seed"]
            .as_u64()
            .ok_or_else(|| "Campaign progress entry has no seed".to_string())?;
        if !(seed_start..seed_start + seeds).contains(&seed)
            || run["mode"] != mode
            || run["complete"] != true
            || run["content_fingerprint"] != content_fingerprint().to_string()
            || !completed.insert(seed)
        {
            return Err("Campaign progress does not match run parameters/content".into());
        }
    }
    Ok(runs)
}

fn persist_progress(path: &std::path::Path, runs: &[Value]) -> Result<(), String> {
    let temporary = path.with_extension("json.tmp");
    let backup = path.with_extension("json.bak");
    std::fs::write(&temporary, serde_json::to_vec_pretty(runs).unwrap())
        .map_err(|error| format!("Could not write campaign progress: {error}"))?;
    if path.exists() {
        std::fs::copy(path, &backup)
            .map_err(|error| format!("Could not back up campaign progress: {error}"))?;
        std::fs::remove_file(path)
            .map_err(|error| format!("Could not replace campaign progress: {error}"))?;
    }
    std::fs::rename(&temporary, path)
        .map_err(|error| format!("Could not commit campaign progress: {error}"))
}

fn main() {
    let started = std::time::Instant::now();
    mine_core::content::validate().unwrap();
    assert!(BUILD_ORDER
        .iter()
        .all(|id| requirements().iter().any(|u| u.id == *id)));
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
    let seed_start = args
        .get(5)
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(42);
    let workers = args
        .get(4)
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(4)
        .clamp(
            1,
            std::thread::available_parallelism().map_or(4, usize::from),
        );
    let resume = args.get(6).map(std::path::Path::new);
    let progress = progress_path();
    let mut runs = if resume.is_some()
        && (progress.exists() || progress.with_extension("json.bak").exists())
    {
        load_progress(&progress, seed_start, seeds, mode).unwrap()
    } else {
        Vec::new()
    };
    persist_progress(&progress, &runs).unwrap();
    let completed: std::collections::BTreeSet<_> =
        runs.iter().filter_map(|run| run["seed"].as_u64()).collect();
    let next = std::sync::atomic::AtomicU64::new(0);
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        for _ in 0..workers {
            let sender = sender.clone();
            let next = &next;
            scope.spawn(move || loop {
                let index = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if index >= seeds {
                    break;
                }
                if completed.contains(&(seed_start + index)) {
                    continue;
                }
                let style = ["bulk", "precision", "reclamation"][(seed_start + index) as usize % 3];
                sender
                    .send(run(seed_start + index, style, days, mode, resume))
                    .unwrap();
            });
        }
        drop(sender);
        for report in receiver {
            eprintln!(
                "Completed seed {}: headquarters={}",
                report["seed"], report["complete"]
            );
            runs.push(report);
            persist_progress(&progress, &runs).unwrap();
            let seed = runs.last().unwrap()["seed"].as_u64().unwrap();
            let _ = std::fs::remove_file(format!("target/campaign-state-{seed}.json"));
            let _ = std::fs::remove_file(format!("target/campaign-checkpoint-{seed}.json"));
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
        let median = median_seconds(&times);
        let early = window[1] <= 3600;
        let comparable = if early {
            mode == "continuous"
        } else {
            mode != "continuous"
        };
        medians.insert(name,json!({"median_seconds":median,"reached":times.len(),"target_seconds":window,"comparison_basis":if early { "continuous first-site play" } else { "two daily 12-minute visits" },"in_window":comparable.then(||median.is_some_and(|v|v>=window[0] as f64&&v<=window[1] as f64))}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"save_version":mine_core::VERSION,"generator_version":mine_core::geometry::GENERATOR_VERSION,"days":days,"compute_seconds":started.elapsed().as_secs(),"milestone_definitions":{"furnace":"first iron product","electrolysis_built":"electrolysis hall purchased","precision":"first aluminium product","rare_earth":"first permanent magnet product","headquarters":"megaproject delivered"},"runs":runs,"milestones":medians}))
            .unwrap()
    );
    if runs.iter().any(|run| run["complete"] != true) {
        eprintln!("Campaign acceptance incomplete: not every seed reached headquarters within {days} days; inspect blockers and still-progressing runs");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod strategy_tests {
    use super::*;
    #[test]
    fn report_medians_include_both_central_observations() {
        assert_eq!(median_seconds(&[]), None);
        assert_eq!(median_seconds(&[10]), Some(10.0));
        assert_eq!(median_seconds(&[10, 13]), Some(11.5));
        assert_eq!(median_seconds(&[10, 13, 30]), Some(13.0));
    }

    #[test]
    fn completed_progress_resumes_without_replaying_seed() {
        let directory =
            std::env::temp_dir().join(format!("deepwork-campaign-progress-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("progress.json");
        let runs = vec![json!({
            "seed": 42,
            "mode": "scheduled",
            "complete": true,
            "content_fingerprint": content_fingerprint().to_string(),
        })];
        persist_progress(&path, &runs).unwrap();
        assert_eq!(load_progress(&path, 42, 30, "scheduled").unwrap(), runs);
        assert!(load_progress(&path, 43, 30, "scheduled").is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }

    fn checkpoint_fixture() -> Checkpoint {
        let mut game = Game::new(42, 1);
        for _ in 0..60 {
            game.second(materials(), false);
        }
        game.last_saved = 1_043_200;
        Checkpoint {
            seed: 42,
            style: "bulk".into(),
            mode: "scheduled".into(),
            content: content_fingerprint(),
            visit: 1,
            wall: 43_200,
            idle_visits: 0,
            events: BTreeMap::new(),
            stalls: vec![],
            game,
        }
    }
    #[test]
    fn junk_sales_do_not_hide_a_missing_construction_feed() {
        let mut game = Game::new(42, 1);
        game.pinned = Some("electrolytic".into());
        game.credits = 10_000;
        let before = Advancement::read(&game);
        game.credits += 5_000;
        game.excavated += 10_000;
        game.products.insert("aggregate".into(), 640_000);
        assert!(!before.advanced_to(&Advancement::read(&game)));
        game.products.insert("insulation".into(), 64_000);
        assert!(before.advanced_to(&Advancement::read(&game)));
    }
    #[test]
    fn saving_for_equipment_and_deeper_access_are_progress() {
        let mut game = Game::new(42, 1);
        game.pinned = Some("electrolytic".into());
        let before = Advancement::read(&game);
        game.credits += 1;
        assert!(before.advanced_to(&Advancement::read(&game)));
        let before = Advancement::read(&game);
        game.heights.excavate_to(256, 400);
        assert!(before.advanced_to(&Advancement::read(&game)));
    }
    #[test]
    fn unrelated_deepening_does_not_hide_missing_aluminium_feed() {
        let mut game = Game::new(42, 1);
        game.pinned = Some("trace".into());
        game.heights
            .excavate_to(256, 1500 * mine_core::geometry::CELLS_PER_METRE);
        let before = Advancement::read(&game);
        game.heights
            .excavate_to(256, 7500 * mine_core::geometry::CELLS_PER_METRE);
        assert!(!before.advanced_to(&Advancement::read(&game)));
        game.products.insert("alumina".into(), 1);
        assert!(before.advanced_to(&Advancement::read(&game)));
    }
    #[test]
    fn revealed_required_source_extends_search_but_unknown_target_does_not() {
        use mine_core::geometry::{bit_index, chunk_id, CELLS_PER_METRE};
        let mut game = Game::new(42, 1);
        game.pinned = Some("trace".into());
        let target = [256, 2500 * CELLS_PER_METRE];
        game.workings.target = Some(target);
        game.heights.excavate_to(256, 1500 * CELLS_PER_METRE);
        let before = Advancement::read(&game);
        game.heights.excavate_to(256, 2000 * CELLS_PER_METRE);
        assert!(!before.advanced_to(&Advancement::read(&game)));
        let source = pinned_feeds(&game)[0];
        game.terrain
            .visible
            .entry(chunk_id(target[0], target[1]))
            .or_insert_with(|| vec![255; 4096])[bit_index(target[0], target[1])] = source as u8;
        let before = Advancement::read(&game);
        game.heights.excavate_to(256, 2200 * CELLS_PER_METRE);
        assert!(before.advanced_to(&Advancement::read(&game)));
    }
    #[test]
    fn resume_rejects_changed_content_clock_or_strategy() {
        let mut checkpoint = checkpoint_fixture();
        checkpoint.validate(42, "bulk", "scheduled").unwrap();
        assert!(checkpoint.validate(42, "precision", "scheduled").is_err());
        checkpoint.content ^= 1;
        assert!(checkpoint.validate(42, "bulk", "scheduled").is_err());
        checkpoint.content ^= 1;
        checkpoint.wall += 1;
        assert!(checkpoint.validate(42, "bulk", "scheduled").is_err());
    }
    #[test]
    fn resumed_checkpoint_continues_identical_simulation() {
        let mut original = checkpoint_fixture();
        let mut restored: Checkpoint =
            serde_json::from_slice(&serde_json::to_vec(&original).unwrap()).unwrap();
        restored.validate(42, "bulk", "scheduled").unwrap();
        for _ in 0..120 {
            original.game.tick(materials(), true);
            restored.game.tick(materials(), true);
        }
        assert_eq!(
            serde_json::to_value(&original.game).unwrap(),
            serde_json::to_value(&restored.game).unwrap()
        );
    }
    #[test]
    fn commissioned_industry_prioritises_trace_sources_while_lift_is_pinned() {
        let mut game = Game::new(42, 1);
        for id in BUILD_ORDER {
            game.levels.insert((*id).into(), 1);
        }
        game.pinned = Some("shaft".into());
        for product in [
            "advanced_structure",
            "precision_controls",
            "batteries",
            "iron",
            "borate",
        ] {
            game.products
                .insert(product.into(), 16 * mine_core::geometry::UNITS);
        }
        assert_eq!(pinned_feeds(&game), vec![53]);
        game.discoveries.insert(54);
        assert_eq!(pinned_feeds(&game), vec![54]);
        assert!(
            game.terrain.revealed.is_empty(),
            "Catalogue priorities must not expose hidden geology"
        );
        let before = Advancement::read(&game);
        for residue in ["neodymium_residue", "praseodymium_residue"] {
            game.trace_feed
                .insert(residue.into(), 2 * mine_core::geometry::UNITS);
        }
        assert!(
            pinned_feeds(&game).is_empty(),
            "Available residues already cover magnet inputs"
        );
        assert!(
            before.advanced_to(&Advancement::read(&game)),
            "Residue production prevents premature site abandonment"
        );
    }
    #[test]
    fn completed_headquarters_lines_release_shared_feeds_and_restart_when_short() {
        let mut g = Game::new(42, 2);
        g.credits = 0;
        for id in BUILD_ORDER {
            g.levels.insert((*id).into(), 1);
        }
        for product in ["magnets", "batteries", "precision_controls"] {
            g.products
                .insert(product.into(), 10 * mine_core::geometry::UNITS);
        }
        for id in [
            "magnets",
            "batteries",
            "controls",
            "separate_neodymium",
            "separate_praseodymium",
            "silicon",
            "separate_gallium",
        ] {
            g.enabled_recipes.insert(id.into());
        }
        strategy(&mut g, "bulk", false);
        for id in [
            "magnets",
            "batteries",
            "controls",
            "separate_neodymium",
            "separate_praseodymium",
            "silicon",
            "separate_gallium",
        ] {
            assert!(
                !g.enabled_recipes.contains(id),
                "Completed component still consumes feed: {id}"
            );
        }
        assert!(g.enabled_recipes.contains("structures"));
        assert!(!g.paused_recipes.contains("steel"));
        g.products
            .insert("magnets".into(), 9 * mine_core::geometry::UNITS);
        strategy(&mut g, "bulk", false);
        for id in [
            "magnets",
            "separate_neodymium",
            "separate_praseodymium",
            "structures",
        ] {
            assert!(
                g.enabled_recipes.contains(id),
                "Missing component must restart its supply chain: {id}"
            );
        }
        assert!(!g.enabled_recipes.contains("batteries"));
    }
    #[test]
    fn construction_shortages_use_known_recipe_feeds_without_reading_geology() {
        let mut g = Game::new(51, 2);
        g.credits = 0;
        g.heights.insert((0) as i64, 1192);
        for id in [
            "furnace",
            "wheelbarrow",
            "conveyor",
            "sorter",
            "survey",
            "capacity",
            "steelworks",
            "shaft",
        ] {
            g.levels.insert(id.into(), 1);
        }
        strategy(&mut g, "bulk", false);
        assert_eq!(g.pinned.as_deref(), Some("supports"));
        assert_eq!(g.policy, "vein");
        assert_eq!(g.priorities, vec![3, 5, 6]);
        assert!(
            g.terrain.revealed.is_empty(),
            "Planning priorities must not reveal ore"
        );
    }
    #[test]
    fn missing_feed_prompts_deeper_survey_until_an_exposed_sample_exists() {
        let mut g = Game::new(51, 2);
        g.credits = 0;
        g.heights.insert((0) as i64, 800);
        for id in [
            "furnace",
            "wheelbarrow",
            "conveyor",
            "sorter",
            "survey",
            "capacity",
            "steelworks",
            "shaft",
        ] {
            g.levels.insert(id.into(), 1);
        }
        strategy(&mut g, "bulk", false);
        assert_eq!(g.policy, "depth");
        // Supply the knowledge index an exposed sampled iron face would create.
        g.terrain
            .ore_frontiers
            .entry(3)
            .or_default()
            .insert(mine_core::geometry::cell_key(256, 800));
        strategy(&mut g, "bulk", false);
        assert_eq!(g.policy, "vein");
    }
    #[test]
    fn stocked_fuels_do_not_compete_with_missing_construction_iron() {
        let mut g = Game::new(51, 2);
        g.pinned = Some("supports".into());
        g.levels.insert("shaft".into(), 1);
        for (product, units) in [("iron", 6), ("coke", 16), ("lime", 16)] {
            g.products
                .insert(product.into(), units * mine_core::geometry::UNITS);
        }
        assert_eq!(pinned_feeds(&g), vec![3]);
        g.products
            .insert("iron".into(), 10 * mine_core::geometry::UNITS);
        assert!(pinned_feeds(&g).is_empty());
        g.products.insert("coke".into(), 0);
        assert_eq!(pinned_feeds(&g), vec![5]);
    }
    #[test]
    fn research_retirement_access_temporarily_overrides_extraction_style() {
        let mut g = Game::new(42, 2);
        g.credits = 0;
        g.heights.insert((0) as i64, 800);
        for id in BUILD_ORDER.iter().take_while(|id| **id != "power") {
            g.levels.insert((*id).into(), 1);
        }
        g.products
            .insert("steel".into(), 16 * mine_core::geometry::UNITS);
        strategy(&mut g, "bulk", false);
        assert_eq!(g.policy, "depth");
        g.ranks.insert("excavation".into(), 3);
        strategy(&mut g, "bulk", false);
        assert_eq!(
            g.policy, "bulk",
            "Return to chosen style after funding research gate"
        );
    }
    #[test]
    fn early_contracts_reward_research_without_spending_endgame_reserves() {
        let mut g = Game::default();
        for c in &g.contracts {
            g.products.insert(c.product.clone(), c.amount);
        }
        strategy(&mut g, "bulk", false);
        assert!(g.site_objectives.len() >= 2);
        assert_eq!(g.level("pump"), 0);
        let mut held = Game::default();
        held.contracts[0].product = "magnets".into();
        held.contracts[0].amount = mine_core::geometry::UNITS;
        held.products
            .insert("magnets".into(), 10 * mine_core::geometry::UNITS);
        held.reserve
            .insert("magnets".into(), 10 * mine_core::geometry::UNITS);
        strategy(&mut held, "bulk", false);
        assert!(!held.contracts[0].complete);
        assert_eq!(held.products["magnets"], 10 * mine_core::geometry::UNITS);
    }
    #[test]
    fn first_electrolysis_output_precedes_research_retirement() {
        let mut g = Game::new(42, 2);
        g.heights.insert((0) as i64, 1200);
        g.steel_made = true;
        g.credits = 0;
        g.ranks.insert("excavation".into(), 3);
        g.ranks.insert("metallurgy".into(), 5);
        for id in BUILD_ORDER.iter().take_while(|id| **id != "trace") {
            g.levels.insert((*id).into(), 1);
        }
        strategy(&mut g, "bulk", false);
        assert_eq!(
            g.site, 2,
            "Do not dismantle an uncommissioned first electrolysis line"
        );
        g.collection.insert("aluminium".into());
        strategy(&mut g, "bulk", false);
        assert_eq!(
            g.site, 3,
            "Resume voluntary research retirement after first output"
        );
    }
    #[test]
    fn public_surveyed_cells_can_direct_a_whole_vein_before_exposure() {
        let mut g = Game::new(42, 1);
        let cat = materials();
        let p = (210..260)
            .map(|x| [x, 24])
            .find(|p| g.cell(p[0], p[1], cat) == 3)
            .unwrap();
        direct_known_feed(&mut g, &[3], 1200);
        assert!(
            g.workings.target.is_none(),
            "hidden reserves cannot be targeted"
        );
        g.terrain.reveal(g.seed, g.profile, p[0], p[1], 0, cat);
        assert!(!g
            .terrain
            .frontier
            .contains(&mine_core::geometry::cell_key(p[0], p[1])));
        direct_known_feed(&mut g, &[3], 1200);
        assert_eq!(g.workings.target, Some(p));
        assert!(g.workings.target_deposit.is_some());
        direct_known_feed(&mut g, &[], 1200);
        assert!(
            g.workings.target.is_none(),
            "fulfilled investment releases its order"
        );
    }
    #[test]
    fn scheduled_decisions_preserve_required_whole_vein_policy_between_surveys() {
        let mut g = Game::new(42, 2);
        g.credits = 0;
        for id in [
            "furnace",
            "wheelbarrow",
            "conveyor",
            "sorter",
            "survey",
            "capacity",
            "steelworks",
            "shaft",
        ] {
            g.levels.insert(id.into(), 1);
        }
        let p = (210..260)
            .map(|x| [x, 24])
            .find(|p| g.cell(p[0], p[1], materials()) == 3)
            .unwrap();
        g.terrain
            .reveal(g.seed, g.profile, p[0], p[1], 0, materials());
        g.pinned = Some("supports".into());
        direct_known_feed(&mut g, &[3], 1200);
        g.ticks = 100; // Between the scheduled 30-second deposit decisions.
        strategy(&mut g, "bulk", false);
        assert_eq!(g.workings.target, Some(p));
        assert_eq!(g.policy, "vein");
    }
    #[test]
    fn first_visit_funds_processing_before_stockpiling_late_industry_inputs() {
        let mut g = Game::default();
        let cat = materials();
        for second in 0..720 {
            if second % 5 == 0 {
                strategy(&mut g, "bulk", false);
            }
            g.second(&cat, false);
        }
        assert!(g.level("conveyor") > 0);
        assert!(g.level("furnace") > 0);
        assert!(g.collection.contains("iron"));
    }
}
