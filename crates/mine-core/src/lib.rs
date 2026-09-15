use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
pub const WIDTH: u32 = 64;
pub const VERSION: u32 = 1;
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
pub fn recipes() -> Vec<Recipe> {
    serde_json::from_str(include_str!("../../../content/recipes.json")).expect("valid recipes")
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
    pub site: u32,
    pub depth: u32,
    pub research: u64,
    pub excavated: u64,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Game {
    pub version: u32,
    pub seed: u64,
    pub site: u32,
    pub ticks: u64,
    #[serde(with = "decimal")]
    pub credits: u64,
    pub workers: u32,
    pub housing: u32,
    pub levels: BTreeMap<String, u32>,
    pub research: u64,
    pub ranks: BTreeMap<String, u32>,
    pub policy: String,
    pub priorities: Vec<usize>,
    pub reserve: BTreeMap<String, u64>,
    pub pinned: Option<String>,
    pub heights: Vec<u32>,
    pub removed: Vec<Cell>,
    pub ore: BTreeMap<usize, u64>,
    pub hauled: BTreeMap<usize, u64>,
    pub products: BTreeMap<String, u64>,
    pub tailings: BTreeMap<usize, u64>,
    pub slag: u64,
    pub depleted: u64,
    pub lifetime_waste: u64,
    pub excavated: u64,
    pub discoveries: BTreeSet<usize>,
    pub site_discoveries: u32,
    pub contracts: Vec<Contract>,
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
    pub enabled_recipes: BTreeSet<String>,
    #[serde(default)]
    pub sold_mass: u64,
    #[serde(default)]
    pub delivered_mass: u64,
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
        String::deserialize(d)?
            .parse()
            .map_err(serde::de::Error::custom)
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
            ticks: 0,
            credits: 30,
            workers: 3,
            housing: 8,
            levels: BTreeMap::new(),
            research: 0,
            ranks: BTreeMap::new(),
            policy: "bulk".into(),
            priorities: vec![],
            reserve: BTreeMap::new(),
            pinned: None,
            heights: vec![0; 64],
            removed: vec![],
            ore: BTreeMap::new(),
            hauled: BTreeMap::new(),
            products: BTreeMap::new(),
            tailings: BTreeMap::new(),
            slag: 0,
            depleted: 0,
            lifetime_waste: 0,
            excavated: 0,
            discoveries: BTreeSet::new(),
            site_discoveries: 0,
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
            enabled_recipes: BTreeSet::new(),
            sold_mass: 0,
            delivered_mass: 0,
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
                    "conveyor" => 75.,
                    "furnace" => 160.,
                    "shaft" => 600.,
                    "steelworks" => 450.,
                    "power" => 900.,
                    "chemical" => 1500.,
                    "electrolytic" => 2200.,
                    "trace" => 3500.,
                    _ => 100.,
                },
                1.18,
                self.level(k),
            )
        };
        (base * growth.powi(n as i32)).ceil() as u64
    }
    pub fn award(&self) -> u64 {
        (10. * (self.depth() as f64 / 300.).sqrt()).floor() as u64
            + 3 * self.site_discoveries as u64
            + 5 * self.contracts.iter().filter(|c| c.complete).count() as u64
    }
    pub fn cell(&self, x: u32, y: u32, cat: &[Material]) -> usize {
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
        let pool: Vec<_> = cat.iter().filter(|m| m.tier <= tier).collect();
        pool[((h >> 8) as usize) % pool.len()].id
    }
    fn throughput(&self, k: &str) -> f64 {
        let n = self.level(k);
        (1. + 0.12 * n as f64) * 1.5f64.powi((n / 10) as i32)
    }
    pub fn second(&mut self, cat: &[Material], offline: bool) {
        self.ticks += 20;
        if !offline {
            for c in &mut self.cooldowns {
                *c = c.saturating_sub(1)
            }
            for b in &mut self.boosts {
                *b = b.saturating_sub(1)
            }
        }
        let cap = 20000 + 5000 * self.level("capacity") as u64;
        let ore_total: u64 = self.ore.values().sum();
        let mut mined = 0;
        let digs = ((self.workers as f64
            * 0.65
            * self.throughput("drill")
            * (1. + 0.05 * self.ranks.get("excavation").copied().unwrap_or(0) as f64))
            * (if !offline && self.boosts[0] > 0 {
                1.5
            } else {
                1.
            }))
        .ceil() as u32;
        let depth_limit = if self.level("shaft") == 0 {
            100
        } else {
            300 * (1 + self.level("shaft"))
        };
        for _ in 0..digs.min(200) {
            if ore_total + mined * 1000 >= cap {
                break;
            }
            let mut cols: Vec<u32> = (0..WIDTH).collect();
            cols.sort_by_key(|&x| {
                let y = self.heights[x as usize];
                match self.policy.as_str() {
                    "depth" => y + x.abs_diff(32) * 10,
                    "vein" => {
                        y + if self.priorities.contains(&self.cell(x, y, cat)) {
                            0
                        } else {
                            8
                        }
                    }
                    _ => y,
                }
            });
            let x = cols[0];
            let y = self.heights[x as usize];
            if y * 2 >= depth_limit {
                break;
            }
            let id = self.cell(x, y, cat);
            self.heights[x as usize] += 1;
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
        let haul_rate = (1000.
            * self.throughput("conveyor")
            * (1. + 0.05 * self.ranks.get("logistics").copied().unwrap_or(0) as f64)
            * (if self.level("conveyor") > 0 { 3. } else { 1. })
            * (if !offline && self.boosts[1] > 0 {
                2.
            } else {
                1.
            })) as u64;
        let mut remaining = haul_rate.min(cap.saturating_sub(self.hauled.values().sum()));
        for (&id, q) in &mut self.ore {
            let n = (*q).min(remaining);
            *q -= n;
            *self.hauled.entry(id).or_default() += n;
            remaining -= n;
        }
        let power_demand =
            1 + self.level("chemical") + 2 * self.level("electrolytic") + 3 * self.level("trace");
        let power_supply = 1 + 5 * self.level("power");
        let power_factor = (power_supply as f64 / power_demand as f64).min(1.);
        let process_rate = (power_factor
            * 1000.
            * self.throughput("sorter")
            * (1. + 0.05 * self.ranks.get("metallurgy").copied().unwrap_or(0) as f64)
            * (if !offline && self.boosts[2] > 0 {
                1.5
            } else {
                1.
            })) as u64;
        let mut left = process_rate;
        let mut feed_ids: Vec<_> = self
            .hauled
            .iter()
            .filter(|(_, q)| **q > 0)
            .map(|(&id, _)| id)
            .collect();
        if !feed_ids.is_empty() {
            let offset = (self.ticks / 20) as usize % feed_ids.len();
            feed_ids.rotate_left(offset);
        }
        for id in feed_ids {
            let q = self.hauled.get_mut(&id).expect("known feed");
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
                self.credits += n * m.price / 2000;
                self.sold_mass += n;
                continue;
            }
            let recovery =
                (65 + 3 * self.levels.get("recovery").copied().unwrap_or(0)).min(95) as u64;
            let good = n * recovery / 100;
            let waste = n - good;
            *self
                .products
                .entry(if m.name == "Bauxite" {
                    "alumina".into()
                } else {
                    m.product.clone()
                })
                .or_default() += good;
            *self.tailings.entry(id).or_default() += waste;
            self.lifetime_waste += waste;
        }
        let recipes = recipes();
        for recipe in &recipes {
            let automatic = recipe.id == "steel" || recipe.id == "aluminium";
            if self.level(&recipe.building) == 0
                || (!automatic && !self.enabled_recipes.contains(&recipe.id))
            {
                continue;
            }
            let amount = recipe
                .inputs
                .iter()
                .map(|(p, n)| self.products.get(p).copied().unwrap_or(0) / n)
                .min()
                .unwrap_or(0)
                .min(250);
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
        if self.level("reclaimer") > 0 {
            for (&id, q) in &mut self.tailings {
                let n = (*q).min(20 + self.ranks.get("reclamation").copied().unwrap_or(0) as u64);
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
                .max(if recipe_hold { 8000 } else { 0 });
            let contract_hold = self
                .contracts
                .iter()
                .filter(|c| !c.complete && c.product == *p)
                .map(|c| c.amount)
                .max()
                .unwrap_or(0);
            let sold = q.saturating_sub(reserved.max(contract_hold));
            *q -= sold;
            self.sold_mass += sold;
            let price = cat
                .iter()
                .find(|m| m.product == *p)
                .map(|m| m.price)
                .unwrap_or(20);
            self.credits = self.credits.saturating_add(sold * price / 1000);
        }
        self.depleted = self.depleted.saturating_sub(100);
        self.stages = vec![
            Stage {
                name: "Digging".into(),
                rate: mined as f64,
                buffer: self.ore.values().sum(),
                capacity: cap,
                blocker: if self.depth() >= depth_limit {
                    "Shaft upgrade required"
                } else if mined == 0 {
                    "Hauling buffer full"
                } else {
                    "Working"
                }
                .into(),
            },
            Stage {
                name: "Hauling".into(),
                rate: (haul_rate - remaining) as f64 / 1000.,
                buffer: self.hauled.values().sum(),
                capacity: cap,
                blocker: if remaining == haul_rate {
                    "Waiting for ore"
                } else {
                    "Working"
                }
                .into(),
            },
            Stage {
                name: "Sorting".into(),
                rate: (process_rate - left) as f64 / 1000.,
                buffer: 0,
                capacity: cap,
                blocker: "Automatic separation".into(),
            },
            Stage {
                name: "Refining".into(),
                rate: (process_rate - left) as f64 / 1000.,
                buffer: self.products.values().sum(),
                capacity: cap,
                blocker: if self.level("furnace") == 0 {
                    "Raw sales · furnace locked"
                } else {
                    "Reserves protected"
                }
                .into(),
            },
            Stage {
                name: "Dispatch".into(),
                rate: 0.,
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
        for _ in 0..effective {
            self.second(cat, true)
        }
        self.offline = Some(Offline {
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
            "buy" => {
                let allowed = [
                    "worker",
                    "housing",
                    "drill",
                    "conveyor",
                    "sorter",
                    "furnace",
                    "shaft",
                    "steelworks",
                    "power",
                    "chemical",
                    "electrolytic",
                    "trace",
                    "recovery",
                    "capacity",
                    "reclaimer",
                    "manufacturing",
                ];
                if !allowed.contains(&a.target.as_str()) {
                    return Err("Unknown upgrade".into());
                }
                if a.target == "worker" && self.workers >= self.housing {
                    return Err("Build more housing".into());
                }
                if self.level(&a.target) >= 50 {
                    return Err("Maximum level".into());
                }
                let cost = self.cost(&a.target);
                if self.credits < cost {
                    return Err("Insufficient credits".into());
                }
                self.credits -= cost;
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
                let c = self
                    .contracts
                    .get_mut(a.value as usize)
                    .ok_or("Unknown contract")?;
                if c.complete {
                    return Err("Contract already delivered".into());
                }
                let q = self.products.entry(c.product.clone()).or_default();
                if *q < c.amount {
                    return Err("More product required".into());
                }
                *q -= c.amount;
                self.delivered_mass += c.amount;
                c.complete = true;
                self.credits += 50;
                self.research += 1;
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
                let award = self.award();
                if a.value > 2 {
                    return Err("Unknown site".into());
                }
                let mut next = Game::new(self.seed.wrapping_add(7919 + a.value), self.site + 1);
                next.research = self.research + award;
                next.discoveries = self.discoveries.clone();
                next.ranks = self.ranks.clone();
                next.records = self.records.clone();
                next.records.push(Record {
                    site: self.site,
                    depth: self.depth(),
                    research: award,
                    excavated: self.excavated,
                });
                next.last_saved = self.last_saved;
                next.megaproject = self.megaproject;
                next.enabled_recipes = self.enabled_recipes.clone();
                next.policy = self.policy.clone();
                next.priorities = self.priorities.clone();
                if next.ranks.values().any(|r| *r >= 3) {
                    next.levels.insert("conveyor".into(), 1);
                }
                *self = next;
            }
            _ => return Err("Unknown command".into()),
        }
        self.last_sequence = a.sequence;
        Ok(())
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != VERSION {
            return Err("Unsupported save version".into());
        }
        if self.heights.len() != 64
            || self.heights.iter().any(|h| *h > 100000)
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
        let n = materials().len();
        if self
            .products
            .values()
            .chain(self.ore.values())
            .chain(self.hauled.values())
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
            + g.products.values().sum::<u64>()
            + g.tailings.values().sum::<u64>()
            + g.sold_mass
            + g.delivered_mass;
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
