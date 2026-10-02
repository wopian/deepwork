pub mod columns;
pub mod content;
pub mod crew;
pub mod fronts;
pub mod geology;
pub mod geometry;
pub mod logistics;
pub mod navigation;
pub mod pacing;
pub mod terrain;
pub mod transport;
pub mod waste;
pub mod workings;
pub use geometry::WIDTH;
use geometry::{CELL_MASS, UNITS};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
pub const VERSION: u32 = 12;
#[cfg(test)]
mod geology_version_tests {
    use super::*;
    #[test]
    fn legacy_migrations_and_reload_preserve_public_and_hidden_geology() {
        for save_version in [10, 11, 12] {
            let mut game = Game::new(42, 1);
            game.generator_version = 5;
            game.version = save_version;
            game.terrain
                .reveal_versioned(42, 0, 5, 256, 480, 12, materials());
            let terrain = serde_json::to_value(&game.terrain).unwrap();
            let before: Vec<_> = (-64..600)
                .step_by(17)
                .map(|x| game.cell(x, 490, materials()))
                .collect();
            let mut loaded: Game =
                serde_json::from_slice(&serde_json::to_vec(&game).unwrap()).unwrap();
            loaded.migrate().unwrap();
            assert_eq!(loaded.generator_version, 5);
            assert_eq!(serde_json::to_value(&loaded.terrain).unwrap(), terrain);
            assert_eq!(
                (-64..600)
                    .step_by(17)
                    .map(|x| loaded.cell(x, 490, materials()))
                    .collect::<Vec<_>>(),
                before
            );
        }
    }
    #[test]
    fn new_mine_surveys_orders_and_reload_use_generator_six() {
        let mut game = Game::new(42, 1);
        assert_eq!(game.generator_version, 6);
        let anchor = (32..480)
            .flat_map(|x| (472..=488).map(move |y| [x, y]))
            .find(|p| game.cell(p[0], p[1], materials()) == 19)
            .unwrap();
        game.terrain.reveal_versioned(
            game.seed,
            game.profile,
            game.generator_version,
            anchor[0],
            anchor[1],
            8,
            materials(),
        );
        assert_eq!(game.terrain.known_material(anchor[0], anchor[1]), Some(19));
        game.workings.target = Some(anchor);
        game.workings.prepare_deposit_order_versioned(
            game.seed,
            game.profile,
            game.generator_version,
            materials(),
        );
        assert!(game
            .workings
            .target_deposit
            .as_ref()
            .unwrap()
            .starts_with("6:"));
        assert!(game.workings.target_cells.len() > 1500);
        let mut loaded: Game = serde_json::from_slice(&serde_json::to_vec(&game).unwrap()).unwrap();
        loaded.migrate().unwrap();
        assert_eq!(loaded.generator_version, 6);
        // Target geometry is a transient cache; rebuild from persisted anchor.
        loaded.workings.prepare_deposit_order_versioned(
            loaded.seed,
            loaded.profile,
            loaded.generator_version,
            materials(),
        );
        assert!(loaded.workings.target_cells == game.workings.target_cells);
        for (&chunk, pixels) in &loaded.terrain.visible {
            let (x, y) = geometry::chunk_origin(chunk);
            for (index, id) in pixels.iter().enumerate().filter(|(_, id)| **id != 255) {
                assert_eq!(
                    loaded.cell(
                        x + (index % 64) as i64,
                        y + (index / 64) as i64,
                        materials()
                    ),
                    *id as usize
                );
            }
        }
    }
}
/// Deterministic fractional throughput without storing idle production credit.
/// `rate` is thousandths of one work unit per tick; no multiplication by full age.
fn work_budget(rate: u64, tick: u64) -> u64 {
    let fraction = rate % 1000;
    rate / 1000 + u64::from((tick % 1000 * fraction) % 1000 < fraction)
}
#[derive(Clone, Serialize)]
pub struct UpgradePreview {
    pub machine_percent: f64,
    pub line_percent: f64,
    pub lift_depth_after: Option<u32>,
    pub access_depth_after: Option<u32>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Material {
    pub id: usize,
    pub name: String,
    pub product: String,
    pub family: String,
    pub tier: u32,
    pub price: u64,
    pub color: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub id: String,
    pub building: String,
    pub inputs: BTreeMap<String, u64>,
    pub output: String,
}
pub fn recipes() -> &'static [Recipe] {
    static CONTENT: std::sync::OnceLock<Vec<Recipe>> = std::sync::OnceLock::new();
    CONTENT.get_or_init(|| {
        let mut entries: Vec<Recipe> =
            serde_json::from_str(include_str!("../../../content/recipes.json"))
                .expect("valid recipes");
        for recipe in &mut entries {
            for amount in recipe.inputs.values_mut() {
                *amount *= 64;
            }
        }
        entries
    })
}
#[derive(Clone, Serialize, Deserialize)]
pub struct SiteProfile {
    pub name: String,
    pub focus: Vec<usize>,
    pub hardness: f64,
    pub haul: f64,
    pub description: String,
}
pub fn sites() -> &'static [SiteProfile] {
    static SITES: std::sync::OnceLock<Vec<SiteProfile>> = std::sync::OnceLock::new();
    SITES.get_or_init(|| {
        serde_json::from_str(include_str!("../../../content/sites.json")).expect("valid sites")
    })
}
#[derive(Clone, Serialize, Deserialize)]
pub struct TraceRule {
    pub feed: usize,
    pub output: String,
    pub permille: u64,
}
pub fn traces() -> &'static [TraceRule] {
    static RULES: std::sync::OnceLock<Vec<TraceRule>> = std::sync::OnceLock::new();
    RULES.get_or_init(|| {
        serde_json::from_str(include_str!("../../../content/traces.json")).expect("valid traces")
    })
}
#[derive(Clone, Serialize, Deserialize)]
pub struct UpgradeRequirement {
    pub id: String,
    #[serde(default)]
    pub research_points: u64,
    pub requires: String,
    pub inputs: BTreeMap<String, u64>,
}
pub fn requirements() -> &'static [UpgradeRequirement] {
    static DATA: std::sync::OnceLock<Vec<UpgradeRequirement>> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        let mut entries: Vec<UpgradeRequirement> =
            serde_json::from_str(include_str!("../../../content/upgrades.json"))
                .expect("valid upgrade requirements");
        for entry in &mut entries {
            for amount in entry.inputs.values_mut() {
                *amount *= 64;
            }
        }
        entries
    })
}
pub fn materials() -> &'static [Material] {
    static DATA: std::sync::OnceLock<Vec<Material>> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../../../content/materials.json"))
            .expect("valid material catalogue")
    })
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Cell {
    pub x: i64,
    pub y: i64,
    pub material: usize,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Contract {
    pub product: String,
    pub amount: u64,
    pub complete: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Record {
    #[serde(default)]
    pub section: Vec<u8>,
    pub site: u32,
    pub depth: u32,
    pub research: u64,
    pub excavated: u64,
}
#[derive(Clone, Serialize)]
pub struct ProcessingFeed {
    pub id: usize,
    pub output: String,
    pub intake: u64,
    pub stored: u64,
    pub transit: u64,
    pub buffered: u64,
    pub queued: u64,
    pub product: u64,
    pub reserve_target: u64,
    pub reserved: u64,
    pub input_rate: f64,
    pub output_rate: f64,
    pub recovery_percent: u32,
    pub blocker: String,
    pub destination: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Game {
    pub version: u32,
    pub campaign_id: String,
    pub generator_version: u32,
    #[serde(skip)]
    pub legacy_pending: bool,
    #[serde(with = "decimal")]
    pub seed: u64,
    pub site: u32,
    #[serde(default)]
    pub profile: usize,
    #[serde(default)]
    pub challenge: String,
    pub ticks: u64,
    #[serde(with = "decimal")]
    pub credits: u64,
    pub workers: u32,
    #[serde(default)]
    pub movement: crew::Movement,
    #[serde(skip)]
    crew_job_cache: Vec<crew::Job>,
    #[serde(skip)]
    crew_job_stamp: Option<u64>,
    #[serde(skip)]
    front_candidate_cache: fronts::CandidateCache,
    pub housing: u32,
    pub levels: BTreeMap<String, u32>,
    pub research: u64,
    pub ranks: BTreeMap<String, u32>,
    pub policy: String,
    #[serde(default)]
    pub specialisation: Option<String>,
    pub priorities: Vec<usize>,
    pub reserve: BTreeMap<String, u64>,
    pub pinned: Option<String>,
    pub heights: columns::ColumnDepths,
    #[serde(default)]
    pub terrain: terrain::Terrain,
    #[serde(default)]
    pub workings: workings::Workings,
    #[serde(default)]
    pub mining_fronts: Vec<fronts::MiningFront>,
    pub removed: Vec<Cell>,
    pub ore: BTreeMap<usize, u64>,
    pub hauled: BTreeMap<usize, u64>,
    #[serde(default)]
    pub concentrate: BTreeMap<usize, u64>,
    #[serde(default)]
    pub raw_stock: BTreeMap<usize, u64>,
    #[serde(default)]
    pub flow_window: [u64; 5],
    #[serde(default)]
    pub processing_window: BTreeMap<usize, [u64; 2]>,
    #[serde(default)]
    pub trace_feed: BTreeMap<String, u64>,
    #[serde(default)]
    pub trace_fraction: BTreeMap<String, u64>,
    #[serde(default)]
    pub transport: transport::Network,
    #[serde(skip)]
    haul_path: Vec<[i64; 2]>,
    #[serde(skip)]
    haul_legs: Vec<logistics::Leg>,
    #[serde(skip)]
    haul_levels: BTreeMap<String, u32>,
    #[serde(skip)]
    haul_route_key: Option<([i64; 2], bool)>,
    #[serde(default)]
    pub crew: logistics::Crew,
    #[serde(default)]
    pub crew_priority: String,
    #[serde(default)]
    pub cargo_policy: String,
    pub products: BTreeMap<String, u64>,
    pub tailings: BTreeMap<usize, u64>,
    #[serde(default)]
    pub recovery_fraction: BTreeMap<usize, u64>,
    pub slag: u64,
    pub depleted: u64,
    #[serde(default)]
    pub disposed_mass: u64,
    pub lifetime_waste: u64,
    #[serde(default)]
    pub waste_profile: waste::WasteProfile,
    pub excavated: u64,
    pub discoveries: BTreeSet<usize>,
    #[serde(default)]
    pub collection: BTreeSet<String>,
    pub site_discoveries: u32,
    pub contracts: Vec<Contract>,
    #[serde(default)]
    pub site_objectives: BTreeSet<String>,
    #[serde(skip)]
    pub retirement_quote: Option<u64>,
    pub records: Vec<Record>,
    pub last_saved: u64,
    pub last_sequence: u64,
    pub stages: Vec<Stage>,
    pub offline: Option<Offline>,
    pub steel_made: bool,
    pub megaproject: bool,
    #[serde(default)]
    pub milestones: BTreeSet<String>,
    #[serde(default)]
    pub blueprint: Vec<String>,
    #[serde(default)]
    pub build_queue: Vec<String>,
    #[serde(default)]
    pub enabled_recipes: BTreeSet<String>,
    #[serde(default)]
    pub paused_recipes: BTreeSet<String>,
    #[serde(default)]
    pub sold_mass: u64,
    #[serde(default)]
    pub delivered_mass: u64,
    #[serde(default)]
    pub dig_progress: u64,
    #[serde(default)]
    pub dig_remainder: u64,
    #[serde(default)]
    pub credit_fraction: u64,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Stage {
    pub name: String,
    pub rate: f64,
    pub buffer: u64,
    pub capacity: u64,
    pub blocker: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Offline {
    /// Stable identity of the committed away interval, independent of UI lifetime.
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub discoveries: Vec<usize>,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub capped: u64,
    pub elapsed: u64,
    pub effective: u64,
    pub credits: String,
    pub excavated: u64,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Action {
    pub sequence: u64,
    pub kind: String,
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub value: u64,
}
mod decimal {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&v.to_string())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Decimal(String),
            Legacy(u64),
        }
        match Wire::deserialize(d)? {
            Wire::Decimal(text) => text.parse().map_err(serde::de::Error::custom),
            Wire::Legacy(value) => Ok(value),
        }
    }
}
impl Default for Game {
    fn default() -> Self {
        Self::new(73429, 1)
    }
}
impl Game {
    pub fn new(seed: u64, site: u32) -> Self {
        Self {
            version: VERSION,
            campaign_id: format!("{seed:016x}-{site}"),
            generator_version: geometry::GENERATOR_VERSION,
            legacy_pending: false,
            seed,
            site,
            profile: 0,
            challenge: String::new(),
            ticks: 0,
            credits: pacing::get().starting_credits,
            workers: 3,
            movement: crew::Movement::new(3),
            crew_job_cache: vec![],
            crew_job_stamp: None,
            front_candidate_cache: fronts::CandidateCache::default(),
            housing: 8,
            levels: BTreeMap::new(),
            research: 0,
            ranks: BTreeMap::new(),
            policy: "vein".into(),
            specialisation: None,
            priorities: vec![],
            reserve: BTreeMap::new(),
            pinned: Some("furnace".into()),
            heights: columns::ColumnDepths::default(),
            terrain: terrain::Terrain::default(),
            workings: workings::Workings::default(),
            mining_fronts: vec![],
            removed: vec![],
            ore: BTreeMap::new(),
            hauled: BTreeMap::new(),
            concentrate: BTreeMap::new(),
            raw_stock: BTreeMap::new(),
            flow_window: [0; 5],
            processing_window: BTreeMap::new(),
            trace_feed: BTreeMap::new(),
            trace_fraction: BTreeMap::new(),
            transport: transport::Network::default(),
            haul_path: vec![],
            haul_legs: vec![],
            haul_levels: BTreeMap::new(),
            haul_route_key: None,
            crew: logistics::Crew::assign(3, &BTreeMap::new()),
            crew_priority: String::new(),
            cargo_policy: String::new(),
            products: BTreeMap::new(),
            tailings: BTreeMap::new(),
            recovery_fraction: BTreeMap::new(),
            slag: 0,
            depleted: 0,
            disposed_mass: 0,
            lifetime_waste: 0,
            waste_profile: waste::WasteProfile::default(),
            excavated: 0,
            discoveries: BTreeSet::new(),
            collection: BTreeSet::new(),
            site_discoveries: 0,
            site_objectives: BTreeSet::new(),
            retirement_quote: None,
            contracts: vec![
                Contract {
                    product: "iron".into(),
                    amount: 2 * UNITS,
                    complete: false,
                },
                Contract {
                    product: "copper".into(),
                    amount: 2 * UNITS,
                    complete: false,
                },
                Contract {
                    product: "steel".into(),
                    amount: UNITS,
                    complete: false,
                },
            ],
            records: vec![],
            last_saved: 0,
            last_sequence: 0,
            stages: vec![],
            offline: None,
            steel_made: false,
            megaproject: false,
            milestones: BTreeSet::new(),
            blueprint: vec![],
            build_queue: vec![],
            enabled_recipes: BTreeSet::new(),
            paused_recipes: BTreeSet::new(),
            sold_mass: 0,
            delivered_mass: 0,
            dig_progress: 0,
            dig_remainder: 0,
            credit_fraction: 0,
        }
    }
    pub fn level(&self, k: &str) -> u32 {
        *self.levels.get(k).unwrap_or(&0)
    }
    pub fn depth(&self) -> u32 {
        geometry::depth(self.heights.deepest_row())
    }
    pub fn cost(&self, k: &str) -> u64 {
        let p = pacing::get();
        // Rebuilding known equipment offsets repeated crew mobilisation while
        // preserving the authored first-site purchase window.
        let base = (if self.site > 1 {
            p.rebuild_costs.get(k).or_else(|| p.costs.get(k))
        } else {
            p.costs.get(k)
        })
        .copied()
        .unwrap_or(100) as f64;
        let (growth, n) = match k {
            "worker" => (p.worker_growth, self.workers.saturating_sub(3)),
            "housing" => (p.capacity_growth, self.housing.saturating_sub(8) / 4),
            "capacity" => (p.capacity_growth, self.level(k)),
            _ => (p.machine_growth, self.level(k)),
        };
        (base * growth.powi(n as i32)).ceil() as u64
    }
    pub fn lift_depth_limit(&self) -> u32 {
        300 + self.level("shaft") * pacing::get().lift_depth_per_level
    }
    pub fn equipment_depth_limit(&self) -> u32 {
        self.lift_depth_limit().min(if self.level("supports") == 0 {
            300
        } else if self.level("pump") == 0 {
            700
        } else if self.level("ventilation") == 0 {
            1500
        } else {
            200000
        })
    }
    pub fn research_invested(&self) -> u64 {
        self.ranks
            .values()
            .map(|&r| {
                let r = r as u64;
                pacing::get().research_base * r * (r + 1) * (2 * r + 1) / 6
            })
            .sum()
    }
    pub fn purchase_blocker(&self, id: &str) -> Option<String> {
        let Some(u) = requirements().iter().find(|u| u.id == id) else {
            return Some("Unknown upgrade".into());
        };
        if self.research_invested() < u.research_points {
            return Some(format!(
                "Requires {} headquarters research invested",
                u.research_points
            ));
        }
        if id == "worker" && self.workers >= self.housing {
            return Some("Build more housing".into());
        }
        if self.level(id) >= 50 {
            return Some("Maximum level".into());
        }
        if !u.requires.is_empty() && self.level(&u.requires) == 0 {
            return Some(format!("Requires {}", u.requires));
        }
        if let Some((p, n)) = u
            .inputs
            .iter()
            .find(|(p, n)| self.products.get(*p).copied().unwrap_or(0) < **n)
        {
            return Some(format!("Needs {} {}", *n as f64 / UNITS as f64, p));
        }
        if self.credits < self.cost(id) {
            return Some("More credits required".into());
        }
        None
    }
    pub fn raw_stock_capacity(&self) -> u64 {
        (128 + 16 * self.level("capacity") as u64) * UNITS
    }
    fn reserved_feed_target(&self, id: usize, m: &Material) -> u64 {
        let starter = if matches!(id, 3 | 5 | 6) {
            pacing::get().starter_hold_units
                * UNITS
                * if id == 3 {
                    pacing::get().starter_iron_multiplier
                } else {
                    1
                }
        } else {
            0
        };
        starter.max(
            self.reserve
                .get(&m.product)
                .copied()
                .unwrap_or(0)
                .saturating_mul(2),
        )
    }
    pub fn product_hold(&self, p: &str) -> u64 {
        let recipe_hold = recipes().iter().any(|r| {
            self.levels.get(&r.building).copied().unwrap_or(0) > 0
                && !self.paused_recipes.contains(&r.id)
                && (r.id == "steel" || r.id == "aluminium" || self.enabled_recipes.contains(&r.id))
                && r.inputs.contains_key(p)
        });
        // While the furnace is pinned, retain its first construction output.
        // Explicitly unpinning releases these provisional reserves.
        let progression_hold = if matches!(self.pinned.as_deref(), Some("furnace" | "steelworks")) {
            match p {
                "iron" => 8 * UNITS,
                "coke" | "lime" => 2 * UNITS,
                _ => 0,
            }
        } else {
            0
        };
        let foundation_hold = if p == "steel" && self.pinned.is_some() {
            requirements()
                .iter()
                .filter(|u| {
                    pacing::get().foundation_upgrades.contains(&u.id)
                        && self.levels.get(&u.id).copied().unwrap_or(0) == 0
                })
                .filter_map(|u| u.inputs.get("steel"))
                .sum()
        } else {
            0
        };
        let reserved = progression_hold.max(foundation_hold).max(
            self.reserve
                .get(p)
                .copied()
                .unwrap_or(0)
                .max(if recipe_hold { 8 * UNITS } else { 0 })
                .max(
                    self.pinned
                        .as_ref()
                        .and_then(|id| requirements().iter().find(|u| u.id == *id))
                        .and_then(|u| u.inputs.get(p))
                        .copied()
                        .unwrap_or(0),
                ),
        );
        let contract_hold = self
            .contracts
            .iter()
            .filter(|c| !c.complete && c.product == *p)
            .map(|c| c.amount)
            .sum::<u64>();
        reserved.max(contract_hold)
    }
    pub fn processing(&self) -> Vec<ProcessingFeed> {
        let seconds = ((self.ticks.saturating_sub(1)) % 20 + 1) as f64 / 20.;
        materials()
            .iter()
            .filter_map(|m| {
                let id = m.id;
                let output = if m.name == "Bauxite" {
                    "alumina"
                } else {
                    &m.product
                };
                let input = self.concentrate.get(&id).copied().unwrap_or(0);
                let stock = self.raw_stock.get(&id).copied().unwrap_or(0);
                let transit = self
                    .transport
                    .segments
                    .iter()
                    .flat_map(|s| &s.batches)
                    .filter(|b| b.material == id)
                    .map(|b| b.amount)
                    .sum::<u64>();
                let buffered = self.ore.get(&id).copied().unwrap_or(0)
                    + self
                        .mining_fronts
                        .iter()
                        .map(|front| {
                            front.stockpiles.get(&id).copied().unwrap_or_else(|| {
                                u64::from(front.stockpiles.is_empty() && front.material == id)
                                    * front.stockpile
                            })
                        })
                        .sum::<u64>()
                    + self
                        .transport
                        .stations
                        .iter()
                        .map(|s| s.cargo.get(&id).copied().unwrap_or(0))
                        .sum::<u64>();
                let queued = self.hauled.get(&id).copied().unwrap_or(0);
                let product = self.products.get(output).copied().unwrap_or(0);
                let flow = self.processing_window.get(&id).copied().unwrap_or_default();
                if input + stock + transit + buffered + queued + product + flow[0] == 0 {
                    return None;
                }
                let hold = self.product_hold(output);
                let unlocked = self.feed_unlocked(m);
                Some(ProcessingFeed {
                    id,
                    output: output.into(),
                    intake: input,
                    stored: stock,
                    transit,
                    buffered,
                    queued,
                    product,
                    reserve_target: hold,
                    reserved: product.min(hold),
                    input_rate: flow[0] as f64 / UNITS as f64 / seconds,
                    output_rate: flow[1] as f64 / UNITS as f64 / seconds,
                    recovery_percent: (65i32
                        + 3 * self.level("recovery") as i32
                        + if self.ranks.get("metallurgy").copied().unwrap_or(0) >= 6 {
                            5
                        } else {
                            0
                        }
                        + if self.ranks.get("reclamation").copied().unwrap_or(0) >= 6 {
                            5
                        } else {
                            0
                        }
                        + match self.specialisation.as_deref() {
                            Some("bulk") => -5,
                            Some("precision") => 10,
                            _ => 0,
                        })
                    .clamp(0, 95) as u32,
                    blocker: if !unlocked {
                        match m.family.as_str() {
                            "furnace" | "industrial" => "Build furnace",
                            "sulfide" | "chemical" | "electrolytic" => "Build chemical refinery",
                            _ => "Build separation hall",
                        }
                    } else if flow[0] > 0 {
                        "Processing"
                    } else if input > 0 {
                        "Waiting for refinery capacity"
                    } else if stock > 0 {
                        "Stored feed awaiting sorting capacity"
                    } else if buffered > 0 {
                        "Ore waiting in route stockpiles"
                    } else if transit + queued > 0 {
                        "Feed arriving"
                    } else {
                        "Waiting for ore"
                    }
                    .into(),
                    destination: if hold > 0 {
                        "Reserve, then sell surplus"
                    } else {
                        "Sell refined output"
                    }
                    .into(),
                })
            })
            .collect()
    }
    fn feed_unlocked(&self, m: &Material) -> bool {
        match m.family.as_str() {
            "physical" => true,
            "furnace" | "industrial" => self.level("furnace") > 0,
            "sulfide" | "chemical" | "electrolytic" => self.level("chemical") > 0,
            _ => self.level("trace") > 0,
        }
    }
    fn power_factor(&self, _offline: bool) -> f64 {
        let p = pacing::get();
        let industry = (p.base_power
            + u32::from(self.level("furnace") > 0) * p.furnace_demand
            + p.chemical_demand * self.level("chemical")
            + p.electrolysis_demand * self.level("electrolytic")
            + p.trace_demand * self.level("trace")) as f64;
        let industry = industry
            * if self.ranks.get("metallurgy").copied().unwrap_or(0) >= 10 {
                0.8
            } else {
                1.
            };
        let logistics: u32 = self.transport.segments.iter().map(|s| s.demand).sum();
        ((p.base_power + p.power_per_level * self.level("power")) as f64
            / (industry + logistics as f64))
            .min(1.)
    }
    fn capacity_rates(&mut self) -> [f64; 4] {
        self.crew = logistics::Crew::prioritise(self.workers, &self.levels, &self.crew_priority);
        let factor = if self.challenge == "long_haul" {
            1.5
        } else {
            1.
        } / sites()[self.profile].haul;
        let (legs, _) = logistics::route(&self.haul_path, &self.levels, factor);
        let rate = self.branch_rate(self.crew.haulers) as f64 / UNITS as f64;
        let buffer_level = self.level("capacity")
            + u32::from(self.ranks.get("logistics").copied().unwrap_or(0) >= 6) * 2;
        self.transport
            .configure(&legs, (rate * UNITS as f64) as u64, buffer_level);
        let power = self.power_factor(true);
        let haul = self
            .transport
            .segments
            .iter()
            .map(|segment| {
                let speed = if segment.demand > 0 { power } else { 1. };
                rate.min(
                    segment.capacity as f64 / UNITS as f64 * 1000. * speed
                        / segment.duration_ms as f64,
                )
            })
            .fold(rate, f64::min);
        [
            self.dig_rate(true) as f64 / UNITS as f64,
            haul,
            pacing::get().sorting_rate * self.throughput("sorter"),
            pacing::get().refining_rate
                * self.throughput("furnace")
                * (1. + 0.15 * self.crew.operators.saturating_sub(1) as f64)
                * (1. + 0.04 * self.ranks.get("metallurgy").copied().unwrap_or(0) as f64)
                * power
                * if self.specialisation.as_deref() == Some("reclamation") {
                    0.85
                } else {
                    1.
                },
        ]
    }
    pub fn upgrade_previews(&self) -> BTreeMap<String, UpgradePreview> {
        // Forecast only rate inputs; never clone the full terrain/stockpile per offer.
        let mut model = Game::default();
        model.levels = self.levels.clone();
        model.ranks = self.ranks.clone();
        model.workers = self.workers;
        model.heights = self.heights.clone();
        model.crew_priority = self.crew_priority.clone();
        model.specialisation = self.specialisation.clone();
        model.challenge = self.challenge.clone();
        model.profile = self.profile;
        model.haul_path = self.haul_path.clone();
        let before = model.capacity_rates();
        let line = before.into_iter().fold(f64::INFINITY, f64::min);
        let mut result = BTreeMap::new();
        for (id, stage) in [
            ("worker", 0),
            ("drill", 0),
            ("conveyor", 1),
            ("sorter", 2),
            ("furnace", 3),
            ("power", 3),
            ("capacity", 1),
            ("shaft", 1),
            ("minecart", 1),
            ("train", 1),
        ] {
            if self.level(id) >= 50 {
                continue;
            }
            let mut next = model.clone();
            if id == "worker" {
                next.workers += 1;
            } else {
                *next.levels.entry(id.into()).or_default() += 1;
            }
            let after = next.capacity_rates();
            let total = after.into_iter().fold(f64::INFINITY, f64::min);
            result.insert(
                id.into(),
                UpgradePreview {
                    lift_depth_after: (id == "shaft").then(|| next.lift_depth_limit()),
                    access_depth_after: (id == "shaft").then(|| next.equipment_depth_limit()),
                    machine_percent: if before[stage] > 0. {
                        100. * (after[stage] / before[stage] - 1.)
                    } else {
                        0.
                    },
                    line_percent: if line > 0. {
                        100. * (total / line - 1.)
                    } else {
                        0.
                    },
                },
            );
        }
        result
    }
    pub fn award(&self) -> u64 {
        (10. * (self.depth() as f64 / 300.).sqrt()).floor() as u64
            + 3 * self.site_discoveries as u64
            + 5 * self.site_objectives.len() as u64
    }
    pub fn cell(&self, x: i64, y: i64, cat: &[Material]) -> usize {
        geology::sample_versioned(self.seed, self.profile, self.generator_version, x, y, cat)
    }
    pub fn rock_work(&self) -> f64 {
        let depth = self.depth();
        let boundaries = [0, 100, 300, 700, 1500, 3000, 6000];
        let band = (0..6).find(|i| depth < boundaries[i + 1]).unwrap_or(5);
        let progress = ((depth - boundaries[band]) as f64
            / (boundaries[band + 1] - boundaries[band]) as f64)
            .min(1.);
        2.5f64.powi(band as i32) * (1. + progress) / 3f64.powi((self.level("drill") / 10) as i32)
    }
    fn throughput(&self, k: &str) -> f64 {
        let n = self.level(k);
        (1. + 0.12 * n as f64) * 1.5f64.powi((n / 10) as i32)
    }
    pub fn travel_power(&self) -> u32 {
        (self.power_factor(false) * 1000.) as u32
    }
    fn surveying_crew(&self) -> u32 {
        let target = self
            .workings
            .passages
            .get(self.workings.active)
            .map(|passage| passage.feet)
            .unwrap_or([WIDTH / 2, 0]);
        self.movement
            .workers
            .iter()
            .filter(|worker| {
                worker.job == "survey"
                    && worker.route == 0
                    && worker.position == target
                    && worker.activity == "surveying"
            })
            .count() as u32
    }
    fn support_crew(&self) -> u32 {
        let position = crew::access_position(&self.terrain, &self.workings);
        self.movement
            .workers
            .iter()
            .filter(|p| {
                p.route == 0
                    && p.position == position
                    && p.activity == "building"
                    && (p.job == "support" || (self.crew.engineers == 0 && p.job == "access"))
            })
            .count() as u32
    }
    fn move_crew(&mut self, access: u32) {
        self.movement.initialise(self.workers);
        use std::hash::{Hash, Hasher};
        let mut stamp = std::collections::hash_map::DefaultHasher::new();
        (
            self.terrain.revision,
            self.workings.revision,
            self.workings.active,
            access,
            self.crew.engineers,
            self.crew.prospectors,
            self.crew.haulers,
            self.crew.operators,
            self.crew.reclaimers,
        )
            .hash(&mut stamp);
        for front in &self.mining_fronts {
            (
                &front.id,
                front.face,
                front.position,
                front.crew,
                front.haulers,
                front.route.first(),
            )
                .hash(&mut stamp);
        }
        let stamp = stamp.finish();
        if self.crew_job_stamp == Some(stamp) {
            self.movement.tick(
                &self.crew_job_cache,
                &self.terrain,
                &self.workings,
                self.level("shaft"),
                self.travel_power(),
            );
            return;
        }
        self.crew_job_stamp = Some(stamp);
        let position = crew::access_position(&self.terrain, &self.workings);
        let building = self
            .workings
            .section
            .as_ref()
            .is_some_and(|s| s.cells.iter().all(|p| self.terrain.contains(p[0], p[1])));
        let mut jobs = vec![crew::Job {
            id: "access".into(),
            role: "diggers",
            target: position,
            count: access,
            activity: if self.workings.section.is_none() {
                "waiting"
            } else if building {
                "building"
            } else {
                "digging"
            },
        }];
        for front in &self.mining_fronts {
            let target = crew::stand_near(front.face, &self.terrain, &self.workings);
            jobs.push(crew::Job {
                id: format!("dig:{}", front.id),
                role: "diggers",
                target,
                count: front.crew,
                activity: "digging",
            });
            jobs.push(crew::Job {
                id: format!("haul:{}", front.id),
                role: "haulers",
                target: crew::stand_near(
                    front.route.first().copied().unwrap_or(front.position),
                    &self.terrain,
                    &self.workings,
                ),
                count: front.haulers,
                activity: "hauling",
            });
        }
        let assigned_haulers: u32 = self.mining_fronts.iter().map(|f| f.haulers).sum();
        jobs.push(crew::Job {
            id: "surface-haul".into(),
            role: "haulers",
            target: [WIDTH / 2, 0],
            count: self.crew.haulers.saturating_sub(assigned_haulers),
            activity: "hauling",
        });
        jobs.push(crew::Job {
            id: "support".into(),
            role: "engineers",
            target: position,
            count: self.crew.engineers,
            activity: if building { "building" } else { "waiting" },
        });
        jobs.push(crew::Job {
            id: "survey".into(),
            role: "prospectors",
            target: crew::stand_near(
                self.workings
                    .passages
                    .get(self.workings.active)
                    .map(|n| n.feet)
                    .unwrap_or([WIDTH / 2, 0]),
                &self.terrain,
                &self.workings,
            ),
            count: self.crew.prospectors,
            activity: "surveying",
        });
        jobs.extend(crew::surface_jobs(&self.crew));
        self.crew_job_cache = jobs;
        self.movement.tick(
            &self.crew_job_cache,
            &self.terrain,
            &self.workings,
            self.level("shaft"),
            self.travel_power(),
        );
    }
    fn dig_rate_for(&self, diggers: u32) -> u64 {
        ((diggers as f64 * self.throughput("drill")
            / sites()[self.profile].hardness
            / if self.challenge == "hard_rock" {
                1.5
            } else {
                1.
            }
            / self.rock_work()
            * match self.specialisation.as_deref() {
                Some("bulk") => 1.3,
                Some("precision") => 0.8,
                _ => 1.,
            }
            * (1. + 0.04 * self.ranks.get("excavation").copied().unwrap_or(0) as f64))
            * UNITS as f64
            * pacing::get().worker_rate) as u64
    }
    fn dig_rate(&self, _offline: bool) -> u64 {
        self.dig_rate_for(self.crew.diggers)
    }
    fn max_mining_fronts(&self) -> usize {
        8 + usize::from(self.ranks.get("excavation").copied().unwrap_or(0) >= 6) * 2
            + usize::from(self.ranks.get("excavation").copied().unwrap_or(0) >= 10) * 2
    }
    fn required_front_materials(&self, cat: &[Material]) -> BTreeSet<usize> {
        let mut products = BTreeSet::new();
        if let Some(id) = &self.pinned {
            if let Some(upgrade) = requirements().iter().find(|upgrade| &upgrade.id == id) {
                for (product, amount) in &upgrade.inputs {
                    if self.products.get(product).copied().unwrap_or(0) < *amount {
                        products.insert(product.as_str());
                    }
                }
            }
        }
        if self.level("furnace") > 0
            && matches!(self.pinned.as_deref(), Some("furnace" | "steelworks"))
        {
            for product in ["iron", "coke", "lime"] {
                if self.products.get(product).copied().unwrap_or(0) < self.product_hold(product) {
                    products.insert(product);
                }
            }
        }
        for recipe in recipes().iter().filter(|recipe| {
            self.level(&recipe.building) > 0
                && !self.paused_recipes.contains(&recipe.id)
                && (recipe.id == "steel"
                    || recipe.id == "aluminium"
                    || self.enabled_recipes.contains(&recipe.id))
                && self.products.get(&recipe.output).copied().unwrap_or(0)
                    < self.product_hold(&recipe.output).max(UNITS)
        }) {
            for (input, amount) in &recipe.inputs {
                let committed = self
                    .pinned
                    .as_ref()
                    .and_then(|id| requirements().iter().find(|upgrade| &upgrade.id == id))
                    .and_then(|upgrade| upgrade.inputs.get(input))
                    .copied()
                    .unwrap_or(0)
                    .max(
                        if input == "iron" && self.pinned.is_some() && self.level("shaft") > 0 {
                            8 * UNITS
                        } else {
                            0
                        },
                    );
                if self.products.get(input).copied().unwrap_or(0)
                    < self.product_hold(input).max(committed + amount)
                {
                    products.insert(input.as_str());
                }
            }
        }
        // Manufactured feeds recurse to their practical mine inputs.
        if products.contains("steel") {
            products.extend(["iron", "coke", "lime"]);
        }
        if products.contains("wiring") {
            products.insert("copper");
        }
        if products.contains("batteries") {
            products.extend(["lithium_carbonate", "graphite", "nickel", "cobalt"]);
        }
        cat.iter()
            .filter(|material| products.contains(material.product.as_str()))
            .map(|material| material.id)
            .collect()
    }
    fn branch_rate(&self, haulers: u32) -> u64 {
        let rank = 1. + 0.04 * self.ranks.get("logistics").copied().unwrap_or(0) as f64;
        let units = if self.level("conveyor") > 0 {
            4. * self.throughput("conveyor")
        } else if self.level("wheelbarrow") > 0 {
            2. * self.throughput("wheelbarrow") * haulers as f64
        } else {
            haulers as f64
        };
        (units * rank * pacing::get().haul_rate * UNITS as f64) as u64
    }
    fn required_feed_unexposed(&self, required: &BTreeSet<usize>, cat: &[Material]) -> bool {
        required.iter().any(|material| {
            let product = &cat[*material].product;
            self.products.get(product).copied().unwrap_or(0) < self.product_hold(product).max(1)
                && !self.mining_fronts.iter().any(|front| {
                    front.status != "Deposit exhausted" && cat[front.material].product == *product
                })
        })
    }
    pub fn work_route(&self) -> &[[i64; 2]] {
        &self.haul_path
    }
    fn next_frontier(&self, _cat: &[Material]) -> Option<i64> {
        self.workings.next_cell(&self.terrain)
    }
    /// No input, pending arrival, eligible recipe or reachable excavation event can fire.
    fn quiescent(&self, cat: &[Material]) -> bool {
        self.quiet_pipeline(cat)
            && self.next_frontier(cat).is_none()
            && self.transport.mass() == 0
            && self
                .mining_fronts
                .iter()
                .all(|front| front.crew == 0 && front.stockpile == 0)
    }
    fn quiet_pipeline(&self, cat: &[Material]) -> bool {
        if self.mining_fronts.iter().any(|front| front.stockpile > 0) {
            return false;
        }
        if self.workings.blocked_at.is_none() {
            return false;
        }
        if self.transport.mass() > 0
            || self
                .ore
                .values()
                .chain(self.hauled.values())
                .any(|q| *q > 0)
            || self.concentrate.values().any(|&q| q > 0)
            || self.raw_stock.iter().any(|(&id, &q)| {
                q > 0
                    && (self.feed_unlocked(&cat[id]) || q > self.reserved_feed_target(id, &cat[id]))
            })
            || self.depleted > 0
            || (self.level("slagcrusher") > 0 && self.slag > 0)
            || (self.level("reclaimer") > 0 && self.tailings.values().any(|q| *q > 0))
        {
            return false;
        }
        if self.survey_pending() {
            return false;
        }
        self.recipes_idle()
    }
    fn survey_pending(&self) -> bool {
        if !self.workings.passages.is_empty() {
            return self
                .workings
                .survey_pending(&self.terrain, self.level("survey"));
        }
        if self.level("survey") == 0 {
            return false;
        }
        let (x, y) = self
            .removed
            .last()
            .map(|c| (c.x, c.y))
            .unwrap_or((WIDTH / 2, 0));
        (x.saturating_sub(16)..=x.saturating_add(16)).any(|px| {
            (y.saturating_sub(16).max(0)..=y.saturating_add(16).min(terrain::MAX_ROWS - 1)).any(
                |py| {
                    px.abs_diff(x).pow(2) + py.abs_diff(y).pow(2) <= 16 * 16
                        && !self.terrain.is_revealed(px, py)
                },
            )
        })
    }
    fn stationary_pipeline(&self, cat: &[Material]) -> bool {
        if self
            .mining_fronts
            .iter()
            .any(|front| front.crew > 0 || front.stockpile > 0)
        {
            return false;
        }
        if self.workings.blocked_at.is_none() {
            return false;
        }
        self.ticks % 20 == 0
            && self.flow_window.iter().all(|&q| q == 0)
            && self
                .transport
                .stations
                .iter()
                .all(|s| s.incoming == 0 && s.outgoing == 0)
            && self
                .transport
                .segments
                .iter()
                .all(|s| s.batches.iter().all(|b| b.remaining_ms == 0))
            && (self.ore.values().sum::<u64>() + CELL_MASS
                > 20 * UNITS + 5 * UNITS * self.level("capacity") as u64
                || self.next_frontier(cat).is_none())
            && !self.survey_pending()
            && self.depleted == 0
            && !(self.level("slagcrusher") > 0 && self.slag > 0)
            && !(self.level("reclaimer") > 0 && self.tailings.values().any(|&q| q > 0))
            && self.recipes_idle()
    }
    fn recipes_idle(&self) -> bool {
        !recipes().iter().any(|r| {
            if self.level(&r.building) == 0
                || self.paused_recipes.contains(&r.id)
                || !(r.id == "steel" || r.id == "aluminium" || self.enabled_recipes.contains(&r.id))
            {
                return false;
            }
            if r.id.starts_with("separate_") {
                return self
                    .trace_feed
                    .get(&format!("{}_residue", r.output))
                    .copied()
                    .unwrap_or(0)
                    > 0;
            }
            r.inputs.iter().all(|(p, n)| {
                let committed = self
                    .pinned
                    .as_ref()
                    .and_then(|id| requirements().iter().find(|u| u.id == *id))
                    .and_then(|u| u.inputs.get(p))
                    .copied()
                    .unwrap_or(0);
                let committed = committed.max(
                    if p == "iron"
                        && self.pinned.is_some()
                        && self.level("shaft") > 0
                        && self.level("shaft") < 50
                    {
                        8 * UNITS
                    } else {
                        0
                    },
                );
                self.products
                    .get(p)
                    .copied()
                    .unwrap_or(0)
                    .saturating_sub(committed)
                    >= *n
            })
        })
    }
    pub fn second(&mut self, cat: &[Material], offline: bool) {
        for _ in 0..20 {
            self.tick(cat, offline);
        }
    }
    pub fn tick(&mut self, cat: &[Material], offline: bool) {
        self.ticks += 1;
        if self.ticks % 20 == 1 {
            self.flow_window = [0; 5];
            self.processing_window.clear();
        }
        let sold_before = self.sold_mass;
        self.crew = logistics::Crew::prioritise(self.workers, &self.levels, &self.crew_priority);
        let required_fronts = self.required_front_materials(cat);
        let seek_feed = self.workings.target_deposit.is_none()
            && self.required_feed_unexposed(&required_fronts, cat);
        let mut planning_priorities = Vec::new();
        for id in required_fronts
            .iter()
            .copied()
            .chain(self.priorities.iter().copied())
        {
            if !planning_priorities.contains(&id) {
                planning_priorities.push(id);
            }
            if planning_priorities.len() == 3 {
                break;
            }
        }
        {
            self.workings.initialise();
            self.workings.first_junction_requested = self.level("shaft") > 0
                && self.workings.target_deposit.is_none()
                && !self.workings.has_underground_branch();
            self.workings.survey_work += (1 + self.surveying_crew() as u64 * 3)
                * (100 + 5 * self.ranks.get("prospecting").copied().unwrap_or(0) as u64)
                / 100;
            if self.workings.survey_work >= 200 || self.workings.surveyed.is_empty() {
                self.workings.survey_work %= 200;
                let upgraded = self.level("survey") > 0;
                let mut found = self.workings.survey_versioned(
                    &mut self.terrain,
                    self.seed,
                    self.profile,
                    self.generator_version,
                    cat,
                    upgraded,
                    false,
                );
                let prospecting_rank = self.ranks.get("prospecting").copied().unwrap_or(0);
                let refinements = if prospecting_rank >= 6 { 2 } else { 1 };
                let accuracy =
                    self.levels
                        .get("survey")
                        .copied()
                        .unwrap_or(0)
                        .max(u32::from(prospecting_rank >= 10) * 2) as i64;
                for _ in 0..refinements {
                    found.extend(self.workings.refine_survey_versioned(
                        &mut self.terrain,
                        self.seed,
                        self.profile,
                        self.generator_version,
                        cat,
                        accuracy,
                    ));
                }
                for id in found {
                    if self.discoveries.insert(id) {
                        self.site_discoveries += 1;
                    }
                }
            }
            let limit = self.equipment_depth_limit() as i64 * geometry::CELLS_PER_METRE;
            self.workings.prepare_deposit_order_versioned(
                self.seed,
                self.profile,
                self.generator_version,
                cat,
            );
            let engineers = if self.ranks.get("excavation").copied().unwrap_or(0) >= 10 {
                (self.support_crew() as i64 * 3 + 1) / 2
            } else {
                self.support_crew() as i64
            };
            self.workings.advance(
                &self.terrain,
                &planning_priorities,
                if seek_feed { "depth" } else { &self.policy },
                limit,
                engineers,
                self.level("supports") as i64,
            );
        }
        if self.workings.passages.is_empty() && self.ticks % 1200 == 0 && self.level("survey") > 0 {
            let (x, y) = self
                .removed
                .last()
                .map(|c| (c.x, c.y))
                .unwrap_or((WIDTH / 2, 0));
            for id in self.terrain.reveal_versioned(
                self.seed,
                self.profile,
                self.generator_version,
                x,
                y,
                16,
                cat,
            ) {
                if self.discoveries.insert(id) {
                    self.site_discoveries += 1;
                }
            }
        }
        let cap = self.raw_stock_capacity();
        let ore_total: u64 = self.ore.values().sum();
        let required = required_fronts;
        let max_fronts = self.max_mining_fronts();
        let buffer_level = self.level("capacity");
        if self.ticks % 100 == 1 || self.mining_fronts.is_empty() {
            fronts::refresh_cached(
                &mut self.front_candidate_cache,
                &mut self.mining_fronts,
                &self.terrain,
                &self.workings,
                self.seed,
                self.profile,
                self.generator_version,
                cat,
                &required,
                &self.priorities,
                max_fronts,
                buffer_level,
            );
        }
        // Keep the access assignment while the next connected section is planned.
        // A short search must not send the crew back down an unrelated ore route.
        let access_pending = self.workings.section.is_some() || self.workings.search.is_some();
        let access_crew = fronts::assign(
            &mut self.mining_fronts,
            self.crew.diggers,
            self.crew.haulers,
            access_pending,
        );
        let mut access_crew = access_crew;
        let required_access = access_pending
            && !required.is_empty()
            && self.required_feed_unexposed(&required, cat)
            && !self.mining_fronts.iter().any(|front| front.selected);
        if access_crew == 0
            && ((access_pending && self.workings.first_junction_requested)
                || required_access
                || (access_pending
                    && required.is_empty()
                    && (self.policy == "depth"
                        || (self.level("shaft") > 0 && !self.workings.has_underground_branch()))
                    && !self.mining_fronts.iter().any(|front| front.selected))
                || (self.crew.engineers == 0
                    && self.workings.section.as_ref().is_some_and(|s| {
                        s.cells.iter().all(|p| self.terrain.contains(p[0], p[1]))
                    })))
        {
            if let Some(front) = self.mining_fronts.iter_mut().rev().find(|f| f.crew > 0) {
                front.crew -= 1;
                access_crew = 1;
            }
        }
        self.move_crew(access_crew);
        let access_crew = self.movement.working("access");
        let working = self.movement.working_counts();
        for front in &mut self.mining_fronts {
            front.crew = working
                .get(&format!("dig:{}", front.id))
                .copied()
                .unwrap_or(0);
            front.haulers = working
                .get(&format!("haul:{}", front.id))
                .copied()
                .unwrap_or(0);
            if front.crew == 0
                && self
                    .movement
                    .workers
                    .iter()
                    .any(|p| p.job == format!("dig:{}", front.id))
            {
                front.status = "Crew travelling".into();
                front.blocker = if self
                    .movement
                    .workers
                    .iter()
                    .any(|p| p.job == format!("dig:{}", front.id) && p.activity == "blocked")
                {
                    "No cleared route to work site".into()
                } else {
                    String::new()
                };
            }
        }
        let mut mined = 0u64;
        let mut mined_cargo = BTreeMap::<usize, u64>::new();
        let dig_rate = self.dig_rate_for(access_crew);
        let work = dig_rate + self.dig_remainder;
        self.dig_progress += if access_crew > 0 { work / 20 } else { 0 };
        self.dig_remainder = work % 20;
        let digs = (self.dig_progress / 1000) as u32;
        self.dig_progress %= 1000;
        let depth_limit = self.lift_depth_limit();
        for _ in 0..digs.min(12800) {
            if self.ore.values().sum::<u64>() + (mined + 1) * CELL_MASS > cap {
                break;
            }
            let target = self.next_frontier(cat);
            let Some(key) = target else { break };
            let (x, y) = {
                let [x, y] = geometry::cell_point(key);
                (x, y)
            };
            let id = self.cell(x, y, cat);
            if !self.terrain.excavate(x, y) {
                break;
            }
            self.heights.excavate_to(x, y + 1);
            self.removed.push(Cell { x, y, material: id });
            if self.removed.len() > 512 {
                self.removed.drain(..256);
            }
            *self.ore.entry(id).or_default() += CELL_MASS;
            *mined_cargo.entry(id).or_default() += CELL_MASS;
            self.terrain.reveal_versioned(
                self.seed,
                self.profile,
                self.generator_version,
                x,
                y,
                1,
                cat,
            );
            self.excavated += 1;
            mined += 1;
            if self.discoveries.insert(id) {
                self.site_discoveries += 1;
            }
        }

        // Every extraction front owns progress and stockpile. One blocked vein
        // pauses locally while other crews continue mining different feeds.
        let mut claimed: BTreeSet<_> = self.mining_fronts.iter().map(|front| front.face).collect();
        for index in 0..self.mining_fronts.len() {
            let crew = self.mining_fronts[index].crew;
            if crew == 0 {
                continue;
            }
            let work = self.dig_rate_for(crew);
            {
                let front = &mut self.mining_fronts[index];
                let work = work + front.work_remainder;
                front.progress += work / 20;
                front.work_remainder = work % 20;
            }
            // Commit one complete worker-sized cut at a time. Partial one-pixel
            // cuts never become visible, and every removed cell still consumes
            // its normal work and enters the work-face material ledger.
            for _ in 0..512 {
                let face = self.mining_fronts[index].face;
                if self.mining_fronts[index].cut_work == 0 {
                    let solid = fronts::clearance_cells(face)
                        .into_iter()
                        .filter(|[x, y]| !self.terrain.contains(*x, *y))
                        .count() as u64;
                    if solid > 0 {
                        self.mining_fronts[index].cut_work = solid * 1000;
                    }
                }
                let required_work = self.mining_fronts[index].cut_work;
                // Work belongs to this solid face. Excess production cannot
                // pre-dig another face while its crew still needs to travel.
                self.mining_fronts[index].progress =
                    self.mining_fronts[index].progress.min(required_work);
                if required_work > 0 && self.mining_fronts[index].progress < required_work {
                    break;
                }
                let solid: Vec<_> = fronts::clearance_cells(face)
                    .into_iter()
                    .filter(|[x, y]| !self.terrain.contains(*x, *y))
                    .map(|[x, y]| ([x, y], self.cell(x, y, cat)))
                    .collect();
                if solid.is_empty() {
                    claimed.remove(&face);
                    let next = fronts::next_face_versioned(
                        &self.mining_fronts[index],
                        &self.terrain,
                        &self.workings,
                        self.seed,
                        self.profile,
                        self.generator_version,
                        cat,
                        &claimed,
                    );
                    let Some(next) = next else {
                        let front = &mut self.mining_fronts[index];
                        front.status = "Deposit exhausted".into();
                        front.crew = 0;
                        break;
                    };
                    self.mining_fronts[index].position = face;
                    self.mining_fronts[index].face = next;
                    self.mining_fronts[index].cut_work = 0;
                    self.mining_fronts[index].work_remainder = 0;
                    claimed.insert(next);
                    break;
                }
                let required_work = solid.len() as u64 * 1000;
                self.mining_fronts[index].cut_work = required_work;
                if self.mining_fronts[index].progress < required_work {
                    break;
                }
                let mass = solid.len() as u64 * CELL_MASS;
                let front = &self.mining_fronts[index];
                if front.stockpile + mass > front.capacity {
                    let front = &mut self.mining_fronts[index];
                    front.status = "Paused".into();
                    front.blocker = "Work-face stockpile full".into();
                    break;
                }
                self.mining_fronts[index].progress -= required_work;
                self.mining_fronts[index].cut_work = 0;
                for ([x, y], material) in solid {
                    if !self.terrain.excavate(x, y) {
                        continue;
                    }
                    self.heights.excavate_to(x, y + 1);
                    self.removed.push(Cell { x, y, material });
                    if self.removed.len() > 512 {
                        self.removed.drain(..256);
                    }
                    let front = &mut self.mining_fronts[index];
                    front.stockpile += CELL_MASS;
                    *front.stockpiles.entry(material).or_default() += CELL_MASS;
                    self.terrain.reveal_versioned(
                        self.seed,
                        self.profile,
                        self.generator_version,
                        x,
                        y,
                        1,
                        cat,
                    );
                    self.excavated += 1;
                    mined += 1;
                    if self.discoveries.insert(material) {
                        self.site_discoveries += 1;
                    }
                }
                claimed.remove(&face);
                let next = fronts::next_face_versioned(
                    &self.mining_fronts[index],
                    &self.terrain,
                    &self.workings,
                    self.seed,
                    self.profile,
                    self.generator_version,
                    cat,
                    &claimed,
                );
                let front = &mut self.mining_fronts[index];
                front.position = face;
                let Some(next) = next else {
                    front.status = "Deposit exhausted".into();
                    front.crew = 0;
                    break;
                };
                front.face = next;
                front.cut_work = 0;
                front.work_remainder = 0;
                claimed.insert(next);
                break;
            }
        }

        if let Some(cell) = self.removed.last() {
            let origin = if !self.workings.passages.is_empty() {
                self.workings.passages[self
                    .workings
                    .section
                    .as_ref()
                    .map(|s| s.from)
                    .unwrap_or(self.workings.active)]
                .feet
            } else {
                [16 + cell.x.saturating_sub(16) / 16 * 16, cell.y]
            };
            let key = (origin, self.level("shaft") > 0);
            if self.haul_route_key != Some(key) {
                self.haul_route_key = Some(key);
                self.haul_legs.clear();
                self.haul_path = if !self.workings.passages.is_empty() {
                    navigation::underground(&self.terrain, &self.heights, &self.workings)
                } else {
                    navigation::route(
                        &self.terrain,
                        &self.heights,
                        origin,
                        self.level("shaft") > 0,
                    )
                };
            }
        }
        let terrain_factor = if self.challenge == "long_haul" {
            1.5
        } else {
            1.
        } / sites()[self.profile].haul;
        if self.haul_legs.is_empty() || self.haul_levels != self.levels {
            self.haul_legs = logistics::route_registered(
                &self.haul_path,
                &self.levels,
                terrain_factor,
                &self.workings,
            )
            .0;
            self.haul_levels = self.levels.clone();
        }
        let haul_rate = self.branch_rate(self.crew.haulers)
            * if self.challenge == "long_haul" { 2 } else { 3 }
            / 2;
        self.transport
            .configure(&self.haul_legs, haul_rate, self.level("capacity"));
        for (id, q) in mined_cargo {
            self.transport.register_source(id, q);
        }
        // Register each work face against its own persisted itinerary before
        // moving cargo into shared underground transfer. Route ownership then
        // survives partial loads, save/load, and downstream backpressure.
        let levels = self.levels.clone();
        let capacity_level = self.level("capacity")
            + u32::from(self.ranks.get("logistics").copied().unwrap_or(0) >= 6) * 2;
        for front in &mut self.mining_fronts {
            if front.stockpile == 0 || front.route.is_empty() {
                continue;
            }
            let (legs, _) =
                logistics::route_registered(&front.route, &levels, terrain_factor, &self.workings);
            if legs.is_empty() {
                front.status = "Blocked".into();
                front.blocker = "No commissioned route to surface".into();
                continue;
            }
            if front.route_id == 0 || !self.transport.has_route(front.route_id) {
                self.transport.configure(&legs, haul_rate, capacity_level);
                front.route_id = self.transport.current_route;
            }
            let rate = if levels.get("conveyor").copied().unwrap_or(0) > 0 {
                (4. * (1. + 0.12 * levels["conveyor"] as f64)
                    * 1.5f64.powi((levels["conveyor"] / 10) as i32)
                    * pacing::get().haul_rate
                    * UNITS as f64) as u64
            } else if levels.get("wheelbarrow").copied().unwrap_or(0) > 0 {
                (2. * front.haulers as f64
                    * (1. + 0.12 * levels["wheelbarrow"] as f64)
                    * pacing::get().haul_rate
                    * UNITS as f64) as u64
            } else {
                (front.haulers as f64 * pacing::get().haul_rate * UNITS as f64) as u64
            };
            front.transfer_remainder += rate;
            let mut budget = front.transfer_remainder / 20;
            front.transfer_remainder %= 20;
            let mut room = cap.saturating_sub(self.ore.values().sum());
            if front.stockpiles.is_empty() && front.stockpile > 0 {
                front.stockpiles.insert(front.material, front.stockpile);
            }
            let mut materials: Vec<_> = front.stockpiles.keys().copied().collect();
            materials.sort_by_key(|material| (*material != front.material, *material));
            let mut transferred = 0;
            for material in materials {
                let available = front.stockpiles.get(&material).copied().unwrap_or(0);
                let amount = available.min(budget).min(room);
                if amount == 0 {
                    continue;
                }
                let remaining = available - amount;
                if remaining == 0 {
                    front.stockpiles.remove(&material);
                } else {
                    front.stockpiles.insert(material, remaining);
                }
                front.stockpile -= amount;
                transferred += amount;
                budget -= amount;
                room -= amount;
                *self.ore.entry(material).or_default() += amount;
                self.transport
                    .register_source_route(material, amount, front.route_id);
            }
            if transferred == 0 && front.stockpile > 0 && rate == 0 {
                front.status = "Waiting for haul crew".into();
                front.blocker = "No haul crew assigned".into();
            } else if transferred == 0 && front.stockpile > 0 && room == 0 {
                front.status = "Blocked".into();
                front.blocker = "Underground transfer full".into();
            }
        }
        // Keep public camera route attached to an active extraction face.
        if let Some(front) = self
            .mining_fronts
            .iter()
            .find(|front| front.crew > 0 && !front.route.is_empty())
        {
            self.haul_path = front.route.clone();
        }
        // Shared telemetry and capacity use current active route; individual
        // lots retain their source route identity above.
        if !self.haul_legs.is_empty() {
            self.transport
                .configure(&self.haul_legs, haul_rate, capacity_level);
        }
        let power = (self.power_factor(offline) * 1000.) as u64;
        let mut inaccessible = BTreeMap::new();
        let source = if self.haul_path.is_empty() {
            &mut inaccessible
        } else {
            &mut self.ore
        };
        self.transport.protect_routes(
            self.mining_fronts
                .iter()
                .map(|front| front.route_id)
                .filter(|route| *route != 0),
        );
        let haul_budget = self.transport.tick(
            self.ticks,
            source,
            &mut self.hauled,
            cap,
            &planning_priorities,
            self.cargo_policy == "preferred" || !required.is_empty(),
            false,
            power,
        );
        let remaining = 0;
        let sort_rate =
            (pacing::get().sorting_rate * UNITS as f64 * self.throughput("sorter")) as u64 / 20;
        let mut sort_left = sort_rate.min(cap.saturating_sub(self.concentrate.values().sum()));
        let sort_budget = sort_left;
        let mut sort_ids: Vec<_> = self
            .hauled
            .iter()
            .filter(|(_, q)| **q > 0)
            .map(|(&id, _)| id)
            .collect();
        logistics::order_cargo(
            &mut sort_ids,
            self.ticks,
            &planning_priorities,
            self.cargo_policy == "preferred" || !required.is_empty(),
        );
        for id in sort_ids {
            let quantity = self.hauled.get_mut(&id).expect("known feed");
            let n = (*quantity).min(sort_left);
            *quantity -= n;
            *self.concentrate.entry(id).or_default() += n;
            sort_left -= n;
        }
        let sorted = sort_budget - sort_left;
        let mut intake_space = cap.saturating_sub(self.concentrate.values().sum());
        for id in self.raw_stock.keys().copied().collect::<Vec<_>>() {
            let held = if self.feed_unlocked(&cat[id]) {
                0
            } else {
                self.reserved_feed_target(id, &cat[id])
            };
            let q = self.raw_stock.get_mut(&id).unwrap();
            let n = q.saturating_sub(held).min(intake_space);
            *q -= n;
            *self.concentrate.entry(id).or_default() += n;
            intake_space -= n;
        }
        let mut stock_space = self
            .raw_stock_capacity()
            .saturating_sub(self.raw_stock.values().sum());

        let power_factor = self.power_factor(offline);
        let process_rate = (power_factor
            * if self.specialisation.as_deref() == Some("reclamation") {
                0.85
            } else {
                1.
            }
            * (pacing::get().refining_rate * UNITS as f64)
            * self.throughput("furnace")
            * (1. + 0.15 * self.crew.operators.saturating_sub(1) as f64)
            * (1. + 0.04 * self.ranks.get("metallurgy").copied().unwrap_or(0) as f64))
            as u64;
        let process_rate = process_rate / 20;
        let mut left = process_rate;
        let mut feed_ids: Vec<_> = self
            .concentrate
            .iter()
            .filter(|(_, q)| **q > 0)
            .map(|(&id, _)| id)
            .collect();
        if !feed_ids.is_empty() {
            let offset = (self.ticks / 20) as usize % feed_ids.len();
            feed_ids.rotate_left(offset);
        }
        for id in feed_ids {
            let q = self.concentrate.get_mut(&id).expect("known feed");
            let m = &cat[id];
            let unlocked = match m.family.as_str() {
                "physical" => true,
                "furnace" | "industrial" => self.levels.get("furnace").copied().unwrap_or(0) > 0,
                "sulfide" | "chemical" | "electrolytic" => {
                    self.levels.get("chemical").copied().unwrap_or(0) > 0
                }
                _ => self.levels.get("trace").copied().unwrap_or(0) > 0,
            };
            let n = (*q).min(left);
            if n == 0 {
                continue;
            }
            *q -= n;
            left -= n;
            if !unlocked {
                let starter = if matches!(id, 3 | 5 | 6) {
                    pacing::get().starter_hold_units
                        * UNITS
                        * if id == 3 {
                            pacing::get().starter_iron_multiplier
                        } else {
                            1
                        }
                } else {
                    0
                };
                let wanted = starter.max(
                    self.reserve
                        .get(&m.product)
                        .copied()
                        .unwrap_or(0)
                        .saturating_mul(2),
                );
                let stored = self.raw_stock.entry(id).or_default();
                let held = n.min(wanted.saturating_sub(*stored)).min(stock_space);
                *stored += held;
                stock_space -= held;
                let n = n - held;
                self.credit_fraction += n * m.price;
                self.credits += self.credit_fraction / (2 * UNITS);
                self.credit_fraction %= 2 * UNITS;
                self.sold_mass += n;
                continue;
            }
            let recovery = (65i32
                + 3 * self.levels.get("recovery").copied().unwrap_or(0) as i32
                + if self.ranks.get("metallurgy").copied().unwrap_or(0) >= 6 {
                    5
                } else {
                    0
                }
                + if self.ranks.get("reclamation").copied().unwrap_or(0) >= 6 {
                    5
                } else {
                    0
                }
                + match self.specialisation.as_deref() {
                    Some("bulk") => -5,
                    Some("precision") => 10,
                    _ => 0,
                })
            .clamp(0, 95) as u64;
            let fraction = self.recovery_fraction.entry(id).or_default();
            let recovered = n * recovery + *fraction;
            let good = recovered / 100;
            *fraction = recovered % 100;
            let waste = n - good;
            let mut primary = good;
            for rule in traces().iter().filter(|rule| rule.feed == id) {
                let fraction = self.trace_fraction.entry(rule.output.clone()).or_default();
                *fraction += good * rule.permille;
                let recovered = (*fraction / 1000).min(primary);
                *fraction -= recovered * 1000;
                primary -= recovered;
                *self
                    .trace_feed
                    .entry(format!("{}_residue", rule.output))
                    .or_default() += recovered;
            }
            let product = if m.name == "Bauxite" {
                "alumina".into()
            } else {
                m.product.clone()
            };
            if primary > 0 {
                self.collection.insert(product.clone());
            }
            let flow = self.processing_window.entry(id).or_default();
            flow[0] += n;
            flow[1] += primary;
            *self.products.entry(product).or_default() += primary;
            let residue = waste / 5;
            if matches!(m.family.as_str(), "furnace" | "sulfide") {
                self.slag += residue;
            } else {
                self.depleted += residue;
            }
            *self.tailings.entry(id).or_default() += waste - residue;
            self.lifetime_waste += waste;
        }
        let recipes = recipes();
        // Rotate the first recipe each second so shared feeds reach all enabled modules.
        for offset in 0..recipes.len() {
            let recipe = &recipes[(offset + (self.ticks / 20) as usize) % recipes.len()];
            let automatic = recipe.id == "steel" || recipe.id == "aluminium";
            if self.level(&recipe.building) == 0
                || self.paused_recipes.contains(&recipe.id)
                || (!automatic && !self.enabled_recipes.contains(&recipe.id))
            {
                continue;
            }
            let rate = (12500.
                * power_factor
                * self.throughput(&recipe.building)
                * (1. + 0.04 * self.ranks.get("metallurgy").copied().unwrap_or(0) as f64))
                as u64;
            let recipe_budget = work_budget(rate, self.ticks);
            if recipe.id.starts_with("separate_") {
                let source = format!("{}_residue", recipe.output);
                let available = self.trace_feed.entry(source).or_default();
                let amount = (*available).min(recipe_budget * 64);
                *available -= amount;
                if amount > 0 {
                    self.collection.insert(recipe.output.clone());
                }
                *self.products.entry(recipe.output.clone()).or_default() += amount;
                continue;
            }
            let amount = recipe
                .inputs
                .iter()
                .map(|(p, n)| {
                    let committed = self
                        .pinned
                        .as_ref()
                        .and_then(|id| requirements().iter().find(|u| u.id == *id))
                        .and_then(|u| u.inputs.get(p))
                        .copied()
                        .unwrap_or(0)
                        .max(
                            if p == "iron"
                                && self.pinned.is_some()
                                && self.level("shaft") > 0
                                && self.level("shaft") < 50
                            {
                                8 * UNITS
                            } else {
                                0
                            },
                        );
                    self.products
                        .get(p)
                        .copied()
                        .unwrap_or(0)
                        .saturating_sub(committed)
                        / n
                })
                .min()
                .unwrap_or(0)
                .min(recipe_budget);
            if amount > 0 {
                for (p, n) in &recipe.inputs {
                    *self.products.entry(p.clone()).or_default() -= amount * n;
                }
                self.collection.insert(recipe.output.clone());
                let mass: u64 = recipe.inputs.values().sum();
                *self.products.entry(recipe.output.clone()).or_default() += amount * mass;
                if recipe.id == "steel" {
                    self.steel_made = true;
                }
            }
        }
        let mut recovery_space = cap.saturating_sub(self.hauled.values().sum());
        if self.level("reclaimer") > 0 {
            let multiplier = if self.specialisation.as_deref() == Some("reclamation") {
                3
            } else {
                1
            };
            for (&id, q) in &mut self.tailings {
                let n = (*q).min(
                    self.crew.reclaimers.max(1) as u64
                        + u64::from(self.ticks % 20 == 0)
                            * self.ranks.get("reclamation").copied().unwrap_or(0) as u64,
                );
                let n = (n * multiplier * 64).min(*q).min(recovery_space);
                recovery_space -= n;
                *q -= n;
                *self.hauled.entry(id).or_default() += n;
            }
        }
        let product_holds: BTreeMap<_, _> = self
            .products
            .iter()
            .filter(|(_, quantity)| **quantity > 0)
            .map(|(p, _)| (p.clone(), self.product_hold(p)))
            .collect();
        for (p, q) in &mut self.products {
            // Keep empty collection entries without repeatedly quoting their reserves.
            // Preserve carry normalization for older diagnostic/imported states too.
            if *q == 0 && self.credit_fraction < 2 * UNITS {
                continue;
            }
            let held = product_holds.get(p).copied().unwrap_or(0);
            let sold = q.saturating_sub(held);
            *q -= sold;
            self.sold_mass += sold;
            let price = cat
                .iter()
                .find(|m| m.product == *p)
                .map(|m| m.price)
                .unwrap_or(20);
            self.credit_fraction += sold * price * 2;
            self.credits = self
                .credits
                .saturating_add(self.credit_fraction / (2 * UNITS));
            self.credit_fraction %= 2 * UNITS;
        }
        let disposed = self.depleted.min(320);
        self.depleted -= disposed;
        self.disposed_mass += disposed;
        if self.level("slagcrusher") > 0 {
            let reclaimed =
                self.slag
                    .min(if self.specialisation.as_deref() == Some("reclamation") {
                        4800
                    } else {
                        1600
                    });
            self.slag -= reclaimed;
            *self.products.entry("aggregate".into()).or_default() += reclaimed;
        }
        if self.ticks % 20 == 0 {
            self.waste_profile
                .reconcile(self.slag + self.depleted + self.tailings.values().sum::<u64>());
        }
        for (i, n) in [
            mined * CELL_MASS,
            haul_budget - remaining,
            sorted,
            process_rate - left,
            self.sold_mass - sold_before,
        ]
        .into_iter()
        .enumerate()
        {
            self.flow_window[i] += n;
        }
        if !self.challenge.is_empty() && self.depth() >= 300 && self.steel_made {
            self.site_objectives.insert("challenge".into());
        }
        let seconds = ((self.ticks - 1) % 20 + 1) as f64 / 20.;
        for (name, reached) in [
            ("First mineral", !self.discoveries.is_empty()),
            ("Mechanised hauling", self.level("conveyor") > 0),
            ("First steel", self.steel_made),
            ("300 metres", self.depth() >= 300),
            ("One kilometre", self.depth() >= 1000),
            ("Rare-earth separation", self.level("trace") > 0),
            ("Headquarters complete", self.megaproject),
        ] {
            if reached && self.milestones.insert(name.into()) {
                self.research += 2;
            }
        }
        if !offline && self.ticks % 20 == 0 {
            if let Some(id) = self.build_queue.first().cloned() {
                if self.level(&id) > 0 {
                    self.build_queue.remove(0);
                } else {
                    self.pinned = Some(id.clone());
                    let sequence = self.last_sequence;
                    if self
                        .action(Action {
                            sequence: sequence + 1,
                            kind: "buy".into(),
                            target: id,
                            value: 0,
                        })
                        .is_ok()
                    {
                        self.build_queue.remove(0);
                    }
                    self.last_sequence = sequence;
                }
            }
        }
        let front_stock = self
            .mining_fronts
            .iter()
            .map(|front| front.stockpile)
            .sum::<u64>();
        let front_capacity = self
            .mining_fronts
            .iter()
            .map(|front| front.capacity)
            .sum::<u64>();
        self.stages = vec![
            Stage {
                name: "Digging".into(),
                rate: self.flow_window[0] as f64 / UNITS as f64 / seconds,
                buffer: self.ore.values().sum::<u64>() + front_stock,
                capacity: cap + front_capacity,
                blocker: if let Some(front) = self
                    .mining_fronts
                    .iter()
                    .find(|front| !front.blocker.is_empty())
                {
                    &front.blocker
                } else if self.workings.status == "Waiting for supports" {
                    "Building local supports"
                } else if self.workings.search.is_some() {
                    "Planning surveyed access"
                } else if self.workings.section.is_some() {
                    // Deeper access may be gated while earlier surveyed rooms still work.
                    "Working surveyed ground"
                } else if self.depth() >= depth_limit {
                    "Shaft upgrade required"
                } else if self.depth() >= 1500 && self.level("ventilation") == 0 {
                    "Ventilation required"
                } else if self.depth() >= 700 && self.level("pump") == 0 {
                    "Drainage required"
                } else if self.depth() >= 300 && self.level("supports") == 0 {
                    "Supports required"
                } else {
                    "Working"
                }
                .to_string(),
            },
            Stage {
                name: "Hauling".into(),
                rate: self.flow_window[1] as f64 / UNITS as f64 / seconds,
                buffer: self.hauled.values().sum::<u64>() + self.transport.mass(),
                capacity: cap
                    + self
                        .transport
                        .stations
                        .iter()
                        .map(|s| s.capacity)
                        .sum::<u64>()
                    + self
                        .transport
                        .segments
                        .iter()
                        .map(|s| s.capacity)
                        .sum::<u64>(),
                blocker: if self.haul_path.is_empty() && self.ore.values().any(|&q| q > 0) {
                    "No walkable loading access".into()
                } else if let Some(segment) =
                    self.transport.segments.iter().rev().find(|s| s.blocked)
                {
                    format!("{}: {}", segment.name, segment.blocker)
                } else if haul_budget == 0 && ore_total > 0 {
                    "Loading bay full".into()
                } else if haul_budget == 0 && self.transport.mass() == 0 {
                    "Waiting for ore".into()
                } else {
                    "Working".into()
                },
            },
            Stage {
                name: "Sorting".into(),
                rate: self.flow_window[2] as f64 / UNITS as f64 / seconds,
                buffer: self.concentrate.values().sum(),
                capacity: cap,
                blocker: if sort_budget == 0 {
                    "Refining buffer full"
                } else if sorted == 0 {
                    "Waiting for arrivals"
                } else {
                    "Automatic separation"
                }
                .into(),
            },
            Stage {
                name: "Refining".into(),
                rate: self.flow_window[3] as f64 / UNITS as f64 / seconds,
                buffer: self.products.values().sum(),
                capacity: cap,
                blocker: if power_factor < 1. && self.concentrate.values().any(|q| *q > 0) {
                    "Power supply limited"
                } else if self.level("furnace") == 0 && self.raw_stock.values().any(|q| *q > 0) {
                    "Raw sales · furnace locked"
                } else if self.flow_window[3] == 0 {
                    "Waiting for ore"
                } else {
                    "Processing continuously"
                }
                .into(),
            },
            Stage {
                name: "Dispatch".into(),
                rate: self.flow_window[4] as f64 / UNITS as f64 / seconds,
                buffer: 0,
                capacity: cap,
                blocker: "Selling surplus".into(),
            },
        ];
    }
    pub fn advance_offline(&mut self, now: u64, cat: &[Material]) {
        self.advance_offline_observed(now, cat, |_, _, _| {});
    }
    /// Skip only empty-production travel between exact feedback and arrival boundaries.
    fn idle_travel_skip(&self, left: u64) -> u64 {
        if self.ticks % 20 == 0
            || self.movement.lift_level != self.level("shaft")
            || self.workings.section.is_none()
            || self.workings.search.is_some()
            || !self.movement.workers.iter().any(|p| p.route != 0)
            || self.movement.workers.iter().any(|p| {
                p.job.is_empty() && p.route != 0
                    || p.route == 0
                        && p.position == p.target
                        && (p.activity == "digging" || p.activity == "building")
            })
            || self.transport.mass() > 0
            || self.mining_fronts.iter().any(|f| f.stockpile > 0)
            || self
                .ore
                .values()
                .chain(self.hauled.values())
                .chain(self.concentrate.values())
                .chain(self.trace_feed.values())
                .any(|q| *q > 0)
            || self.raw_stock.values().any(|q| *q > 0)
            || self.depleted > 0
            || self.level("slagcrusher") > 0 && self.slag > 0
            || self.level("reclaimer") > 0 && self.tailings.values().any(|q| *q > 0)
            || self.products.iter().any(|(p, q)| *q > self.product_hold(p))
            || !self.recipes_idle()
        {
            return 0;
        }
        let mut skip = left.min(19 - self.ticks % 20);
        let survey_rate = (1 + self.surveying_crew() as u64 * 3)
            * (100 + 5 * self.ranks.get("prospecting").copied().unwrap_or(0) as u64)
            / 100;
        skip = skip.min(
            (200 - self.workings.survey_work)
                .div_ceil(survey_rate)
                .saturating_sub(1),
        );
        let power = self.travel_power() as u64;
        for worker in self.movement.workers.iter().filter(|p| p.route != 0) {
            let leg = &self.movement.routes[&worker.route][worker.leg];
            let speed = if leg.mode == "lift" { power } else { 1000 };
            if speed == 0 {
                continue;
            }
            let remaining =
                (leg.milliseconds - worker.elapsed_ms) as u64 * 1000 - worker.time_fraction as u64;
            skip = skip.min(
                (remaining * 1000)
                    .div_ceil(50_000 * speed)
                    .saturating_sub(1),
            );
        }
        skip
    }
    fn skip_idle_travel(&mut self, skip: u64) {
        let power = self.travel_power() as u64;
        let survey_rate = (1 + self.surveying_crew() as u64 * 3)
            * (100 + 5 * self.ranks.get("prospecting").copied().unwrap_or(0) as u64)
            / 100;
        self.workings.survey_work += survey_rate * skip;
        for worker in self.movement.workers.iter_mut().filter(|p| p.route != 0) {
            let leg = &self.movement.routes[&worker.route][worker.leg];
            let speed = if leg.mode == "lift" { power } else { 1000 };
            let elapsed = worker.time_fraction as u64 + 50 * speed * skip;
            worker.elapsed_ms += (elapsed / 1000) as u32;
            worker.time_fraction = (elapsed % 1000) as u32;
        }
        for segment in &mut self.transport.segments {
            let speed = if segment.demand > 0 { power } else { 1000 };
            segment.time_fraction = (segment.time_fraction + 50 * speed * skip) % 1000;
        }
        self.ticks += skip;
    }
    pub fn advance_offline_observed(
        &mut self,
        now: u64,
        cat: &[Material],
        mut observe: impl FnMut(&Game, u64, u64),
    ) {
        let elapsed = if self.last_saved == 0 {
            0
        } else {
            now.saturating_sub(self.last_saved)
        };
        let effective = elapsed.min(28800) / 2;
        let old = self.credits;
        let mined = self.excavated;
        let known = self.discoveries.clone();
        let total = effective * 20;
        let mut left = total;
        let mut observed = 0;
        observe(self, 0, total);
        while left > 0 {
            self.tick(cat, true);
            left -= 1;
            if self.ticks % 20 == 0 || left == 0 {
                observe(self, total - left, total);
                observed = total - left;
            }
            let travel_skip = self.idle_travel_skip(left);
            if travel_skip > 0 {
                self.skip_idle_travel(travel_skip);
                left -= travel_skip;
                continue;
            }
            let skip = if self.movement.workers.iter().any(|p| p.route != 0) {
                0
            } else if self.quiescent(cat) || self.stationary_pipeline(cat) {
                left.saturating_sub(1)
            } else if self.quiet_pipeline(cat) {
                // Stop immediately before the next dig, arrival, or final feedback tick.
                // Filled processing/reclamation pipelines still use exact fixed steps.
                let access_rate = self.dig_rate_for(self.movement.working("access"));
                let dig = if self.next_frontier(cat).is_some() && access_rate > 0 {
                    ((1000 - self.dig_progress) * 20 - self.dig_remainder).div_ceil(access_rate)
                } else {
                    u64::MAX
                };
                let front_dig = self
                    .mining_fronts
                    .iter()
                    .filter(|front| front.crew > 0 && front.stockpile < front.capacity)
                    .filter_map(|front| {
                        let rate = self.dig_rate_for(front.crew);
                        let required = front.cut_work.max(1000);
                        (rate > 0).then(|| {
                            (required.saturating_sub(front.progress) * 20)
                                .saturating_sub(front.work_remainder)
                                .div_ceil(rate)
                        })
                    })
                    .min()
                    .unwrap_or(u64::MAX);
                left.min(dig.min(front_dig)).saturating_sub(1)
            } else {
                0
            };
            if skip > 0 {
                let work =
                    self.dig_rate_for(self.movement.working("access")) * skip + self.dig_remainder;
                self.dig_progress = (self.dig_progress + work / 20) % 1000;
                self.dig_remainder = work % 20;
                let front_rates: Vec<_> = self
                    .mining_fronts
                    .iter()
                    .map(|front| self.dig_rate_for(front.crew))
                    .collect();
                for (front, rate) in self.mining_fronts.iter_mut().zip(front_rates) {
                    if front.crew == 0 || front.stockpile >= front.capacity {
                        continue;
                    }
                    let work = rate * skip + front.work_remainder;
                    front.progress += work / 20;
                    front.work_remainder = work % 20;
                }
                if (self.ticks - 1) / 20 != (self.ticks + skip - 1) / 20 {
                    self.flow_window = [0; 5];
                    self.processing_window.clear();
                }
                let transport_seconds = ((self.ticks + skip) / 20 - self.ticks / 20) as u32;
                if transport_seconds > 0 {
                    self.waste_profile
                        .reconcile(self.slag + self.depleted + self.tailings.values().sum::<u64>());
                }
                let power = (self.power_factor(true) * 1000.) as u64;
                for segment in &mut self.transport.segments {
                    let speed = if segment.demand > 0 { power } else { 1000 };
                    segment.time_fraction = (segment.time_fraction + 50 * speed * skip) % 1000;
                }
                if !self.workings.passages.is_empty() {
                    self.workings.survey_work = (self.workings.survey_work
                        + (1 + self.surveying_crew() as u64 * 3)
                            * (100
                                + 5 * self.ranks.get("prospecting").copied().unwrap_or(0) as u64)
                            / 100
                            * skip)
                        % 200;
                }
                self.ticks += skip;
                left -= skip;
            }
        }
        self.offline = Some(Offline {
            id: format!("{}:{}:{}", self.campaign_id, self.last_saved, now),
            discoveries: self.discoveries.difference(&known).copied().collect(),
            blockers: self
                .stages
                .iter()
                .filter(|s| {
                    s.blocker.contains("required")
                        || s.blocker.contains("full")
                        || s.blocker.contains("limited")
                })
                .map(|s| format!("{}: {}", s.name, s.blocker))
                .collect(),
            capped: elapsed.saturating_sub(28800),
            elapsed,
            effective,
            credits: (self.credits - old).to_string(),
            excavated: self.excavated - mined,
        });
        // A backwards clock must not make an already-accounted interval eligible again.
        if observed < total {
            observe(self, total, total);
        }
        self.last_saved = self.last_saved.max(now);
    }
    pub fn action(&mut self, a: Action) -> Result<(), String> {
        if a.sequence <= self.last_sequence {
            return Err("Command already processed".into());
        }
        match a.kind.as_str() {
            "retirement_preview" => {
                if !self.steel_made || self.depth() < 300 {
                    return Err("Reach 300 m and produce steel first".into());
                }
                self.retirement_quote = Some(self.award());
            }
            "cancel_retirement" => {
                self.retirement_quote = None;
            }

            "crew_priority" => {
                if ![
                    "balanced",
                    "digging",
                    "hauling",
                    "refining",
                    "reclaiming",
                    "engineering",
                    "prospecting",
                ]
                .contains(&a.target.as_str())
                {
                    return Err("Unknown crew priority".into());
                }
                self.crew_priority = a.target;
                self.crew =
                    logistics::Crew::prioritise(self.workers, &self.levels, &self.crew_priority);
            }
            "specialise" => {
                if self.depth() < pacing::get().specialisation_depth {
                    return Err(format!(
                        "Reach {} metres to specialise",
                        pacing::get().specialisation_depth
                    ));
                }
                if !self.steel_made {
                    return Err("Produce steel to specialise".into());
                }
                if self.specialisation.is_some() {
                    return Err("Specialisation lasts until site retirement".into());
                }
                if !["bulk", "precision", "reclamation"].contains(&a.target.as_str()) {
                    return Err("Unknown specialisation".into());
                }
                self.specialisation = Some(a.target.clone());
                self.milestones.insert("specialisation".into());
            }
            "buy" => {
                if let Some(reason) = self.purchase_blocker(&a.target) {
                    return Err(reason);
                }
                let requirement = requirements()
                    .iter()
                    .find(|u| u.id == a.target)
                    .expect("validated upgrade");
                let cost = self.cost(&a.target);
                if self.credits < cost {
                    return Err("Insufficient credits".into());
                }
                self.credits -= cost;
                for (p, n) in &requirement.inputs {
                    *self.products.entry(p.clone()).or_default() -= n;
                    self.delivered_mass += n;
                }
                match a.target.as_str() {
                    "worker" => self.workers += 1,
                    "housing" => self.housing += 4,
                    _ => *self.levels.entry(a.target).or_default() += 1,
                }
            }
            "policy" => {
                if self.site == 1 && self.depth() < pacing::get().tactics_depth {
                    return Err(format!(
                        "Reach {} metres to unlock tactics",
                        pacing::get().tactics_depth
                    ));
                }
                if !["bulk", "vein", "depth"].contains(&a.target.as_str()) {
                    return Err("Invalid policy".into());
                }
                if self.policy != a.target {
                    self.policy = a.target;
                    self.workings.search = None;
                    self.workings.blocked_at = None;
                    self.workings.revision += 1;
                }
            }
            "buffer" => {
                let index: usize = a.target.parse().map_err(|_| "Unknown station")?;
                let station = self
                    .transport
                    .stations
                    .get_mut(index)
                    .ok_or("Unknown station")?;
                let cost = (pacing::get().station_cost as f64
                    * pacing::get().capacity_growth.powi(station.level as i32))
                .ceil() as u64;
                if station.level >= 50 || self.credits < cost {
                    return Err("Buffer upgrade unavailable".into());
                }
                self.credits -= cost;
                station.level += 1;
                station.quote = (pacing::get().station_cost as f64
                    * pacing::get().capacity_growth.powi(station.level as i32))
                .ceil()
                .to_string();
            }
            "station_priority" => {
                let index: usize = a.target.parse().map_err(|_| "Unknown station")?;
                self.transport
                    .stations
                    .get_mut(index)
                    .ok_or("Unknown station")?
                    .preferred = a.value != 0;
            }
            "express" => {
                if a.value >= 4 {
                    return Err("Unknown express segment".into());
                }
                self.transport.express = a.value as usize;
            }
            "cargo_policy" => {
                if !["balanced", "preferred"].contains(&a.target.as_str()) {
                    return Err("Unknown cargo policy".into());
                }
                self.cargo_policy = a.target;
            }
            "clear_vein" => {
                self.workings.target = None;
                self.workings.target_deposit = None;
                self.workings.target_cells.clear();
                self.workings.search = None;
                self.workings.blocked_at = None;
                self.workings.revision += 1;
            }
            "target_vein" => {
                let coordinates: Vec<_> = a
                    .target
                    .split(',')
                    .map(str::parse::<i64>)
                    .collect::<Result<_, _>>()
                    .map_err(|_| "Invalid ore target")?;
                if coordinates.len() != 2 {
                    return Err("Invalid ore target".into());
                }
                let p = [coordinates[0], coordinates[1]];
                if !geometry::valid_cell(p[0], p[1])
                    || !self
                        .terrain
                        .known_material(p[0], p[1])
                        .is_some_and(|id| id > 1)
                {
                    return Err("Select surveyed, unmined ore".into());
                }
                let cells = geology::deposit_cells_versioned(
                    self.seed,
                    self.profile,
                    self.generator_version,
                    p,
                    materials(),
                );
                if !cells.iter().any(|cell| {
                    self.workings.mineable_cell(*cell)
                        && !self.terrain.contains(cell[0], cell[1])
                        && self
                            .terrain
                            .known_material(cell[0], cell[1])
                            .is_some_and(|id| id > 1)
                }) {
                    return Err("This vein has no mineable surveyed ore remaining".into());
                }
                // The face may be mined while its inspector is open. Keep the
                // stable deposit anchor when other surveyed cells still remain.
                self.workings.target = Some(p);
                self.workings.target_deposit = geology::deposit_id_versioned(
                    self.seed,
                    self.profile,
                    self.generator_version,
                    p,
                    materials(),
                );
                self.workings.target_cells = cells;
                self.workings.search = None;
                self.workings.blocked_at = None;
                self.workings.deferred.clear();
                self.workings.revision += 1;
                self.policy = "vein".into();
            }
            "priority" => {
                let id = a.value as usize;
                if id >= materials().len() {
                    return Err("Unknown mineral".into());
                }
                if self.priorities.contains(&id) {
                    self.priorities.retain(|x| *x != id)
                } else if self.priorities.len() < 3 {
                    self.priorities.push(id)
                } else {
                    return Err("Three priorities maximum".into());
                }
                self.workings.search = None;
                self.workings.blocked_at = None;
                self.workings.revision += 1;
            }
            "blueprint" => {
                if self.ranks.get("logistics").copied().unwrap_or(0) < 3 {
                    return Err("Requires logistics research rank 3".into());
                }
                self.blueprint = match a.target.as_str() {
                    "camp" => vec!["conveyor", "furnace", "steelworks", "shaft"],
                    "industry" => vec![
                        "conveyor",
                        "furnace",
                        "steelworks",
                        "shaft",
                        "supports",
                        "manufacturing",
                        "power",
                        "chemical",
                    ],
                    "off" => vec![],
                    _ => return Err("Unknown blueprint".into()),
                }
                .into_iter()
                .map(str::to_string)
                .collect();
                self.build_queue = self.blueprint.clone();
            }
            "pin" => {
                if !requirements().iter().any(|u| u.id == a.target) {
                    return Err("Unknown upgrade".into());
                }
                self.pinned = if self.pinned.as_ref() == Some(&a.target) {
                    None
                } else {
                    Some(a.target)
                };
            }
            "reserve" => {
                if a.value > 1_000_000_000 {
                    return Err("Reserve limit exceeded".into());
                }
                self.reserve.insert(a.target, a.value);
            }
            "recipe" => {
                if !recipes().iter().any(|r| r.id == a.target) {
                    return Err("Unknown recipe".into());
                }
                if matches!(a.target.as_str(), "steel" | "aluminium") {
                    if !self.paused_recipes.remove(&a.target) {
                        self.paused_recipes.insert(a.target);
                    }
                } else if !self.enabled_recipes.remove(&a.target) {
                    self.enabled_recipes.insert(a.target);
                }
            }
            "megaproject" => {
                if self.megaproject {
                    return Err("Project already complete".into());
                }
                if self.research_invested() < pacing::get().headquarters_research {
                    return Err(format!(
                        "Invest {} research in headquarters first",
                        pacing::get().headquarters_research
                    ));
                }
                let needs = [
                    ("advanced_structure", 10 * UNITS),
                    ("precision_controls", 10 * UNITS),
                    ("magnets", 10 * UNITS),
                    ("batteries", 10 * UNITS),
                ];
                if needs
                    .iter()
                    .any(|(p, n)| self.products.get(*p).copied().unwrap_or(0) < *n)
                {
                    return Err("Reserve ten units of each megaproject component".into());
                }
                for (p, n) in needs {
                    *self.products.entry(p.into()).or_default() -= n;
                    self.delivered_mass += n;
                }
                self.megaproject = true;
                self.research += 100;
            }

            "contract" => {
                let index = a.value as usize;
                let c = self.contracts.get(index).ok_or("Unknown contract")?.clone();
                if c.complete {
                    return Err("Request a new order first".into());
                }
                let q = self.products.entry(c.product.clone()).or_default();
                if *q < c.amount {
                    return Err("More product required".into());
                }
                *q -= c.amount;
                self.delivered_mass += c.amount;
                // Same fixed price as dispatch, with a 25% premium, retaining fractions.
                let price = materials()
                    .iter()
                    .find(|m| m.product == c.product)
                    .map(|m| m.price)
                    .unwrap_or(20);
                let reward = c.amount * price * 5 / 2;
                self.credit_fraction += reward;
                self.credits += self.credit_fraction / (2 * UNITS);
                self.credit_fraction %= 2 * UNITS;
                self.contracts[index].complete = true;
                self.site_objectives.insert(format!("order-{index}"));
                if self.milestones.insert(format!("delivery-{}", c.product)) {
                    self.research += 1;
                }
            }
            "new_contract" => {
                let index = a.value as usize;
                let old = self.contracts.get(index).ok_or("Unknown contract")?.clone();
                if !old.complete {
                    return Err("Deliver current order first".into());
                }
                let candidates: Vec<_> = self
                    .products
                    .iter()
                    .filter(|(p, q)| {
                        **q > 0
                            && !self
                                .contracts
                                .iter()
                                .enumerate()
                                .any(|(i, c)| i != index && !c.complete && c.product == **p)
                    })
                    .map(|(p, _)| p.clone())
                    .collect();
                let product = if candidates.is_empty() {
                    old.product
                } else {
                    candidates[(self.ticks as usize + index) % candidates.len()].clone()
                };
                self.contracts[index] = Contract {
                    product,
                    amount: old.amount.saturating_add(UNITS).min(20 * UNITS),
                    complete: false,
                };
            }
            "research" => {
                if ![
                    "excavation",
                    "logistics",
                    "metallurgy",
                    "prospecting",
                    "reclamation",
                ]
                .contains(&a.target.as_str())
                {
                    return Err("Unknown research".into());
                }
                let rank = *self.ranks.get(&a.target).unwrap_or(&0);
                let cost = pacing::get().research_base * (rank as u64 + 1).pow(2);
                if rank >= 10 || self.research < cost {
                    return Err("Research unavailable".into());
                }
                self.research -= cost;
                self.ranks.insert(a.target, rank + 1);
            }
            "retire" => {
                if !self.steel_made || self.depth() < 300 {
                    return Err("Reach 300 m and produce steel first".into());
                }
                let award = self.retirement_quote.unwrap_or_else(|| self.award());
                if a.value > 2 {
                    return Err("Unknown site".into());
                }
                if !["", "hard_rock", "long_haul"].contains(&a.target.as_str()) {
                    return Err("Unknown site challenge".into());
                }
                let mut next = Game::new(self.seed.wrapping_add(7919 + a.value), self.site + 1);
                next.campaign_id = self.campaign_id.clone();
                next.challenge = a.target;
                next.profile = a.value as usize;
                next.milestones = self.milestones.clone();
                next.blueprint = self.blueprint.clone();
                next.build_queue = next.blueprint.clone();
                next.research = self.research + award;
                next.discoveries = self.discoveries.clone();
                next.collection = self.collection.clone();
                next.ranks = self.ranks.clone();
                next.records = self.records.clone();
                let mut section = vec![0; 4096];
                let rows = (self.depth() as i64 * geometry::CELLS_PER_METRE).max(1);
                let left = self.heights.keys().next().copied().unwrap_or(0).min(0);
                let right = self
                    .heights
                    .keys()
                    .next_back()
                    .copied()
                    .unwrap_or(WIDTH - 1)
                    .max(WIDTH - 1);
                for y in 0..64 {
                    for x in 0..64 {
                        section[(y * 64 + x) as usize] = u8::from(
                            self.terrain
                                .contains(left + x * (right - left + 1) / 64, y * rows / 64),
                        );
                    }
                }
                next.records.push(Record {
                    section,
                    site: self.site,
                    depth: self.depth(),
                    research: award,
                    excavated: self.excavated,
                });
                next.last_saved = self.last_saved;
                next.megaproject = self.megaproject;
                next.enabled_recipes = self.enabled_recipes.clone();
                next.paused_recipes = self.paused_recipes.clone();
                next.policy = self.policy.clone();
                next.crew_priority = self.crew_priority.clone();
                next.cargo_policy = self.cargo_policy.clone();
                next.priorities = self.priorities.clone();
                if next.ranks.values().any(|r| *r >= 3) {
                    next.levels.insert("conveyor".into(), 1);
                }
                next.apply_headquarters();
                next.pinned = ["furnace", "steelworks", "shaft", "supports", "power"]
                    .into_iter()
                    .find(|id| next.level(id) == 0)
                    .map(str::to_string);
                *self = next;
            }
            _ => return Err("Unknown command".into()),
        }
        self.last_sequence = a.sequence;
        Ok(())
    }
    pub fn apply_headquarters(&mut self) {
        for (branch, grants) in &pacing::get().headquarters_starting {
            let rank = self.ranks.get(branch).copied().unwrap_or(0);
            for grant in grants.iter().filter(|grant| rank >= grant.rank) {
                for building in &grant.upgrades {
                    self.levels.entry(building.clone()).or_insert(1);
                }
            }
        }
    }
    pub fn migrate(&mut self) -> Result<(), String> {
        let old_version = self.version;
        match self.version {
            10 => {
                self.transport
                    .protect_routes(self.mining_fronts.iter().map(|front| front.route_id));
                let remap = self.transport.compact_routes()?;
                for front in &mut self.mining_fronts {
                    if front.route_id != 0 {
                        front.route_id = remap.get(&front.route_id).copied().unwrap_or(0);
                    }
                }
                self.version = VERSION;
            }
            11 => {
                self.version = VERSION;
            }
            VERSION => {}
            _ => {
                return Err(
                    "This save requires a fresh campaign; export or archive it first".into(),
                )
            }
        }
        for (index, contract) in self.contracts.iter().enumerate() {
            if contract.complete {
                self.site_objectives.insert(format!("order-{index}"));
            }
        }
        self.collection.extend(
            self.products
                .iter()
                .filter(|(_, q)| **q > 0)
                .map(|(p, _)| p.clone()),
        );
        self.terrain.rebuild()?;
        self.workings.blocked_at = None;
        if old_version < VERSION {
            self.movement.initialise(self.workers);
            self.transport.initialise_batch_ids();
        }
        self.validate()
    }
    pub fn validate(&self) -> Result<(), String> {
        if !self.movement.valid(self.workers) {
            return Err("Invalid worker travel state".into());
        }
        if self
            .specialisation
            .as_deref()
            .is_some_and(|s| !["bulk", "precision", "reclamation"].contains(&s))
        {
            return Err("Invalid specialisation".into());
        }
        if self.raw_stock.keys().any(|&id| id >= materials().len())
            || self
                .raw_stock
                .values()
                .any(|&q| q > self.raw_stock_capacity())
            || self.raw_stock.values().sum::<u64>() > self.raw_stock_capacity()
        {
            return Err("Invalid reserved feed stock".into());
        }
        if !self.workings.valid() {
            return Err("Invalid underground workings".into());
        }
        if !fronts::valid(
            &self.mining_fronts,
            materials().len(),
            self.max_mining_fronts(),
        ) {
            return Err("Invalid mining fronts".into());
        }
        if !self.transport.valid(materials().len()) || !self.transport.sources_match(&self.ore) {
            return Err("Invalid transport network".into());
        }
        if !self.waste_profile.valid() {
            return Err("Invalid waste profile".into());
        }
        if self
            .paused_recipes
            .iter()
            .any(|id| !matches!(id.as_str(), "steel" | "aluminium"))
        {
            return Err("Invalid paused automatic recipe".into());
        }
        if self.version != VERSION || !matches!(self.generator_version, 5 | 6) {
            return Err("Unsupported save version".into());
        }
        if ![
            "",
            "balanced",
            "digging",
            "hauling",
            "refining",
            "reclaiming",
            "engineering",
            "prospecting",
        ]
        .contains(&self.crew_priority.as_str())
            || !["", "balanced", "preferred"].contains(&self.cargo_policy.as_str())
            || !["", "hard_rock", "long_haul"].contains(&self.challenge.as_str())
            || self.profile >= sites().len()
            || self.heights.len() > 1_000_000
            || self
                .heights
                .iter()
                .any(|(x, h)| *h < 0 || !geometry::valid_cell(*x, h.saturating_sub(1).max(0)))
            || self.dig_remainder >= 20
            || self.dig_progress >= 1000
            || self.workers < 3
            || self.workers > 10000
            || self.housing < 8
            || self.housing > 10004
            || self.priorities.len() > 3
            || self.levels.values().any(|v| *v > 50)
            || self.ranks.values().any(|v| *v > 10)
            || self.removed.len() > 512
        {
            return Err("Invalid save state".into());
        }
        if self
            .terrain
            .chunks
            .iter()
            .any(|(id, bytes)| !geometry::valid_chunk(*id) || bytes.len() != 512)
        {
            return Err("Invalid terrain chunk".into());
        }
        let material_count = materials().len();
        if self
            .recovery_fraction
            .iter()
            .any(|(&id, &fraction)| id >= material_count || fraction >= 100)
        {
            return Err("Invalid recovery remainder".into());
        }
        for (&id, mask) in &self.terrain.revealed {
            if !geometry::valid_chunk(id) || mask.len() != 512 {
                return Err("Invalid reveal mask".into());
            }
            let pixels = self
                .terrain
                .visible
                .get(&id)
                .ok_or("Missing revealed geology")?;
            if pixels.len() != 4096
                || pixels.iter().enumerate().any(|(index, material)| {
                    let revealed = mask[index / 8] & (1 << (index % 8)) != 0;
                    if revealed {
                        *material as usize >= material_count
                    } else {
                        *material != 255
                    }
                })
            {
                return Err("Invalid visible geology".into());
            }
        }
        if self.terrain.visible.len() != self.terrain.revealed.len() {
            return Err("Unexpected geology chunks".into());
        }
        let n = materials().len();
        if self
            .products
            .values()
            .chain(self.ore.values())
            .chain(self.mining_fronts.iter().map(|front| &front.stockpile))
            .chain(self.hauled.values())
            .chain(self.concentrate.values())
            .chain(self.raw_stock.values())
            .chain(self.trace_feed.values())
            .chain(self.tailings.values())
            .chain(self.reserve.values())
            .any(|v| *v > 1_000_000_000_000)
        {
            return Err("Save inventory exceeds supported limits".into());
        }
        if self
            .removed
            .iter()
            .any(|c| !geometry::valid_cell(c.x, c.y) || c.material >= n)
            || self
                .ore
                .keys()
                .chain(self.hauled.keys())
                .chain(self.concentrate.keys())
                .chain(self.tailings.keys())
                .any(|id| *id >= n)
        {
            return Err("Invalid material reference".into());
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mining_front_commits_full_clearance_and_owns_every_removed_material() {
        let cat = materials();
        let mut g = Game::new(42, 1);
        g.ticks = 1; // Keep the explicit fixture front for this tick.
        g.workers = 12;
        g.workings.initialise();
        for y in 0..=300 {
            assert!(g.terrain.excavate(400, y));
        }
        let face = [401, 300];
        let newly_cut = fronts::clearance_cells(face)
            .iter()
            .filter(|[x, y]| !g.terrain.contains(*x, *y))
            .count() as u64;
        let material = g.cell(face[0], face[1], &cat);
        g.mining_fronts.push(fronts::MiningFront {
            id: "fixture".into(),
            deposit: "fixture".into(),
            material,
            face,
            position: [256, geometry::PIT_ROWS - 1],
            crew: 1,
            haulers: 0,
            // An older save can retain excessive credit. Only this face may use it.
            progress: 1_000_000_000,
            cut_work: 0,
            work_remainder: 0,
            transfer_remainder: 0,
            stockpile: 0,
            stockpiles: BTreeMap::new(),
            capacity: 100 * UNITS,
            route: vec![],
            route_id: 0,
            selected: true,
            status: "Excavating".into(),
            blocker: String::new(),
        });
        // Accounting fixture starts with arrived people; travel is tested separately.
        g.movement.initialise(g.workers);
        for worker in g.movement.workers.iter_mut().take(6) {
            worker.job = "dig:fixture".into();
            worker.role = "diggers".into();
            worker.position = [400, 300];
            worker.target = [400, 300];
            worker.activity = "digging".into();
        }
        g.tick(&cat, false);
        let front = &g.mining_fronts[0];
        let cleared = fronts::clearance_cells(face)
            .iter()
            .filter(|[x, y]| g.terrain.contains(*x, *y))
            .count();
        assert_eq!(cleared, 56);
        assert_eq!(front.position, face);
        assert_eq!(front.progress, 0);
        assert_eq!(front.stockpile, newly_cut * CELL_MASS);
        assert_eq!(front.stockpiles.values().sum::<u64>(), front.stockpile);
        assert!(front.stockpiles.len() >= 1);
    }

    #[test]
    fn processing_telemetry_reports_real_output_and_sales_reservations() {
        let mut g = Game::new(42, 1);
        g.action(Action {
            sequence: 1,
            kind: "buy".into(),
            target: "furnace".into(),
            value: 0,
        })
        .unwrap();
        g.concentrate.insert(3, UNITS);
        g.reserve.insert("iron".into(), UNITS);
        g.second(&materials(), false);
        let rows = g.processing();
        let iron = rows.iter().find(|f| f.id == 3).unwrap();
        assert!(iron.input_rate > 0. && iron.output_rate > 0.);
        assert_eq!(iron.reserved, iron.product.min(g.product_hold("iron")));
        assert_eq!(iron.product, g.products["iron"]);
        assert!(
            (iron.output_rate * UNITS as f64 - g.processing_window[&3][1] as f64).abs() < 0.001
        );
        assert_eq!(iron.recovery_percent, 65);
        assert_eq!(iron.reserve_target, 8 * UNITS);
        g.ore.insert(3, 17);
        g.transport.stations[0].cargo.insert(3, 23);
        let rows = g.processing();
        assert_eq!(rows.iter().find(|f| f.id == 3).unwrap().buffered, 40);
    }
    #[test]
    fn deterministic() {
        let mut a = Game::default();
        let mut b = a.clone();
        let cat = materials();
        for _ in 0..500 {
            a.second(&cat, false);
            b.second(&cat, false)
        }
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
    }
    #[test]
    fn offline_once() {
        let mut g = Game::default();
        g.last_saved = 100;
        g.advance_offline(200, &materials());
        let n = g.excavated;
        g.advance_offline(200, &materials());
        assert_eq!(n, g.excavated);
    }
    #[test]
    fn separate_returns_have_distinct_persisted_report_ids() {
        let mut g = Game::default();
        g.last_saved = 100;
        g.advance_offline(120, &materials());
        let first = g.offline.as_ref().unwrap().id.clone();
        let mut loaded: Game = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        assert_eq!(loaded.offline.as_ref().unwrap().id, first);
        loaded.advance_offline(140, &materials());
        assert_ne!(loaded.offline.as_ref().unwrap().id, first);
        assert_eq!(loaded.offline.as_ref().unwrap().effective, 10);
    }
    #[test]
    fn roundtrip() {
        let mut g = Game::default();
        g.second(&materials(), false);
        let copy: Game = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        copy.validate().unwrap();
        assert_eq!(copy.excavated, g.excavated);
    }
    #[test]
    fn no_duplicate_purchase() {
        let mut g = Game::default();
        let a = Action {
            sequence: 1,
            kind: "buy".into(),
            target: "worker".into(),
            value: 0,
        };
        g.action(a.clone()).unwrap();
        assert!(g.action(a).is_err());
        assert_eq!(g.workers, 4);
    }
    #[test]
    fn backwards_clock_cannot_reaward_an_already_consumed_interval() {
        let mut g = Game::default();
        g.last_saved = 100;
        g.advance_offline(120, &materials());
        let ticks = g.ticks;
        assert_eq!(ticks, 200);
        g.advance_offline(100, &materials());
        assert_eq!(g.last_saved, 120);
        assert_eq!(g.offline.as_ref().unwrap().effective, 0);
        g.advance_offline(120, &materials());
        assert_eq!(g.ticks, ticks);
        g.advance_offline(140, &materials());
        assert_eq!(g.ticks, ticks + 200);
        assert_eq!(g.offline.as_ref().unwrap().effective, 10);
    }
    #[test]
    fn offline_equivalent() {
        let mut a = Game::default();
        let mut b = a.clone();
        a.last_saved = 100;
        a.advance_offline(1100, &materials());
        for _ in 0..500 {
            b.second(&materials(), false)
        }
        assert_eq!(a.credits, b.credits);
        assert_eq!(a.excavated, b.excavated);
    }
}

#[cfg(test)]
mod accounting_tests {
    use super::*;
    #[test]
    fn all_material_is_accounted_for() {
        let cat = materials();
        let mut g = Game::default();
        for k in [
            "furnace",
            "chemical",
            "conveyor",
            "steelworks",
            "reclaimer",
            "electrolytic",
            "power",
        ] {
            g.levels.insert(k.into(), 1);
        }
        for _ in 0..3000 {
            g.second(&cat, false);
        }
        let mass: u64 = g.ore.values().sum::<u64>()
            + g.mining_fronts
                .iter()
                .map(|front| front.stockpile)
                .sum::<u64>()
            + g.hauled.values().sum::<u64>()
            + g.concentrate.values().sum::<u64>()
            + g.trace_feed.values().sum::<u64>()
            + g.products.values().sum::<u64>()
            + g.tailings.values().sum::<u64>()
            + g.sold_mass
            + g.transport.mass()
            + g.raw_stock.values().sum::<u64>()
            + g.delivered_mass
            + g.slag
            + g.depleted
            + g.disposed_mass;
        assert_eq!(mass, g.excavated * 1000);
        assert!(g.removed.len() <= 512);
    }
    #[test]
    fn aluminium_requires_two_modules() {
        let cat = materials();
        let id = cat.iter().find(|m| m.name == "Bauxite").unwrap().id;
        let mut g = Game::default();
        g.levels.insert("chemical".into(), 1);
        g.reserve.insert("alumina".into(), 10000 * 64);
        g.hauled.insert(id, 1000 * 64);
        for _ in 0..10 {
            g.second(&cat, false);
        }
        assert!(g.products.get("alumina").copied().unwrap_or(0) > 0);
        assert_eq!(g.products.get("aluminium").copied().unwrap_or(0), 0);
        g.levels.insert("electrolytic".into(), 1);
        g.reserve.insert("aluminium".into(), 10000 * 64);
        g.second(&cat, false);
        assert!(g.products.get("aluminium").copied().unwrap_or(0) > 0);
    }
    #[test]
    fn offline_caps_and_clock_rollback() {
        let mut g = Game::default();
        g.last_saved = 1000;
        g.advance_offline(900, &materials());
        assert_eq!(g.offline.as_ref().unwrap().effective, 0);
        g.last_saved = 1000;
        g.advance_offline(100000, &materials());
        assert_eq!(g.offline.as_ref().unwrap().effective, 14400);
    }
    #[test]
    fn recipe_references_are_available() {
        let mut available: BTreeSet<_> = materials().iter().map(|m| m.product.clone()).collect();
        available.insert("alumina".into());
        available.extend(traces().iter().map(|r| format!("{}_residue", r.output)));
        let recipes = recipes();
        available.extend(recipes.iter().map(|r| r.output.clone()));
        for recipe in recipes {
            assert!(recipe.inputs.values().all(|v| *v > 0));
            for input in recipe.inputs.keys() {
                assert!(available.contains(input), "missing {input}");
            }
        }
    }
}

#[cfg(test)]
mod pipeline_tests {
    use super::*;
    #[test]
    fn refinery_backpressure_stops_sorting() {
        let mut g = Game::default();
        g.concentrate.insert(2, g.raw_stock_capacity());
        g.hauled.insert(2, 1000 * 64);
        g.tick(&materials(), false);
        assert_eq!(g.flow_window[2], 0);
        assert_eq!(g.hauled[&2], 1000 * 64);
        assert!(g.concentrate[&2] < g.raw_stock_capacity());
        g.tick(&materials(), false);
        assert!(g.flow_window[2] > 0);
    }
}

#[cfg(test)]
mod site_tests {
    use super::*;
    #[test]
    fn every_profile_guarantees_steel_feed() {
        let cat = materials();
        for profile in 0..sites().len() {
            let mut g = Game::default();
            g.profile = profile;
            for (id, y) in [(3, 24), (5, 76), (6, 130)] {
                assert!((200..310).any(|x| g.cell(x, y, &cat) == id));
            }
        }
    }
    #[test]
    fn site_profiles_change_deposits() {
        let cat = materials();
        let a = Game::default();
        let mut b = a.clone();
        b.profile = 1;
        assert!((0..512).step_by(3).any(|x| (200..2000)
            .step_by(7)
            .any(|y| a.cell(x, y, &cat) != b.cell(x, y, &cat))));
    }
}

#[cfg(test)]
mod trace_tests {
    use super::*;
    #[test]
    fn trace_recovery_cannot_transmute_bulk_metal() {
        let mut g = Game::default();
        g.levels.insert("trace".into(), 1);
        g.enabled_recipes.insert("separate_gallium".into());
        g.products.insert("alumina".into(), 10000 * 64);
        g.reserve.insert("gallium".into(), 10000 * 64);
        g.tick(&materials(), false);
        assert_eq!(g.products.get("gallium").copied().unwrap_or(0), 0);
        g.trace_feed.insert("gallium_residue".into(), 100);
        g.tick(&materials(), false);
        assert!(g.products["gallium"] > 0);
        assert_eq!(g.products["gallium"] + g.trace_feed["gallium_residue"], 100);
    }
    #[test]
    fn trace_fractions_never_exceed_feed() {
        for id in 0..materials().len() {
            assert!(
                traces()
                    .iter()
                    .filter(|r| r.feed == id)
                    .map(|r| r.permille)
                    .sum::<u64>()
                    <= 1000
            );
        }
    }
}

#[cfg(test)]
mod waste_tests {
    use super::*;
    #[test]
    fn disposal_reduces_only_depleted_stock() {
        let mut g = Game::default();
        g.depleted = 100;
        g.slag = 100;
        g.tailings.insert(2, 100);
        g.tick(&materials(), false);
        assert_eq!(g.depleted, 0);
        assert_eq!(g.disposed_mass, 100);
        assert_eq!(g.slag, 100);
        assert_eq!(g.tailings[&2], 100);
    }
}

#[cfg(test)]
mod upgrade_tests {
    use super::*;
    #[test]
    fn unavailable_materials_do_not_spend_credits() {
        let mut g = Game::default();
        g.credits = 10000;
        g.levels.insert("furnace".into(), 1);
        let a = Action {
            sequence: 1,
            kind: "buy".into(),
            target: "shaft".into(),
            value: 0,
        };
        assert!(g.action(a.clone()).is_err());
        assert_eq!(g.credits, 10000);
        g.products.insert("iron".into(), 2000 * 64);
        g.action(a).unwrap();
        assert_eq!(g.products["iron"], 0);
        assert_eq!(g.delivered_mass, 2 * UNITS);
    }
    #[test]
    fn pin_holds_upgrade_material() {
        let mut g = Game::default();
        g.pinned = Some("shaft".into());
        g.products.insert("iron".into(), 3000 * 64);
        g.contracts.clear();
        g.tick(&materials(), false);
        assert_eq!(g.products["iron"], 2000 * 64);
    }
}

#[cfg(test)]
mod progression_tests {
    use super::*;
    #[test]
    fn blueprint_does_not_spend_offline() {
        let mut g = Game::default();
        g.credits = 10000;
        g.build_queue = vec!["furnace".into()];
        g.second(&materials(), true);
        assert_eq!(g.level("furnace"), 0);
        g.second(&materials(), false);
        assert_eq!(g.level("furnace"), 1);
        assert!(g.build_queue.is_empty());
    }
    #[test]
    fn headquarters_tiers_apply_to_fresh_site() {
        let mut g = Game::default();
        g.ranks.insert("logistics".into(), 6);
        g.apply_headquarters();
        assert_eq!(g.level("wheelbarrow"), 1);
        assert_eq!(g.level("conveyor"), 0);
        assert_eq!(g.level("minecart"), 0);
        assert_eq!(g.level("train"), 0);
    }
    #[test]
    fn milestones_award_only_once() {
        let mut g = Game::default();
        g.levels.insert("conveyor".into(), 1);
        g.tick(&materials(), false);
        let research = g.research;
        g.tick(&materials(), false);
        assert_eq!(g.research, research);
    }
}

#[cfg(test)]
mod hardness_tests {
    use super::*;
    #[test]
    fn deeper_bands_need_more_work_and_drill_tiers_help() {
        let mut g = Game::default();
        let shallow = g.rock_work();
        g.heights.insert((0) as i64, 1200);
        let deep = g.rock_work();
        assert!(deep > shallow);
        g.levels.insert("drill".into(), 10);
        assert!((g.rock_work() * 3. - deep).abs() < 0.0001);
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    #[test]
    fn repeat_orders_pay_premium_without_farming_research() {
        let mut g = Game::default();
        let price = materials()
            .iter()
            .find(|m| m.product == "iron")
            .unwrap()
            .price;
        g.products.insert("iron".into(), 10000 * 64);
        let before = g.credits;
        g.action(Action {
            sequence: 1,
            kind: "contract".into(),
            target: String::new(),
            value: 0,
        })
        .unwrap();
        assert_eq!(g.credits - before, price * 2 * 5 / 4);
        assert_eq!(g.research, 1);
        g.action(Action {
            sequence: 2,
            kind: "new_contract".into(),
            target: String::new(),
            value: 0,
        })
        .unwrap();
        assert_eq!(g.contracts[0].amount, 3 * UNITS);
        g.action(Action {
            sequence: 3,
            kind: "contract".into(),
            target: String::new(),
            value: 0,
        })
        .unwrap();
        assert_eq!(g.research, 1);
        assert_eq!(g.site_objectives.len(), 1);
        assert_eq!(g.delivered_mass, 5 * UNITS);
    }
}

#[cfg(test)]
mod specialisation_tests {
    use super::*;
    #[test]
    fn specialisation_is_site_locked_and_reclamation_cannot_duplicate() {
        let mut g = Game::default();
        let choose = |sequence| Action {
            sequence,
            kind: "specialise".into(),
            target: "reclamation".into(),
            value: 0,
        };
        assert!(g.action(choose(1)).is_err());
        g.heights.insert(
            (256) as i64,
            pacing::get().specialisation_depth as i64 * geometry::CELLS_PER_METRE,
        );
        assert!(g.action(choose(2)).is_err());
        g.steel_made = true;
        g.action(choose(3)).unwrap();
        assert!(g.action(choose(4)).is_err());
        g.levels.insert("reclaimer".into(), 1);
        g.tailings.insert(0, 2);
        g.second(&materials(), true);
        assert_eq!(g.tailings.get(&0).copied().unwrap_or(0), 0);
        assert!(g.hauled.get(&0).copied().unwrap_or(0) <= 2);
        let restored: Game = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        assert_eq!(restored.specialisation.as_deref(), Some("reclamation"));
    }
}

#[cfg(test)]
mod throughput_tests {
    use super::*;
    #[test]
    fn full_haul_buffer_reports_no_transfer() {
        let mut g = Game::default();
        g.workings.initialise();
        g.removed.push(Cell {
            x: 256,
            y: 1,
            material: 0,
        });
        g.tick(&materials(), false);
        g.flow_window[1] = 0;
        g.hauled.insert(0, g.raw_stock_capacity());
        g.ore.insert(0, 1000 * 64);
        let station_capacity = g.transport.stations[0].capacity;
        g.transport.stations[0].cargo.insert(0, station_capacity);
        let segment_capacity = g.transport.segments[0].capacity;
        g.transport.segments[0].batches.push(transport::Batch {
            id: 0,
            route: g.transport.current_route,
            material: 0,
            amount: segment_capacity,
            remaining_ms: 10000,
            duration_ms: 10000,
        });
        g.tick(&materials(), false);
        assert_eq!(g.flow_window[1], 0);
        assert!(g.transport.mass() > 0);
        assert_eq!(g.stages[1].blocker, "Short haul: Vehicle capacity full");
    }
}

#[cfg(test)]
mod challenge_tests {
    use super::*;
    #[test]
    fn retirement_matches_preview_and_resets_site_choices() {
        let mut g = Game::default();
        g.heights.insert((256) as i64, 1200);
        g.steel_made = true;
        g.challenge = "hard_rock".into();
        g.specialisation = Some("bulk".into());
        g.tick(&materials(), false);
        let award = g.award();
        let research = g.research;
        assert!(g.site_objectives.contains("challenge"));
        g.action(Action {
            sequence: 1,
            kind: "retire".into(),
            target: "long_haul".into(),
            value: 1,
        })
        .unwrap();
        assert_eq!(g.research, research + award);
        assert_eq!(g.records.last().unwrap().research, award);
        assert_eq!(g.challenge, "long_haul");
        assert!(g.specialisation.is_none());
        assert!(g.site_objectives.is_empty());
    }
}

#[cfg(test)]
mod offline_idle_tests {
    use super::*;
    #[test]
    fn blocked_interval_skip_matches_every_tick() {
        let cat = materials();
        let mut a = Game::default();
        a.heights = (0..WIDTH).map(|x| (x, 400)).collect();
        a.terrain = terrain::Terrain::from_heights(&a.heights);
        a.dig_progress = 127;
        a.dig_remainder = 7;
        a.last_saved = 100;
        let mut b = a.clone();
        a.advance_offline(2100, &cat);
        for _ in 0..1000 {
            b.second(&cat, true);
        }
        b.last_saved = a.last_saved;
        b.offline = a.offline.clone();
        assert_eq!(
            serde_json::to_value(&a).unwrap(),
            serde_json::to_value(&b).unwrap()
        );
    }
    #[test]
    fn pending_arrival_prevents_idle_skip() {
        let mut g = Game::default();
        g.heights = (0..WIDTH).map(|x| (x, 400)).collect();
        g.terrain = terrain::Terrain::from_heights(&g.heights);
        g.transport.segments[0].batches.push(transport::Batch {
            id: 0,
            route: 0,
            material: 0,
            amount: 1000,
            remaining_ms: 30000,
            duration_ms: 30000,
        });
        assert!(!g.quiescent(&materials()));
    }
}

#[cfg(test)]
mod fractional_work_tests {
    use super::*;
    #[test]
    fn slow_deep_work_does_not_round_to_zero() {
        let mut g = Game::default();
        g.heights = (0..WIDTH).map(|x| (x, 24000)).collect();
        let rate = g.dig_rate(true);
        assert!(rate > 0 && rate % 20 != 0);
        g.second(&materials(), true);
        assert_eq!(g.dig_progress, rate % 1000);
    }
}

#[cfg(test)]
mod wire_tests {
    use super::*;
    #[test]
    fn seeds_keep_all_bits_and_legacy_numbers_load() {
        let g = Game::new(u64::MAX, 1);
        let mut value = serde_json::to_value(&g).unwrap();
        assert_eq!(value["seed"], u64::MAX.to_string());
        assert_eq!(
            serde_json::from_value::<Game>(value.clone()).unwrap().seed,
            u64::MAX
        );
        value["seed"] = serde_json::json!(73429);
        assert_eq!(serde_json::from_value::<Game>(value).unwrap().seed, 73429);
    }
}

#[cfg(test)]
mod construction_tests {
    use super::*;
    #[test]
    fn engineers_commission_local_sections_before_navigation_opens() {
        let mut w = workings::Workings::default();
        w.initialise();
        let mut terrain = terrain::Terrain::from_columns(&vec![196; WIDTH as usize]);
        w.section = Some(workings::Section {
            from: 0,
            to: [260, 192],
            lift: false,
            cells: workings::cut_cells([256, 191], [260, 192], false),
            support_work: 0,
        });
        w.advance(&terrain, &[], "depth", 1200, 1, 0);
        assert_eq!(w.passages.len(), 1);
        assert_eq!(w.status, "Waiting for supports");
        for _ in 1..workings::settings().support_work.div_ceil(80) {
            w.advance(&terrain, &[], "depth", 1200, 1, 0);
        }
        assert!(w.passages.len() > 1);
        assert!(w.passages[1].supported);
        // A rock cell in the clearance keeps the next section uncommissioned.
        terrain = terrain::Terrain::default();
        w.section = Some(workings::Section {
            from: 1,
            to: [264, 193],
            lift: false,
            cells: vec![[264, 193]],
            support_work: 0,
        });
        for _ in 0..20 {
            w.advance(&terrain, &[], "depth", 1200, 10, 10);
        }
        assert_eq!(w.passages.len(), 2);
    }
}

#[cfg(test)]
mod retirement_quote_tests {
    use super::*;
    #[test]
    fn production_between_preview_and_confirmation_does_not_change_quote() {
        let mut g = Game::default();
        g.heights.insert((256) as i64, 1200);
        g.steel_made = true;
        g.action(Action {
            sequence: 1,
            kind: "retirement_preview".into(),
            target: String::new(),
            value: 0,
        })
        .unwrap();
        let quoted = g.retirement_quote.unwrap();
        g.site_discoveries += 1;
        assert!(g.award() > quoted);
        g.action(Action {
            sequence: 2,
            kind: "retire".into(),
            target: String::new(),
            value: 0,
        })
        .unwrap();
        assert_eq!(g.records[0].research, quoted);
        assert_eq!(g.research, quoted);
        assert!(g.retirement_quote.is_none());
    }
}

#[cfg(test)]
mod preview_tests {
    use super::*;
    #[test]
    fn forecast_distinguishes_machine_gain_from_line_bottleneck() {
        let mut g = Game::default();
        // Keep processing above hauling so this fixture measures a transport bottleneck.
        for machine in ["sorter", "furnace", "power"] {
            g.levels.insert(machine.into(), 8);
        }
        let offers = g.upgrade_previews();
        assert!(offers["conveyor"].machine_percent > offers["conveyor"].line_percent);
        assert!(offers["conveyor"].line_percent > 0.);
        assert!(offers["drill"].machine_percent > 0.);
        assert_eq!(offers["drill"].line_percent, 0.);
    }
}

#[cfg(test)]
mod collection_tests {
    use super::*;
    #[test]
    fn trace_products_remain_collected_after_sale_and_retirement() {
        let mut g = Game::default();
        g.levels.insert("trace".into(), 1);
        g.enabled_recipes.insert("separate_neodymium".into());
        g.trace_feed.insert("neodymium_residue".into(), 12);
        g.tick(&materials(), true);
        assert!(g.collection.contains("neodymium"));
        assert_eq!(g.products.get("neodymium").copied().unwrap_or(0), 0);
        g.heights.insert((256) as i64, 1200);
        g.steel_made = true;
        g.action(Action {
            sequence: 1,
            kind: "retire".into(),
            target: String::new(),
            value: 0,
        })
        .unwrap();
        assert!(g.collection.contains("neodymium"));
        assert!(g.products.is_empty());
    }
}

#[cfg(test)]
mod recipe_power_tests {
    use super::*;
    #[test]
    fn fractional_budget_preserves_slow_and_fast_work_without_overflow() {
        for rate in [0, 1, 125, 12500, 95123] {
            assert_eq!(
                (1..=1000).map(|tick| work_budget(rate, tick)).sum::<u64>(),
                rate
            );
            assert!(work_budget(rate, u64::MAX) <= rate / 1000 + 1);
        }
    }
    #[test]
    fn recipe_power_and_module_upgrades_change_output_without_creating_mass() {
        fn output(power: u32, module: u32) -> u64 {
            let mut g = Game::default();
            g.levels.insert("manufacturing".into(), module);
            g.levels.insert("trace".into(), 5);
            g.levels.insert("power".into(), power);
            g.enabled_recipes.insert("bronze".into());
            g.products.insert("copper".into(), 6000 * 64);
            g.products.insert("tin".into(), 2000 * 64);
            g.reserve.insert("bronze".into(), 8000 * 64);
            let cat = materials();
            for _ in 0..20 {
                g.tick(&cat, true);
            }
            let bronze = g.products.get("bronze").copied().unwrap_or(0);
            assert_eq!(g.products["copper"] + g.products["tin"] + bronze, 8 * UNITS);
            bronze
        }
        let constrained = output(0, 1);
        let powered = output(10, 1);
        let upgraded = output(10, 10);
        assert!(constrained > 0 && constrained < powered);
        assert!(powered < upgraded);
    }
}

#[cfg(test)]
mod pinned_recipe_tests {
    use super::*;
    #[test]
    fn automatic_steel_respects_iron_committed_to_shaft() {
        let mut g = Game::default();
        g.levels.insert("furnace".into(), 1);
        g.levels.insert("steelworks".into(), 1);
        g.products.insert("iron".into(), 2000 * 64);
        g.products.insert("coke".into(), 8000 * 64);
        g.products.insert("lime".into(), 8000 * 64);
        g.pinned = Some("shaft".into());
        g.credits = 1000;
        g.second(&materials(), true);
        assert_eq!(g.products["iron"], 2000 * 64);
        assert!(!g.steel_made);
        g.action(Action {
            sequence: 1,
            kind: "buy".into(),
            target: "shaft".into(),
            value: 0,
        })
        .unwrap();
        assert_eq!(g.level("shaft"), 1);
        assert_eq!(g.products["iron"], 0);
    }
}

#[cfg(test)]
mod offline_event_tests {
    use super::*;
    #[test]
    fn event_skips_match_every_tick_across_sites_policies_and_cargo() {
        let cat = materials();
        for profile in 0..sites().len() {
            for (index, policy) in ["bulk", "vein", "depth"].iter().enumerate() {
                let mut a = Game::default();
                a.profile = profile;
                a.policy = (*policy).into();
                a.cargo_policy = "preferred".into();
                a.priorities = vec![3, 5, 6];
                a.challenge = "hard_rock".into();
                a.dig_progress = 127;
                a.dig_remainder = 7;
                a.ticks = index as u64 * 7;
                a.last_saved = 100;
                if index > 0 {
                    a.levels.insert("conveyor".into(), 1);
                    a.levels.insert("furnace".into(), 1);
                }
                a.transport.segments[0].batches.push(transport::Batch {
                    id: 0,
                    route: 0,
                    material: 3,
                    amount: 1000,
                    remaining_ms: 31000,
                    duration_ms: 31000,
                });
                let mut b = a.clone();
                a.advance_offline(700, &cat);
                for _ in 0..6000 {
                    b.tick(&cat, true);
                }
                b.last_saved = a.last_saved;
                b.offline = a.offline.clone();
                assert_eq!(
                    serde_json::to_value(&a).unwrap(),
                    serde_json::to_value(&b).unwrap(),
                    "profile {profile}, policy {policy}"
                );
            }
        }
    }
}

#[cfg(test)]
mod shaft_access_tests {
    use super::*;
    #[test]
    fn lift_preview_preserves_independent_support_water_and_heat_gates() {
        let mut g = Game::default();
        assert_eq!(g.lift_depth_limit(), 300);
        g.levels.insert("shaft".into(), 10);
        assert_eq!(
            g.lift_depth_limit(),
            300 + 10 * pacing::get().lift_depth_per_level
        );
        assert_eq!(g.equipment_depth_limit(), 300);
        g.levels.insert("supports".into(), 1);
        assert_eq!(g.equipment_depth_limit(), 700);
        g.levels.insert("pump".into(), 1);
        assert_eq!(g.equipment_depth_limit(), 1500);
        let previews = g.upgrade_previews();
        assert_eq!(previews["shaft"].access_depth_after, Some(1500));
        assert_eq!(
            previews["shaft"].lift_depth_after,
            Some(300 + 11 * pacing::get().lift_depth_per_level)
        );
        g.levels.insert("ventilation".into(), 1);
        assert_eq!(g.equipment_depth_limit(), g.lift_depth_limit());
    }

    #[test]
    fn basic_supported_access_requires_no_iron_or_shaft_purchase() {
        let mut g = Game::default();
        let cat = materials();
        for _ in 0..30 {
            g.second(&cat, false);
        }
        assert_eq!(g.level("shaft"), 0);
        assert!(g.excavated > 0);
        assert!(g.sold_mass > 0);
        assert!(g.workings.passages.len() > 1);
        assert_eq!(g.workings.passages[0].feet, [256, 0]);
        assert!(!navigation::underground(&g.terrain, &g.heights, &g.workings).is_empty());
    }
}

#[cfg(test)]
mod fine_recovery_tests {
    use super::*;
    #[test]
    fn single_quantum_tailings_finish_reprocessing_without_loss_or_loop() {
        let mut game = Game::default();
        game.terrain.frontier.clear();
        game.levels.insert("furnace".into(), 1);
        game.levels.insert("reclaimer".into(), 1);
        game.tailings.insert(3, 1);
        for _ in 0..100 {
            game.tick(&materials(), false);
        }
        assert_eq!(game.tailings.get(&3).copied().unwrap_or(0), 0);
        assert_eq!(game.products.get("iron").copied().unwrap_or(0), 1);
    }
}

#[cfg(test)]
mod reserved_feed_tests {
    use super::*;
    #[test]
    fn locked_feed_is_owned_once_and_released_after_module_purchase() {
        let mut g = Game::default();
        g.terrain.frontier.clear();
        g.concentrate.insert(4, UNITS);
        g.reserve.insert("copper".into(), UNITS);
        let cat = materials();
        for _ in 0..40 {
            g.tick(&cat, false);
        }
        assert_eq!(g.raw_stock[&4], UNITS);
        assert_eq!(g.sold_mass, 0);
        g.levels.insert("chemical".into(), 1);
        for _ in 0..80 {
            g.tick(&cat, false);
        }
        assert_eq!(g.raw_stock[&4], 0);
        assert!(g.products["copper"] > 0);
        let total = g.products.values().sum::<u64>()
            + g.tailings.values().sum::<u64>()
            + g.trace_feed.values().sum::<u64>()
            + g.slag
            + g.depleted
            + g.disposed_mass
            + g.sold_mass;
        assert_eq!(total, UNITS);
    }
}

#[cfg(test)]
mod stationary_network_tests {
    use super::*;
    #[test]
    fn stationary_loaded_stockpile_matches_fixed_steps() {
        let mut a = Game::default();
        a.terrain.frontier.clear();
        a.ore.insert(1, 20 * UNITS);
        a.last_saved = 100;
        let mut b = a.clone();
        let cat = materials();
        a.advance_offline(500, &cat);
        for _ in 0..4000 {
            b.tick(&cat, true);
        }
        a.offline = None;
        b.last_saved = a.last_saved;
        assert_eq!(
            serde_json::to_value(a).unwrap(),
            serde_json::to_value(b).unwrap()
        );
    }
}

#[cfg(test)]
mod survey_offline_tests {
    use super::*;
    #[test]
    fn blocked_idle_intervals_finish_pending_vein_accuracy() {
        let cat = materials();
        let mut offline = Game::new(42, 1);
        offline.last_saved = 1;
        offline.levels.insert("survey".into(), 2);
        offline.workings.initialise();
        offline
            .workings
            .survey(&mut offline.terrain, 42, 0, cat, true, false);
        assert!(offline.workings.veins.values().any(|v| v.stage == 0));
        offline.terrain.frontier.clear();
        offline.terrain.ore_frontiers.clear();
        offline.terrain.access_frontier.clear();
        offline.workings.blocked_at =
            Some((offline.terrain.revision, offline.workings.revision, 1200));
        let mut stepped = offline.clone();
        offline.advance_offline(241, cat);
        for _ in 0..2400 {
            stepped.tick(cat, true);
        }
        assert!(stepped.workings.veins.values().any(|v| v.stage == 2));
        assert_eq!(offline.terrain.revealed, stepped.terrain.revealed);
        assert_eq!(offline.terrain.visible, stepped.terrain.visible);
        assert_eq!(
            serde_json::to_value(&offline.workings.veins).unwrap(),
            serde_json::to_value(&stepped.workings.veins).unwrap()
        );
    }
    #[test]
    fn known_commodities_do_not_skip_local_prospecting() {
        let cat = materials();
        let mut offline = Game::new(42, 1);
        offline.last_saved = 1;
        offline.second(&cat, true);
        offline.levels.insert("survey".into(), 1);
        offline.discoveries = (0..cat.len()).collect();
        offline.terrain.frontier.clear();
        assert!(offline.survey_pending());
        let mut stepped = offline.clone();
        offline.advance_offline(241, &cat);
        for _ in 0..2400 {
            stepped.tick(&cat, true);
        }
        assert!(!offline.survey_pending());
        assert_eq!(offline.terrain.revealed, stepped.terrain.revealed);
        assert_eq!(offline.terrain.visible, stepped.terrain.visible);
        assert_eq!(offline.ticks, stepped.ticks);
    }
}

#[cfg(test)]
mod vein_inspection_tests {
    use super::*;
    #[test]
    fn mined_inspector_anchor_can_select_remaining_surveyed_vein() {
        let mut game = Game::new(42, 1);
        let cat = materials();
        let anchor = (220..250)
            .map(|x| [x, 24])
            .find(|p| geology::sample(42, 0, p[0], p[1], cat) == 3)
            .unwrap();
        for y in 0..=anchor[1] {
            assert!(game.terrain.excavate(anchor[0], y));
        }
        game.terrain.reveal(42, 0, anchor[0], anchor[1], 0, cat);
        let command = || Action {
            sequence: 1,
            kind: "target_vein".into(),
            target: format!("{},{}", anchor[0], anchor[1]),
            value: 0,
        };
        assert!(game.action(command()).is_err());
        assert_eq!(game.workings.target, None);
        assert_eq!(game.last_sequence, 0);
        game.terrain.reveal(42, 0, anchor[0], anchor[1], 4, cat);
        game.action(command()).unwrap();
        assert_eq!(game.workings.target, Some(anchor));
        assert!(game.workings.order_view(&game.terrain).unwrap().known_cells > 0);
    }
}

#[cfg(test)]
mod recipe_fairness_tests {
    use super::*;
    #[test]
    fn automatic_recipe_pause_preserves_feed_and_round_trips() {
        let mut g = Game::new(42, 1);
        g.terrain.frontier.clear();
        g.levels.insert("steelworks".into(), 1);
        for p in ["iron", "coke", "lime"] {
            g.products.insert(p.into(), 4 * UNITS);
            g.reserve.insert(p.into(), 4 * UNITS);
        }
        g.reserve.insert("steel".into(), 20 * UNITS);
        g.action(Action {
            sequence: 1,
            kind: "recipe".into(),
            target: "steel".into(),
            value: 0,
        })
        .unwrap();
        g.second(&materials(), true);
        assert_eq!(g.products["iron"], 4 * UNITS);
        assert_eq!(g.products.get("steel").copied().unwrap_or(0), 0);
        let restored: Game = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        assert!(restored.paused_recipes.contains("steel"));
        g.action(Action {
            sequence: 2,
            kind: "recipe".into(),
            target: "steel".into(),
            value: 0,
        })
        .unwrap();
        g.second(&materials(), true);
        assert!(g.products["steel"] > 0);
    }
    #[test]
    fn steel_cannot_starve_enabled_magnet_production() {
        let mut g = Game::new(42, 1);
        g.terrain.frontier.clear();
        g.levels.insert("steelworks".into(), 1);
        g.levels.insert("manufacturing".into(), 1);
        g.enabled_recipes.insert("magnets".into());
        for p in ["coke", "lime", "neodymium", "praseodymium", "borate"] {
            g.products.insert(p.into(), 20 * UNITS);
            g.reserve.insert(p.into(), 20 * UNITS);
        }
        for p in ["steel", "magnets"] {
            g.reserve.insert(p.into(), 20 * UNITS);
        }
        let cat = materials();
        for _ in 0..100 {
            *g.products.entry("iron".into()).or_default() += 640;
            g.tick(&cat, true);
        }
        assert!(g.products.get("steel").copied().unwrap_or(0) > 0);
        assert!(g.products.get("magnets").copied().unwrap_or(0) > 0);
    }
}

#[cfg(test)]
mod campaign_research_tests {
    use super::*;
    #[test]
    fn industry_requires_retained_research_investment_without_spending_it_again() {
        let mut g = Game::new(42, 1);
        g.credits = 10000;
        g.levels.insert("steelworks".into(), 1);
        g.products.insert("steel".into(), 10 * UNITS);
        let purchase = || Action {
            sequence: 1,
            kind: "buy".into(),
            target: "power".into(),
            value: 0,
        };
        assert!(g.action(purchase()).is_err());
        assert_eq!(g.credits, 10000);
        assert_eq!(g.products["steel"], 10 * UNITS);
        g.ranks.insert("excavation".into(), 3);
        assert_eq!(g.research_invested(), 70);
        g.action(purchase()).unwrap();
        assert_eq!(g.research_invested(), 70);
        assert_eq!(g.level("power"), 1);
        assert!(g.purchase_blocker("chemical").unwrap().contains("research"));
    }
    #[test]
    fn headquarters_requires_research_and_components_atomically() {
        let mut g = Game::new(42, 1);
        for p in [
            "advanced_structure",
            "precision_controls",
            "magnets",
            "batteries",
        ] {
            g.products.insert(p.into(), 10 * UNITS);
        }
        let command = || Action {
            sequence: 1,
            kind: "megaproject".into(),
            target: String::new(),
            value: 0,
        };
        let products = g.products.clone();
        assert!(g.action(command()).is_err());
        assert_eq!(g.products, products);
        g.ranks.insert("metallurgy".into(), 7);
        g.ranks.insert("excavation".into(), 4);
        assert_eq!(g.research_invested(), 850);
        g.action(command()).unwrap();
        assert!(g.megaproject);
        assert_eq!(g.products.values().sum::<u64>(), 0);
    }
}

#[cfg(test)]
mod retired_start_tests {
    use super::*;
    #[test]
    fn steelworks_rebuild_discount_preserves_first_site_prices_and_stock() {
        let mut game = Game::new(42, 1);
        let stock = game.products.clone();
        assert_eq!(game.cost("steelworks"), 600);
        let shaft_price = game.cost("shaft");
        game.site = 2;
        assert_eq!(game.cost("steelworks"), 200);
        assert_eq!(game.cost("shaft"), shaft_price);
        assert_eq!(game.products, stock);
        game.levels.insert("steelworks".into(), 1);
        assert_eq!(game.cost("steelworks"), 236);
    }
    #[test]
    fn researched_furnace_keeps_starter_construction_feed_while_offline() {
        let mut g = Game::new(46, 1);
        g.ranks.insert("metallurgy".into(), 3);
        g.heights
            .insert((256) as i64, 300 * geometry::CELLS_PER_METRE);
        g.steel_made = true;
        g.action(Action {
            sequence: 1,
            kind: "retire".into(),
            target: String::new(),
            value: 0,
        })
        .unwrap();
        assert_eq!(g.pinned.as_deref(), Some("furnace"));
        assert_eq!(g.level("sorter"), 1);
        assert_eq!(g.level("power"), 0);
        g.levels.insert("furnace".into(), 1);
        g.pinned = Some("steelworks".into());
        g.last_saved = 1;
        g.advance_offline(2401, &materials());
        assert!(g.products.get("iron").copied().unwrap_or(0) >= 4 * UNITS);
        assert!(g.products.get("coke").copied().unwrap_or(0) >= UNITS);
        assert!(g.products.get("lime").copied().unwrap_or(0) >= UNITS);
        // This contract verifies protected construction feed, independent of
        // the separate credit-earning pace exercised by opening scenarios.
        g.credits = 100_000;
        for target in ["steelworks", "shaft"] {
            g.action(Action {
                sequence: g.last_sequence + 1,
                kind: "buy".into(),
                target: target.into(),
                value: 0,
            })
            .unwrap();
        }
    }
}

#[cfg(test)]
mod travel_replay_tests {
    use super::*;
    #[test]
    fn survey_reassignment_has_no_remote_work_credit() {
        let mut game = Game::new(42, 1);
        game.workings.initialise();
        game.movement.workers[0].job = "survey".into();
        game.movement.workers[0].activity = "surveying".into();
        assert_eq!(game.surveying_crew(), 1);
        game.workings.passages.push(workings::Passage {
            feet: [256, 32],
            parent: 0,
            lift: true,
            supported: true,
            column: false,
        });
        game.workings.active = 1;
        assert_eq!(game.movement.working("survey"), 1);
        assert_eq!(game.surveying_crew(), 0);
    }
    #[test]
    fn captured_offline_travel_matches_fixed_steps() {
        let mut captured = Game::new(42, 1);
        captured.last_saved = 100;
        let mut stepped = captured.clone();
        let mut previous = 0;
        let mut frames = 0;
        captured.advance_offline_observed(300, materials(), |game, done, total| {
            assert!(done >= previous && done <= total);
            previous = done;
            frames += 1;
            assert_eq!(game.movement.workers.len(), game.workers as usize);
        });
        for _ in 0..2000 {
            stepped.tick(materials(), true);
        }
        stepped.last_saved = captured.last_saved;
        stepped.offline = captured.offline.clone();
        assert!(frames > 2);
        assert_eq!(
            serde_json::to_value(&captured).unwrap(),
            serde_json::to_value(&stepped).unwrap()
        );
    }
    #[test]
    fn version_eleven_migrates_identities_without_changing_mine() {
        let mut game = Game::new(42, 1);
        game.generator_version = 5;
        for _ in 0..10 {
            game.second(materials(), false);
        }
        let terrain = serde_json::to_value(&game.terrain).unwrap();
        let excavated = game.excavated;
        let inventory = game.ore.clone();
        game.version = 11;
        game.movement = crew::Movement::default();
        game.migrate().unwrap();
        assert_eq!(game.version, 12);
        assert_eq!(game.generator_version, 5);
        assert_eq!(game.movement.workers.len(), game.workers as usize);
        assert_eq!(serde_json::to_value(&game.terrain).unwrap(), terrain);
        assert_eq!(game.excavated, excavated);
        assert_eq!(game.ore, inventory);
    }
    #[test]
    fn empty_travel_skip_preserves_every_serialized_field() {
        let mut game = Game::new(42, 1);
        game.terrain = terrain::Terrain::from_heights(&(240..=420).map(|x| (x, 120)).collect());
        game.workings.initialise();
        game.workings.section = Some(workings::Section {
            from: 0,
            to: [400, 100],
            lift: false,
            cells: vec![[400, 100]],
            support_work: 0,
        });
        game.last_saved = 100;
        game.tick(materials(), true);
        assert!(game.idle_travel_skip(400) > 0);
        let mut stepped = game.clone();
        game.advance_offline(140, materials());
        for _ in 0..400 {
            stepped.tick(materials(), true);
        }
        stepped.last_saved = game.last_saved;
        stepped.offline = game.offline.clone();
        assert_eq!(
            serde_json::to_value(&game).unwrap(),
            serde_json::to_value(&stepped).unwrap()
        );
    }
}

#[cfg(test)]
mod tactics_unlock_tests {
    use super::*;
    #[test]
    fn changed_policy_reopens_planning_but_repeated_policy_preserves_search() {
        let mut g = Game::new(42, 2);
        g.workings.blocked_at = Some((10, 20, 1200));
        let revision = g.workings.revision;
        g.action(Action {
            sequence: 1,
            kind: "policy".into(),
            target: "depth".into(),
            value: 0,
        })
        .unwrap();
        assert!(g.workings.blocked_at.is_none());
        assert_eq!(g.workings.revision, revision + 1);
        g.workings.blocked_at = Some((11, 21, 1200));
        g.action(Action {
            sequence: 2,
            kind: "policy".into(),
            target: "depth".into(),
            value: 0,
        })
        .unwrap();
        assert_eq!(g.workings.blocked_at, Some((11, 21, 1200)));
        assert_eq!(g.workings.revision, revision + 1);
    }
    #[test]
    fn policies_are_available_from_start_and_removed_abilities_are_rejected() {
        let mut g = Game::default();
        g.action(Action {
            sequence: 1,
            kind: "policy".into(),
            target: "depth".into(),
            value: 0,
        })
        .unwrap();
        assert_eq!(g.policy, "depth");
        assert!(g
            .action(Action {
                sequence: 2,
                kind: "ability".into(),
                target: String::new(),
                value: 0
            })
            .is_err());
        let save = serde_json::to_value(&g).unwrap();
        assert!(save.get("boosts").is_none());
        assert!(save.get("cooldowns").is_none());
    }
}
