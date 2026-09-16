pub mod content;
pub mod logistics;
pub mod terrain;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
pub const WIDTH: u32 = 64;
pub const VERSION: u32 = 2;
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
        serde_json::from_str(include_str!("../../../content/recipes.json")).expect("valid recipes")
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
    pub requires: String,
    pub inputs: BTreeMap<String, u64>,
}
pub fn requirements() -> &'static [UpgradeRequirement] {
    static DATA: std::sync::OnceLock<Vec<UpgradeRequirement>> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../../../content/upgrades.json"))
            .expect("valid upgrade requirements")
    })
}
pub fn materials() -> Vec<Material> {
    serde_json::from_str(include_str!("../../../content/materials.json"))
        .expect("valid material catalogue")
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Cell {
    pub x: u32,
    pub y: u32,
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
#[derive(Clone, Serialize, Deserialize)]
pub struct Game {
    pub version: u32,
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
    pub heights: Vec<u32>,
    #[serde(default)]
    pub terrain: terrain::Terrain,
    pub removed: Vec<Cell>,
    pub ore: BTreeMap<usize, u64>,
    pub hauled: BTreeMap<usize, u64>,
    #[serde(default)]
    pub concentrate: BTreeMap<usize, u64>,
    #[serde(default)]
    pub flow_window: [u64; 5],
    #[serde(default)]
    pub trace_feed: BTreeMap<String, u64>,
    #[serde(default)]
    pub trace_fraction: BTreeMap<String, u64>,
    #[serde(default)]
    pub shipments: Vec<logistics::Shipment>,
    #[serde(skip)]
    haul_path: Vec<[u32; 2]>,
    #[serde(default)]
    pub crew: logistics::Crew,
    #[serde(default)]
    pub crew_priority: String,
    #[serde(default)]
    pub support_rows: u32,
    #[serde(default)]
    pub support_work: u64,
    pub products: BTreeMap<String, u64>,
    pub tailings: BTreeMap<usize, u64>,
    pub slag: u64,
    pub depleted: u64,
    #[serde(default)]
    pub disposed_mass: u64,
    pub lifetime_waste: u64,
    pub excavated: u64,
    pub discoveries: BTreeSet<usize>,
    pub site_discoveries: u32,
    pub contracts: Vec<Contract>,
    #[serde(default)]
    pub site_objectives: BTreeSet<String>,
    #[serde(skip)]
    pub retirement_quote: Option<u64>,
    pub records: Vec<Record>,
    pub cooldowns: [u32; 4],
    pub boosts: [u32; 3],
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
            seed,
            site,
            profile: 0,
            challenge: String::new(),
            ticks: 0,
            credits: 30,
            workers: 3,
            housing: 8,
            levels: BTreeMap::new(),
            research: 0,
            ranks: BTreeMap::new(),
            policy: "bulk".into(),
            specialisation: None,
            priorities: vec![],
            reserve: BTreeMap::new(),
            pinned: None,
            heights: vec![0; 64],
            terrain: terrain::Terrain::default(),
            removed: vec![],
            ore: BTreeMap::new(),
            hauled: BTreeMap::new(),
            concentrate: BTreeMap::new(),
            flow_window: [0; 5],
            trace_feed: BTreeMap::new(),
            trace_fraction: BTreeMap::new(),
            shipments: vec![],
            haul_path: vec![],
            crew: logistics::Crew::assign(3, &BTreeMap::new()),
            crew_priority: String::new(),
            support_rows: 150,
            support_work: 0,
            products: BTreeMap::new(),
            tailings: BTreeMap::new(),
            slag: 0,
            depleted: 0,
            disposed_mass: 0,
            lifetime_waste: 0,
            excavated: 0,
            discoveries: BTreeSet::new(),
            site_discoveries: 0,
            site_objectives: BTreeSet::new(),
            retirement_quote: None,
            contracts: vec![
                Contract {
                    product: "iron".into(),
                    amount: 2000,
                    complete: false,
                },
                Contract {
                    product: "copper".into(),
                    amount: 2000,
                    complete: false,
                },
                Contract {
                    product: "steel".into(),
                    amount: 1000,
                    complete: false,
                },
            ],
            records: vec![],
            cooldowns: [0; 4],
            boosts: [0; 3],
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
        self.heights.iter().max().copied().unwrap_or(0) * 2
    }
    pub fn cost(&self, k: &str) -> u64 {
        let (base, growth, n): (f64, f64, u32) = if k == "worker" {
            (25., 1.15, self.workers - 3)
        } else if k == "housing" {
            (90., 1.12, (self.housing - 8) / 4)
        } else {
            (
                match k {
                    "conveyor" => 450.,
                    "furnace" => 800.,
                    "shaft" => 600.,
                    "steelworks" => 450.,
                    "power" => 900.,
                    "chemical" => 1500.,
                    "electrolytic" => 2200.,
                    "trace" => 3500.,
                    _ => 100.,
                },
                if k == "capacity" { 1.12 } else { 1.18 },
                self.level(k),
            )
        };
        (base * growth.powi(n as i32)).ceil() as u64
    }
    pub fn purchase_blocker(&self, id: &str) -> Option<String> {
        let Some(u) = requirements().iter().find(|u| u.id == id) else {
            return Some("Unknown upgrade".into());
        };
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
            return Some(format!("Needs {} {}", *n as f64 / 1000., p));
        }
        if self.credits < self.cost(id) {
            return Some("More credits required".into());
        }
        None
    }
    pub fn award(&self) -> u64 {
        (10. * (self.depth() as f64 / 300.).sqrt()).floor() as u64
            + 3 * self.site_discoveries as u64
            + 5 * self.site_objectives.len() as u64
    }
    pub fn cell(&self, x: u32, y: u32, cat: &[Material]) -> usize {
        if (30..=34).contains(&x) && y % 24 < 3 {
            return [3, 5, 6][((y / 24) % 3) as usize];
        }
        let h = (self
            .seed
            .wrapping_add((x / 4) as u64 * 374761393)
            .wrapping_add((y / 3) as u64 * 668265263))
        .wrapping_mul(1274126177);
        if h % 100 < 55 {
            return if y < 4 { 0 } else { 1 };
        }
        let tier = match y * 2 {
            0..=99 => 0,
            100..=299 => 1,
            300..=699 => 2,
            700..=1499 => 3,
            1500..=2999 => 4,
            _ => 5,
        };
        let mut pool: Vec<_> = cat.iter().filter(|m| m.tier <= tier).collect();
        let profile = &sites()[self.profile];
        pool.extend(
            cat.iter()
                .filter(|m| m.tier <= tier && profile.focus.contains(&m.id)),
        );
        pool[((h >> 8) as usize) % pool.len()].id
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
    fn dig_rate(&self, offline: bool) -> u64 {
        (((self.crew.diggers as f64 * self.throughput("drill")
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
            * (1. + 0.05 * self.ranks.get("excavation").copied().unwrap_or(0) as f64))
            * (if !offline && self.boosts[0] > 0 {
                1.5
            } else {
                1.
            }))
            * 1000.) as u64
    }
    fn next_frontier(&self, cat: &[Material]) -> Option<u32> {
        let depth_limit = if self.level("shaft") == 0 {
            100
        } else {
            300 * (1 + self.level("shaft"))
        };
        self.terrain
            .frontier
            .iter()
            .filter_map(|&key| {
                let (x, y) = (key % WIDTH, key / WIDTH);
                if y * 2 >= depth_limit {
                    return None;
                }
                // Below the open pit, create a main shaft with branches every 12 rows.
                if self.policy != "bulk" && y >= 24 && x.abs_diff(32) > 1 && y % 12 > 1 {
                    return None;
                }
                if y >= 150 && (self.level("supports") == 0 || y >= self.support_rows) {
                    return None;
                }
                if y >= 350 && self.level("pump") == 0 {
                    return None;
                }
                if y >= 750 && self.level("ventilation") == 0 {
                    return None;
                }
                let score = match self.policy.as_str() {
                    "depth" => -(y as i64) * 8 + x.abs_diff(32) as i64,
                    "vein" => {
                        y as i64
                            - if self.priorities.contains(&self.cell(x, y, cat)) {
                                40
                            } else {
                                0
                            }
                    }
                    _ => y as i64,
                };
                Some((score, key))
            })
            .min()
            .map(|(_, key)| key)
    }
    /// No input, pending arrival, eligible recipe or reachable excavation event can fire.
    fn quiescent(&self, cat: &[Material]) -> bool {
        if (self.level("supports") > 0 && self.support_rows < self.support_target())
            || self.next_frontier(cat).is_some()
            || self
                .ore
                .values()
                .chain(self.hauled.values())
                .chain(self.concentrate.values())
                .any(|q| *q > 0)
            || !self.shipments.is_empty()
            || self.depleted > 0
            || (self.level("slagcrusher") > 0 && self.slag > 0)
            || (self.level("reclaimer") > 0 && self.tailings.values().any(|q| *q > 0))
        {
            return false;
        }
        if self.level("survey") > 0
            && self
                .terrain
                .frontier
                .iter()
                .rev()
                .take(self.crew.prospectors as usize)
                .any(|key| {
                    !self
                        .discoveries
                        .contains(&self.cell(key % WIDTH, key / WIDTH, cat))
                })
        {
            return false;
        }
        !recipes().iter().any(|r| {
            if self.level(&r.building) == 0
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
            r.inputs
                .iter()
                .all(|(p, n)| self.products.get(p).copied().unwrap_or(0) >= *n)
        })
    }
    fn support_target(&self) -> u32 {
        (self.heights.iter().copied().max().unwrap_or(0).max(150) + 1).min(terrain::MAX_ROWS)
    }
    fn construction(&mut self) {
        let target = self.support_target();
        if self.level("supports") > 0 && self.support_rows < target {
            self.support_work +=
                (10. * self.crew.engineers.max(1) as f64 * self.throughput("supports")) as u64;
            self.support_rows = (self.support_rows + (self.support_work / 1000) as u32).min(target);
            self.support_work %= 1000;
        }
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
        }
        let sold_before = self.sold_mass;
        if !offline && self.ticks % 20 == 0 {
            for c in &mut self.cooldowns {
                *c = c.saturating_sub(1)
            }
            for b in &mut self.boosts {
                *b = b.saturating_sub(1)
            }
        }
        self.crew = logistics::Crew::prioritise(self.workers, &self.levels, &self.crew_priority);
        self.construction();
        if self.ticks % 1200 == 0 && self.level("survey") > 0 {
            let candidates: Vec<_> = self
                .terrain
                .frontier
                .iter()
                .rev()
                .take(self.crew.prospectors as usize)
                .map(|key| self.cell(key % WIDTH, key / WIDTH, cat))
                .collect();
            for id in candidates {
                if self.discoveries.insert(id) {
                    self.site_discoveries += 1;
                }
            }
        }
        let cap = 20000 + 5000 * self.level("capacity") as u64;
        let ore_total: u64 = self.ore.values().sum();
        let mut mined = 0;
        let dig_rate = self.dig_rate(offline);
        let work = dig_rate + self.dig_remainder;
        self.dig_progress += work / 20;
        self.dig_remainder = work % 20;
        let digs = (self.dig_progress / 1000) as u32;
        self.dig_progress %= 1000;
        let depth_limit = if self.level("shaft") == 0 {
            100
        } else {
            300 * (1 + self.level("shaft"))
        };
        for _ in 0..digs.min(200) {
            if ore_total + (mined + 1) * 1000 > cap {
                break;
            }
            let target = self.next_frontier(cat);
            let Some(key) = target else { break };
            let (x, y) = (key % WIDTH, key / WIDTH);
            let id = self.cell(x, y, cat);
            if !self.terrain.excavate(x, y) {
                break;
            }
            self.heights[x as usize] = self.heights[x as usize].max(y + 1);
            self.removed.push(Cell { x, y, material: id });
            if self.removed.len() > 512 {
                self.removed.drain(..256);
            }
            *self.ore.entry(id).or_default() += 1000;
            self.excavated += 1;
            mined += 1;
            if self.discoveries.insert(id) {
                self.site_discoveries += 1;
            }
        }
        if self.ticks % 20 == 0 {
            logistics::arrive_at_speed(
                &mut self.shipments,
                &mut self.hauled,
                cap,
                if !offline && self.boosts[1] > 0 { 2 } else { 1 },
            );
        }
        let (transport, _) = logistics::mode(&self.levels);
        let route_depth = self.depth();
        if let Some(cell) = self.removed.last() {
            let origin = [cell.x, cell.y];
            if self.haul_path.first() != Some(&origin) {
                self.haul_path = self.terrain.surface_route(origin);
            }
        }
        let terrain_factor = if self.challenge == "long_haul" {
            1.5
        } else {
            1.
        } / sites()[self.profile].haul;
        let (legs, duration) = logistics::route(&self.haul_path, &self.levels, terrain_factor);
        let haul_rate = (1000.
            * self.crew.haulers as f64
            * if !offline && self.boosts[0] > 0 && matches!(transport, "carrying" | "wheelbarrow") {
                1.5
            } else {
                1.
            }
            * self.throughput("conveyor")
            * (1. + 0.05 * self.ranks.get("logistics").copied().unwrap_or(0) as f64)
            * (if self.level("conveyor") > 0 { 3. } else { 1. })
            * (if !offline && self.boosts[1] > 0 {
                2.
            } else {
                1.
            })) as u64;
        let haul_rate = haul_rate / 20;
        let mut remaining = haul_rate.min(cap.saturating_sub(
            self.hauled.values().sum::<u64>()
                + self.shipments.iter().map(|s| s.amount).sum::<u64>(),
        ));
        let haul_budget = remaining;
        let mut ore_ids: Vec<_> = self
            .ore
            .iter()
            .filter(|(_, q)| **q > 0)
            .map(|(&id, _)| id)
            .collect();
        if !ore_ids.is_empty() {
            let offset = self.ticks as usize % ore_ids.len();
            ore_ids.rotate_left(offset);
        }
        for id in ore_ids {
            let q = self.ore.get_mut(&id).expect("known ore");
            let n = (*q).min(remaining);
            *q -= n;
            if n > 0 {
                self.shipments.push(logistics::Shipment {
                    material: id,
                    amount: n,
                    remaining: duration,
                    duration,
                    depth: route_depth,
                    mode: transport.into(),
                    path: self.haul_path.clone(),
                    legs: legs.clone(),
                });
            }
            remaining -= n;
        }
        let sort_rate = (1000. * self.throughput("sorter")) as u64 / 20;
        let mut sort_left = sort_rate.min(cap.saturating_sub(self.concentrate.values().sum()));
        let sort_budget = sort_left;
        let mut sort_ids: Vec<_> = self
            .hauled
            .iter()
            .filter(|(_, q)| **q > 0)
            .map(|(&id, _)| id)
            .collect();
        if !sort_ids.is_empty() {
            let offset = self.ticks as usize % sort_ids.len();
            sort_ids.rotate_left(offset);
        }
        for id in sort_ids {
            let quantity = self.hauled.get_mut(&id).expect("known feed");
            let n = (*quantity).min(sort_left);
            *quantity -= n;
            *self.concentrate.entry(id).or_default() += n;
            sort_left -= n;
        }
        let sorted = sort_budget - sort_left;
        let power_demand =
            1 + self.level("chemical") + 2 * self.level("electrolytic") + 3 * self.level("trace");
        let power_demand = power_demand as f64
            * if !offline && self.boosts[2] > 0 {
                1.5
            } else {
                1.
            };
        let power_supply = 1 + 5 * self.level("power");
        let power_factor = (power_supply as f64 / power_demand as f64).min(1.);
        let process_rate = (power_factor
            * if self.specialisation.as_deref() == Some("reclamation") {
                0.85
            } else {
                1.
            }
            * 1000.
            * self.throughput("furnace")
            * (1. + 0.15 * self.crew.operators.saturating_sub(1) as f64)
            * (1. + 0.05 * self.ranks.get("metallurgy").copied().unwrap_or(0) as f64)
            * (if !offline && self.boosts[2] > 0 {
                1.5
            } else {
                1.
            })) as u64;
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
                "sulfide" | "chemical" => self.levels.get("chemical").copied().unwrap_or(0) > 0,
                "electrolytic" => self.levels.get("chemical").copied().unwrap_or(0) > 0,
                _ => self.levels.get("trace").copied().unwrap_or(0) > 0,
            };
            let n = (*q).min(left);
            if n == 0 {
                continue;
            }
            *q -= n;
            left -= n;
            if !unlocked {
                self.credit_fraction += n * m.price;
                self.credits += self.credit_fraction / 2000;
                self.credit_fraction %= 2000;
                self.sold_mass += n;
                continue;
            }
            let recovery = (65i32
                + 3 * self.levels.get("recovery").copied().unwrap_or(0) as i32
                + match self.specialisation.as_deref() {
                    Some("bulk") => -5,
                    Some("precision") => 10,
                    _ => 0,
                })
            .clamp(0, 95) as u64;
            let good = n * recovery / 100;
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
            *self
                .products
                .entry(if m.name == "Bauxite" {
                    "alumina".into()
                } else {
                    m.product.clone()
                })
                .or_default() += primary;
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
        for recipe in recipes {
            let automatic = recipe.id == "steel" || recipe.id == "aluminium";
            if self.level(&recipe.building) == 0
                || (!automatic && !self.enabled_recipes.contains(&recipe.id))
            {
                continue;
            }
            if recipe.id.starts_with("separate_") {
                let source = format!("{}_residue", recipe.output);
                let available = self.trace_feed.entry(source).or_default();
                let amount = (*available).min(12);
                *available -= amount;
                *self.products.entry(recipe.output.clone()).or_default() += amount;
                continue;
            }
            let amount = recipe
                .inputs
                .iter()
                .map(|(p, n)| self.products.get(p).copied().unwrap_or(0) / n)
                .min()
                .unwrap_or(0)
                .min(if self.ticks % 2 == 0 { 13 } else { 12 });
            if amount > 0 {
                for (p, n) in &recipe.inputs {
                    *self.products.entry(p.clone()).or_default() -= amount * n;
                }
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
                let n = (n * multiplier).min(*q).min(recovery_space);
                recovery_space -= n;
                *q -= n;
                *self.hauled.entry(id).or_default() += n;
            }
        }
        for (p, q) in &mut self.products {
            let recipe_hold = recipes.iter().any(|r| {
                self.levels.get(&r.building).copied().unwrap_or(0) > 0
                    && (r.id == "steel"
                        || r.id == "aluminium"
                        || self.enabled_recipes.contains(&r.id))
                    && r.inputs.contains_key(p)
            });
            let reserved = self
                .reserve
                .get(p)
                .copied()
                .unwrap_or(0)
                .max(if recipe_hold { 8000 } else { 0 })
                .max(
                    self.pinned
                        .as_ref()
                        .and_then(|id| requirements().iter().find(|u| u.id == *id))
                        .and_then(|u| u.inputs.get(p))
                        .copied()
                        .unwrap_or(0),
                );
            let contract_hold = self
                .contracts
                .iter()
                .filter(|c| !c.complete && c.product == *p)
                .map(|c| c.amount)
                .sum::<u64>();
            let sold = q.saturating_sub(reserved.max(contract_hold));
            *q -= sold;
            self.sold_mass += sold;
            let price = cat
                .iter()
                .find(|m| m.product == *p)
                .map(|m| m.price)
                .unwrap_or(20);
            self.credit_fraction += sold * price * 2;
            self.credits = self.credits.saturating_add(self.credit_fraction / 2000);
            self.credit_fraction %= 2000;
        }
        let disposed = self.depleted.min(5);
        self.depleted -= disposed;
        self.disposed_mass += disposed;
        if self.level("slagcrusher") > 0 {
            let reclaimed =
                self.slag
                    .min(if self.specialisation.as_deref() == Some("reclamation") {
                        75
                    } else {
                        25
                    });
            self.slag -= reclaimed;
            *self.products.entry("aggregate".into()).or_default() += reclaimed;
        }
        for (i, n) in [
            mined * 1000,
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
        self.stages = vec![
            Stage {
                name: "Digging".into(),
                rate: self.flow_window[0] as f64 / 1000. / seconds,
                buffer: self.ore.values().sum(),
                capacity: cap,
                blocker: if self.depth() >= depth_limit {
                    "Shaft upgrade required"
                } else if self.depth() >= 1500 && self.level("ventilation") == 0 {
                    "Ventilation required"
                } else if self.depth() >= 700 && self.level("pump") == 0 {
                    "Drainage required"
                } else if self.depth() >= 300 && self.level("supports") == 0 {
                    "Supports required"
                } else if self.level("supports") > 0
                    && self.support_rows < self.support_target()
                    && self.depth() >= 300
                {
                    "Building supports"
                } else if ore_total + 1000 > cap {
                    "Hauling buffer full"
                } else {
                    "Working"
                }
                .into(),
            },
            Stage {
                name: "Hauling".into(),
                rate: self.flow_window[1] as f64 / 1000. / seconds,
                buffer: self.hauled.values().sum::<u64>()
                    + self.shipments.iter().map(|s| s.amount).sum::<u64>(),
                capacity: cap,
                blocker: if haul_budget == 0 {
                    "Cargo buffers full"
                } else if remaining == haul_budget {
                    "Waiting for ore"
                } else {
                    "Working"
                }
                .into(),
            },
            Stage {
                name: "Sorting".into(),
                rate: self.flow_window[2] as f64 / 1000. / seconds,
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
                rate: self.flow_window[3] as f64 / 1000. / seconds,
                buffer: self.products.values().sum(),
                capacity: cap,
                blocker: if power_factor < 1. {
                    "Power supply limited"
                } else if self.level("furnace") == 0 {
                    "Raw sales · furnace locked"
                } else {
                    "Reserves protected"
                }
                .into(),
            },
            Stage {
                name: "Dispatch".into(),
                rate: self.flow_window[4] as f64 / 1000. / seconds,
                buffer: 0,
                capacity: cap,
                blocker: "Selling surplus".into(),
            },
        ];
    }
    pub fn advance_offline(&mut self, now: u64, cat: &[Material]) {
        let elapsed = if self.last_saved == 0 {
            0
        } else {
            now.saturating_sub(self.last_saved)
        };
        let effective = elapsed.min(28800) / 2;
        let old = self.credits;
        let mined = self.excavated;
        let known = self.discoveries.clone();
        for second in 0..effective {
            self.second(cat, true);
            let left = effective - second - 1;
            if left > 0 && self.quiescent(cat) {
                // Preserve fractional work and phase; the final tick refreshes feedback.
                let skip = left * 20 - 1;
                let work = self.dig_rate(true) * skip + self.dig_remainder;
                self.dig_progress = (self.dig_progress + work / 20) % 1000;
                self.dig_remainder = work % 20;
                self.ticks += skip;
                self.flow_window = [0; 5];
                self.tick(cat, true);
                break;
            }
        }
        self.offline = Some(Offline {
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
        self.last_saved = now;
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
                if self.depth() < 100 {
                    return Err("Reach 100 metres to specialise".into());
                }
                if self.specialisation.is_some() {
                    return Err("Specialisation lasts until site retirement".into());
                }
                if !["bulk", "precision", "reclamation"].contains(&a.target.as_str()) {
                    return Err("Unknown specialisation".into());
                }
                self.specialisation = Some(a.target.clone());
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
                if !["bulk", "vein", "depth"].contains(&a.target.as_str()) {
                    return Err("Invalid policy".into());
                }
                self.policy = a.target;
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
                if !self.enabled_recipes.remove(&a.target) {
                    self.enabled_recipes.insert(a.target);
                }
            }
            "megaproject" => {
                if self.megaproject {
                    return Err("Project already complete".into());
                }
                let needs = [
                    ("advanced_structure", 10000),
                    ("precision_controls", 10000),
                    ("magnets", 10000),
                    ("batteries", 10000),
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
            "ability" => {
                let i = a.value as usize;
                if i > 3 {
                    return Err("Invalid ability".into());
                }
                if self.cooldowns[i] > 0 {
                    return Err("Ability cooling down".into());
                }
                self.cooldowns[i] = if i == 3 { 300 } else { 180 };
                if i < 3 {
                    self.boosts[i] = if i == 1 { 20 } else { 30 }
                } else {
                    self.policy = "vein".into();
                    let cat = materials();
                    let id = self.cell(32, self.heights[32], &cat);
                    if self.discoveries.insert(id) {
                        self.site_discoveries += 1;
                    }
                    self.priorities = vec![id]
                }
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
                self.credits += self.credit_fraction / 2000;
                self.credit_fraction %= 2000;
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
                    amount: old.amount.saturating_add(1000).min(20000),
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
                let cost = 5 * (rank as u64 + 1).pow(2);
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
                next.challenge = a.target;
                next.profile = a.value as usize;
                next.milestones = self.milestones.clone();
                next.blueprint = self.blueprint.clone();
                next.build_queue = next.blueprint.clone();
                next.research = self.research + award;
                next.discoveries = self.discoveries.clone();
                next.ranks = self.ranks.clone();
                next.records = self.records.clone();
                let mut section = vec![0; 4096];
                let rows = (self.depth() / 2).max(1);
                for y in 0..64 {
                    for x in 0..64 {
                        section[(y * 64 + x) as usize] =
                            u8::from(self.terrain.contains(x, y * rows / 64));
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
                next.policy = self.policy.clone();
                next.crew_priority = self.crew_priority.clone();
                next.priorities = self.priorities.clone();
                if next.ranks.values().any(|r| *r >= 3) {
                    next.levels.insert("conveyor".into(), 1);
                }
                next.apply_headquarters();
                *self = next;
            }
            _ => return Err("Unknown command".into()),
        }
        self.last_sequence = a.sequence;
        Ok(())
    }
    pub fn apply_headquarters(&mut self) {
        for (branch, buildings) in [
            ("excavation", ["drill", "supports", "shaft"]),
            ("logistics", ["conveyor", "minecart", "train"]),
            ("metallurgy", ["furnace", "chemical", "electrolytic"]),
            ("prospecting", ["survey", "pump", "trace"]),
            ("reclamation", ["recovery", "reclaimer", "slagcrusher"]),
        ] {
            let rank = self.ranks.get(branch).copied().unwrap_or(0);
            for (required, building) in [3, 6, 10].into_iter().zip(buildings) {
                if rank >= required {
                    self.levels.entry(building.into()).or_insert(1);
                }
            }
        }
    }
    pub fn migrate(&mut self) -> Result<(), String> {
        if self.version == 1 {
            if self.profile >= sites().len()
                || self.heights.len() != 64
                || self.heights.iter().any(|h| *h > 100000)
            {
                return Err("Invalid legacy terrain".into());
            }
            self.terrain = terrain::Terrain::from_columns(&self.heights);
            self.version = VERSION;
        }
        for (index, contract) in self.contracts.iter().enumerate() {
            if contract.complete {
                self.site_objectives.insert(format!("order-{index}"));
            }
        }
        if self.support_rows == 0 {
            self.support_rows = self.support_target();
        }
        self.terrain.rebuild()?;
        self.validate()
    }
    pub fn validate(&self) -> Result<(), String> {
        if self
            .specialisation
            .as_deref()
            .is_some_and(|s| !["bulk", "precision", "reclamation"].contains(&s))
        {
            return Err("Invalid specialisation".into());
        }
        if self.version != VERSION {
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
            || !["", "hard_rock", "long_haul"].contains(&self.challenge.as_str())
            || self.profile >= sites().len()
            || self.heights.len() != 64
            || self.heights.iter().any(|h| *h > 100000)
            || self.support_rows > terrain::MAX_ROWS
            || self.support_work >= 1000
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
            .any(|(id, bytes)| *id >= 1563 || bytes.len() != 512)
        {
            return Err("Invalid terrain chunk".into());
        }
        let n = materials().len();
        if self.shipments.len() > 20000
            || self.shipments.iter().any(|s| {
                s.duration == 0
                    || s.legs.len() > 100002
                    || s.legs.iter().any(|l| {
                        l.milliseconds == 0
                            || [l.from, l.to].iter().any(|p| {
                                p[0] < 0
                                    || p[0] > 72
                                    || p[1] < -3
                                    || p[1] >= terrain::MAX_ROWS as i32
                            })
                    })
                    || s.path.len() > 100000
                    || s.path
                        .iter()
                        .any(|p| p[0] >= 64 || p[1] >= terrain::MAX_ROWS)
                    || s.material >= n
                    || s.amount > 1_000_000_000_000
                    || s.duration > 100000
                    || s.remaining > s.duration
            })
        {
            return Err("Invalid cargo shipment".into());
        }
        if self
            .products
            .values()
            .chain(self.ore.values())
            .chain(self.hauled.values())
            .chain(self.concentrate.values())
            .chain(self.trace_feed.values())
            .chain(self.tailings.values())
            .chain(self.reserve.values())
            .any(|v| *v > 1_000_000_000_000)
        {
            return Err("Save inventory exceeds supported limits".into());
        }
        if self.removed.iter().any(|c| c.x >= 64 || c.material >= n)
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
            + g.hauled.values().sum::<u64>()
            + g.concentrate.values().sum::<u64>()
            + g.trace_feed.values().sum::<u64>()
            + g.products.values().sum::<u64>()
            + g.tailings.values().sum::<u64>()
            + g.sold_mass
            + g.shipments.iter().map(|s| s.amount).sum::<u64>()
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
        g.reserve.insert("alumina".into(), 10000);
        g.hauled.insert(id, 1000);
        for _ in 0..10 {
            g.second(&cat, false);
        }
        assert!(g.products.get("alumina").copied().unwrap_or(0) > 0);
        assert_eq!(g.products.get("aluminium").copied().unwrap_or(0), 0);
        g.levels.insert("electrolytic".into(), 1);
        g.reserve.insert("aluminium".into(), 10000);
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
        g.concentrate.insert(2, 20000);
        g.hauled.insert(2, 1000);
        g.tick(&materials(), false);
        assert_eq!(g.flow_window[2], 0);
        assert_eq!(g.hauled[&2], 1000);
        assert!(g.concentrate[&2] < 20000);
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
            assert_eq!(g.cell(32, 0, &cat), 3);
            assert_eq!(g.cell(32, 24, &cat), 5);
            assert_eq!(g.cell(32, 48, &cat), 6);
        }
    }
    #[test]
    fn site_profiles_change_deposits() {
        let cat = materials();
        let a = Game::default();
        let mut b = a.clone();
        b.profile = 1;
        assert!((0..64).any(|x| a.cell(x, 100, &cat) != b.cell(x, 100, &cat)));
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
        g.products.insert("alumina".into(), 10000);
        g.reserve.insert("gallium".into(), 10000);
        g.tick(&materials(), false);
        assert_eq!(g.products.get("gallium").copied().unwrap_or(0), 0);
        g.trace_feed.insert("gallium_residue".into(), 100);
        g.tick(&materials(), false);
        assert_eq!(g.products["gallium"], 12);
        assert_eq!(g.trace_feed["gallium_residue"], 88);
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
        assert_eq!(g.depleted, 95);
        assert_eq!(g.disposed_mass, 5);
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
        g.products.insert("iron".into(), 2000);
        g.action(a).unwrap();
        assert_eq!(g.products["iron"], 0);
        assert_eq!(g.delivered_mass, 2000);
    }
    #[test]
    fn pin_holds_upgrade_material() {
        let mut g = Game::default();
        g.pinned = Some("shaft".into());
        g.products.insert("iron".into(), 3000);
        g.contracts.clear();
        g.tick(&materials(), false);
        assert_eq!(g.products["iron"], 2000);
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
        assert_eq!(g.level("conveyor"), 1);
        assert_eq!(g.level("minecart"), 1);
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
        g.heights[0] = 150;
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
        g.products.insert("iron".into(), 10000);
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
        assert_eq!(g.contracts[0].amount, 3000);
        g.action(Action {
            sequence: 3,
            kind: "contract".into(),
            target: String::new(),
            value: 0,
        })
        .unwrap();
        assert_eq!(g.research, 1);
        assert_eq!(g.site_objectives.len(), 1);
        assert_eq!(g.delivered_mass, 5000);
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
        g.heights[32] = 50;
        g.action(choose(2)).unwrap();
        assert!(g.action(choose(3)).is_err());
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
        g.hauled.insert(0, 20000);
        g.ore.insert(0, 1000);
        g.tick(&materials(), false);
        assert_eq!(g.flow_window[1], 0);
        assert!(g.shipments.is_empty());
        assert_eq!(g.stages[1].blocker, "Cargo buffers full");
    }
}

#[cfg(test)]
mod challenge_tests {
    use super::*;
    #[test]
    fn retirement_matches_preview_and_resets_site_choices() {
        let mut g = Game::default();
        g.heights[32] = 150;
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
        a.heights = vec![50; 64];
        a.terrain = terrain::Terrain::from_columns(&a.heights);
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
        g.heights = vec![50; 64];
        g.terrain = terrain::Terrain::from_columns(&g.heights);
        g.shipments.push(logistics::Shipment {
            material: 0,
            amount: 1000,
            remaining: 30,
            duration: 30,
            depth: 100,
            mode: "carrying".into(),
            path: vec![],
            legs: vec![],
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
        g.heights = vec![3000; 64];
        let rate = g.dig_rate(true);
        assert!(rate > 0 && rate < 20);
        g.second(&materials(), true);
        assert_eq!(g.dig_progress, rate);
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
    fn engineers_build_supports_before_deeper_fronts_open() {
        let cat = materials();
        let mut g = Game::default();
        g.levels.insert("supports".into(), 1);
        g.levels.insert("shaft".into(), 1);
        g.workers = 10;
        g.housing = 12;
        g.crew_priority = "engineering".into();
        g.policy = "depth".into();
        for y in 0..150 {
            g.terrain.excavate(32, y);
        }
        g.heights[32] = 150;
        assert_ne!(g.next_frontier(&cat), Some(150 * 64 + 32));
        for _ in 0..30 {
            g.tick(&cat, true);
        }
        assert!(g.support_rows > 150);
        assert!(g.crew.engineers > 1);
    }
}

#[cfg(test)]
mod retirement_quote_tests {
    use super::*;
    #[test]
    fn production_between_preview_and_confirmation_does_not_change_quote() {
        let mut g = Game::default();
        g.heights[32] = 150;
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
