//! Knowledge-limited underground planning. Coordinates are fine terrain cells.
use crate::{
    geometry::{MAX_ROWS, PIT_ROWS, WIDTH},
    terrain::Terrain,
};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
#[derive(Deserialize)]
pub struct Mining {
    pub signal_radius: u32,
    pub upgraded_signal_radius: u32,
    pub sample_radius: u32,
    pub signal_tile: u32,
    pub section_length: u32,
    pub clearance: u32,
    pub chamber_height: u32,
    pub pillar_width: u32,
    pub search_budget: u32,
    pub search_limit: u32,
    pub search_radius: u32,
    pub lift_work: u64,
    pub passage_work: u64,
    pub support_work: u64,
}
pub fn settings() -> &'static Mining {
    static CONFIG: std::sync::OnceLock<Mining> = std::sync::OnceLock::new();
    CONFIG.get_or_init(|| {
        serde_json::from_str(include_str!("../../../content/mining.json"))
            .expect("validated mining content")
    })
}
pub fn validate_content() -> Result<(), String> {
    let c = settings();
    if !(16..=256).contains(&c.signal_radius)
        || c.upgraded_signal_radius < c.signal_radius
        || c.upgraded_signal_radius > 256
        || !(4..=16).contains(&c.sample_radius)
        || c.signal_tile != 32
        || c.section_length != 16
        || c.clearance != 8
        || c.chamber_height != 16
        || c.pillar_width != 8
        || !(1..=256).contains(&c.search_budget)
        || !(128..=8192).contains(&c.search_limit)
        || !(32..=256).contains(&c.search_radius)
        || c.lift_work == 0
        || c.lift_work > 10000
        || c.passage_work == 0
        || c.passage_work > 10000
        || c.support_work == 0
        || c.support_work > 100000
    {
        return Err("Invalid underground planning content".into());
    }
    Ok(())
}
pub type Point = [u32; 2];
fn key(p: Point) -> u32 {
    p[1] * WIDTH + p[0]
}
fn point(k: u32) -> Point {
    [k % WIDTH, k / WIDTH]
}
fn area(p: Point) -> u32 {
    (p[1] / 4) * (WIDTH / 4) + p[0] / 4
}
fn estimate(a: Point, b: Point) -> u64 {
    a[0].abs_diff(b[0]) as u64 * 5 + a[1].abs_diff(b[1]) as u64 * 12
}
fn distance(a: Point, b: Point) -> u32 {
    a[0].abs_diff(b[0]) + a[1].abs_diff(b[1])
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Passage {
    pub feet: Point,
    pub parent: usize,
    pub lift: bool,
    pub supported: bool,
    #[serde(default)]
    pub column: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Signal {
    pub centre: Point,
    pub radius: u32,
    pub confidence: u32,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Section {
    pub from: usize,
    pub to: Point,
    pub lift: bool,
    pub cells: Vec<Point>,
    pub support_work: u64,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Search {
    pub goal: Point,
    pub origin: Point,
    pub open: BinaryHeap<Reverse<(u64, u64, u32)>>,
    pub costs: BTreeMap<u32, u64>,
    pub previous: BTreeMap<u32, (u32, bool)>,
    pub sources: BTreeMap<u32, usize>,
    pub expanded: u32,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Workings {
    #[serde(default)]
    pub veins: BTreeMap<String, VeinSurvey>,
    #[serde(default)]
    pub target: Option<Point>,
    /// Stable deposit order; anchor survives excavation of the selected cell.
    #[serde(default)]
    pub target_deposit: Option<String>,
    /// Derived private geometry. Rebuilt after load; never sent to the client.
    #[serde(skip)]
    pub target_cells: Vec<Point>,
    pub passages: Vec<Passage>,
    #[serde(default)]
    pub chambers: BTreeMap<usize, u32>,
    pub signals: Vec<Signal>,
    pub surveyed: BTreeSet<u32>,
    pub exhausted: BTreeSet<u32>,
    #[serde(default)]
    pub deferred: BTreeSet<u32>,
    #[serde(default)]
    pub deferred_at: (u64, usize),
    pub section: Option<Section>,
    pub search: Option<Search>,
    pub active: usize,
    pub revision: u64,
    pub status: String,
    pub survey_work: u64,
    pub blocked_at: Option<(u64, u64, u32)>,
    #[serde(skip)]
    floor_index: BTreeMap<u32, BTreeSet<u32>>,
    #[serde(skip)]
    node_index: BTreeMap<u32, usize>,
    #[serde(skip)]
    indexed: usize,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct VeinSurvey {
    pub anchor: Point,
    pub stage: u32,
}
impl Workings {
    pub fn prepare_deposit_order(&mut self, seed: u64, profile: usize, cat: &[crate::Material]) {
        if self.target_cells.is_empty() {
            if let Some(anchor) = self.target {
                self.target_deposit = crate::geology::deposit_id(seed, profile, anchor, cat);
                self.target_cells = crate::geology::deposit_cells(seed, profile, anchor, cat);
            }
        }
    }

    /// Reveal the facing edge at accuracy one, the remaining deposit at accuracy two.
    /// One descriptor per survey cycle bounds foreground work and snapshot changes.
    pub fn refine_survey(
        &mut self,
        terrain: &mut Terrain,
        seed: u64,
        profile: usize,
        cat: &[crate::Material],
        accuracy: u32,
    ) -> Vec<usize> {
        let stage = accuracy.min(2);
        if stage == 0 || self.passages.is_empty() {
            return vec![];
        }
        let at = self.passages[self.active.min(self.passages.len() - 1)].feet;
        let next = self
            .veins
            .iter()
            .filter(|(_, v)| {
                v.stage < stage && distance(v.anchor, at) <= settings().upgraded_signal_radius * 2
            })
            .min_by_key(|(_, v)| distance(v.anchor, at))
            .map(|(id, v)| (id.clone(), v.anchor));
        let Some((id, anchor)) = next else {
            return vec![];
        };
        let cells = crate::geology::deposit_cells(seed, profile, anchor, cat);
        let nearest = cells
            .iter()
            .filter(|p| !terrain.contains(p[0], p[1]))
            .map(|p| distance(*p, at))
            .min()
            .unwrap_or(0);
        let mut found = BTreeSet::new();
        for p in cells {
            if !terrain.contains(p[0], p[1]) && (stage == 2 || distance(p, at) <= nearest + 6) {
                found.extend(terrain.reveal(seed, profile, p[0], p[1], 0, cat));
            }
        }
        self.veins.get_mut(&id).unwrap().stage = stage;
        self.revision += 1;
        found.into_iter().collect()
    }
    fn index_passages(&mut self) {
        while self.indexed < self.passages.len() {
            let n = &self.passages[self.indexed];
            self.node_index.insert(key(n.feet), self.indexed);
            if !n.lift {
                self.floor_index
                    .entry(n.feet[0])
                    .or_default()
                    .insert(n.feet[1]);
            }
            self.indexed += 1;
        }
    }
    pub fn initialise(&mut self) {
        if self.passages.is_empty() {
            self.passages.push(Passage {
                feet: [WIDTH / 2, PIT_ROWS - 1],
                parent: 0,
                lift: false,
                supported: true,
                column: true,
            });
            self.revision += 1;
        }
    }
    /// Survey is the ONLY planner boundary allowed to query hidden geology.
    /// Measurements collapse into 8m cells without material identities or outlines.
    pub fn survey(
        &mut self,
        terrain: &mut Terrain,
        seed: u64,
        profile: usize,
        cat: &[crate::Material],
        upgraded: bool,
        directed: bool,
    ) -> Vec<usize> {
        if self.passages.is_empty() {
            return vec![];
        }
        let at = self.passages[self.active].feet;
        let radius = if upgraded || directed {
            settings().upgraded_signal_radius
        } else {
            settings().signal_radius
        };
        let mut found = terrain.reveal(seed, profile, at[0], at[1], settings().sample_radius, cat);
        let mut measured = false;
        for by in (at[1].saturating_sub(radius) / 32)..=((at[1] + radius).min(MAX_ROWS - 1) / 32) {
            for bx in (at[0].saturating_sub(radius) / 32)..=((at[0] + radius).min(WIDTH - 1) / 32) {
                let centre = [bx * 32 + 16, by * 32 + 16];
                if centre[1] < PIT_ROWS
                    || distance(at, centre) > radius
                    || !self.surveyed.insert(key(centre))
                {
                    continue;
                }
                measured = true;
                let mut hits = 0;
                let mut anchor = None;
                for oy in [4, 12, 20, 28] {
                    for ox in [4, 12, 20, 28] {
                        let id =
                            crate::geology::sample(seed, profile, bx * 32 + ox, by * 32 + oy, cat);
                        if id > 1 {
                            hits += 1;
                            anchor.get_or_insert([bx * 32 + ox, by * 32 + oy]);
                        }
                    }
                }
                if hits >= 2 {
                    if let Some(anchor) = anchor {
                        if let Some(id) = crate::geology::deposit_id(seed, profile, anchor, cat) {
                            self.veins
                                .entry(id)
                                .or_insert(VeinSurvey { anchor, stage: 0 });
                        }
                    }
                    self.signals.push(Signal {
                        centre,
                        radius: 23,
                        confidence: if hits >= 8 { 2 } else { 1 },
                    });
                }
            }
        }
        if measured || directed {
            self.revision += 1;
        }
        let signals_before = self.signals.len();
        self.signals
            .retain(|s| !self.exhausted.contains(&key(s.centre)));
        if self.signals.len() != signals_before {
            self.revision += 1;
        }
        found.sort_unstable();
        found.dedup();
        found
    }
    pub fn next_cell(&self, terrain: &Terrain) -> Option<u32> {
        self.section
            .as_ref()?
            .cells
            .iter()
            .find(|&&p| terrain.frontier.contains(&key(p)))
            .map(|&p| key(p))
    }
    pub fn working_route(&self) -> Vec<Point> {
        if self.passages.is_empty() {
            return vec![];
        }
        let mut i = self.section.as_ref().map(|s| s.from).unwrap_or(self.active);
        let mut result = vec![];
        loop {
            let n = &self.passages[i];
            result.push(n.feet);
            if i == 0 {
                break;
            }
            i = n.parent;
        }
        result
    }
    pub fn is_lift_edge(&self, a: Point, b: Point) -> bool {
        if a[0] != b[0] {
            return false;
        }
        let Some(mut i) = self
            .node_index
            .get(&key(a))
            .copied()
            .or_else(|| self.passages.iter().position(|n| n.feet == a))
        else {
            return false;
        };
        while i > 0 {
            let n = &self.passages[i];
            if !n.lift {
                return false;
            }
            let parent = &self.passages[n.parent];
            if parent.feet == b {
                return true;
            }
            if parent.feet[0] != a[0] {
                return false;
            }
            i = n.parent;
        }
        false
    }
    pub fn survey_pending(&self, terrain: &Terrain, upgraded: bool) -> bool {
        if self
            .signals
            .iter()
            .any(|s| self.exhausted.contains(&key(s.centre)))
        {
            return true;
        }
        if self.passages.is_empty() {
            return false;
        }
        let at = self.passages[self.active].feet;
        let r = if upgraded {
            settings().upgraded_signal_radius
        } else {
            settings().signal_radius
        };
        for by in at[1].saturating_sub(r) / 32..=(at[1] + r).min(MAX_ROWS - 1) / 32 {
            for bx in at[0].saturating_sub(r) / 32..=(at[0] + r).min(WIDTH - 1) / 32 {
                let centre = [bx * 32 + 16, by * 32 + 16];
                if centre[1] >= PIT_ROWS
                    && distance(at, centre) <= r
                    && !self.surveyed.contains(&key(centre))
                {
                    return true;
                }
            }
        }
        let r = settings().sample_radius;
        (at[0].saturating_sub(r)..=(at[0] + r).min(WIDTH - 1)).any(|x| {
            (at[1].saturating_sub(r)..=(at[1] + r).min(MAX_ROWS - 1)).any(|y| {
                x.abs_diff(at[0]).pow(2) + y.abs_diff(at[1]).pow(2) <= r * r
                    && !terrain.is_revealed(x, y)
            })
        })
    }
    /// Bounded deterministic planning; no geology access, seed or catalogue arguments.
    pub fn advance(
        &mut self,
        terrain: &Terrain,
        priorities: &[usize],
        policy: &str,
        depth_limit: u32,
        engineers: u32,
        supports: u32,
    ) {
        self.initialise();
        self.index_passages();
        let geometry_stamp = (terrain.revision, self.passages.len());
        if self.deferred_at != geometry_stamp {
            self.deferred.clear();
            self.deferred_at = geometry_stamp;
        }
        let stamp = (terrain.revision, self.revision, depth_limit);
        if self.blocked_at == Some(stamp) {
            return;
        }
        self.blocked_at = None;
        if let Some(s) = &mut self.section {
            if s.cells.iter().any(|p| !terrain.contains(p[0], p[1])) {
                self.status = "Opening access / extracting vein".into();
                return;
            }
            self.status = "Waiting for supports".into();
            s.support_work += 80 * engineers.max(1) as u64 * (1 + supports) as u64;
            if s.support_work < settings().support_work {
                return;
            }
            let s = self.section.take().unwrap();
            if s.to == self.passages[s.from].feet {
                let roof = s.cells.iter().map(|p| p[1]).min().unwrap_or(s.to[1]);
                self.chambers
                    .entry(s.from)
                    .and_modify(|y| *y = (*y).min(roof))
                    .or_insert(roof);
                self.active = s.from;
                self.revision += 1;
                return;
            }
            let mut span = distance(self.passages[s.from].feet, s.to);
            let mut ancestor = s.from;
            while ancestor > 0
                && !self.passages[ancestor].column
                && !self.passages[ancestor].lift
                && span < settings().section_length
            {
                let node = &self.passages[ancestor];
                span += distance(node.feet, self.passages[node.parent].feet);
                ancestor = node.parent;
            }
            let column = !s.lift
                && (span >= settings().section_length || self.passages[s.from].lift || s.from == 0);
            self.passages.push(Passage {
                column,
                feet: s.to,
                parent: s.from,
                lift: s.lift,
                supported: true,
            });
            self.active = self.passages.len() - 1;
            self.index_passages();
            self.revision += 1;
        }
        if self.search.is_none() {
            let deepest = self.passages.iter().map(|n| n.feet[1]).max().unwrap();
            let access_blocked = depth_limit <= deepest + 17;
            let development = policy == "depth" && !access_blocked;
            let nearest_distance = |p: Point| {
                if access_blocked {
                    // Reuse older surveyed workings when the next equipment gate stops
                    // access development. The derived spatial index bounds this lookup.
                    self.node_index
                        .range(
                            key([0, p[1].saturating_sub(256)])
                                ..=key([WIDTH - 1, p[1].saturating_add(256)]),
                        )
                        .map(|(_, &i)| distance(self.passages[i].feet, p))
                        .min()
                        .unwrap_or(u32::MAX)
                } else {
                    self.passages
                        .iter()
                        .rev()
                        .take(128)
                        .map(|n| distance(n.feet, p))
                        .min()
                        .unwrap_or(u32::MAX)
                }
            };
            let mut candidates: Vec<(i64, Point)> = vec![];
            // A deposit order persists across individual cuts. Unknown cells never
            // enter the planner, even when they belong to the selected descriptor.
            for &p in &self.target_cells {
                if !terrain.contains(p[0], p[1])
                    && p[1] < depth_limit
                    && terrain.known_material(p[0], p[1]).is_some_and(|id| id > 1)
                    && !self.deferred.contains(&area(p))
                    && !protected(&self.floor_index, p, &[])
                    && nearest_distance(p) <= settings().search_radius
                {
                    candidates.push((-10000 + nearest_distance(p) as i64, p));
                }
            }
            // Targets come only from locally sampled frontier cells.
            for (&id, faces) in &terrain.ore_frontiers {
                if id <= 1 {
                    continue;
                }
                for &k in faces {
                    let p = point(k);
                    if p[1] < PIT_ROWS
                        || p[1] >= depth_limit
                        || self.deferred.contains(&area(p))
                        || protected(&self.floor_index, p, &[])
                        || !access_blocked && p[1] + 256 < deepest
                    {
                        continue;
                    }
                    let near = nearest_distance(p);
                    if near > 256 {
                        continue;
                    }
                    let bonus = if policy == "vein" && priorities.contains(&id) {
                        120
                    } else {
                        40
                    };
                    candidates.push((near as i64 - bonus, p));
                }
            }
            for s in &self.signals {
                if s.centre[1] >= depth_limit
                    || self.exhausted.contains(&key(s.centre))
                    || self.deferred.contains(&area(s.centre))
                    || !access_blocked && s.centre[1] + 256 < deepest
                {
                    continue;
                }
                let near = nearest_distance(s.centre);
                if near <= 256 {
                    candidates.push((near as i64 - 12 * s.confidence as i64, s.centre));
                }
            }
            if development {
                // Develop access toward deeper surveyed ground. Nearby shallow ore remains
                // available when the foreman switches back to an extraction policy.
                candidates.retain(|(_, p)| p[1] > deepest + 4);
                for (score, p) in &mut candidates {
                    *score -= (p[1] - deepest) as i64;
                }
            }
            candidates.sort_unstable();
            let goal = candidates.first().map(|(_, p)| *p).unwrap_or_else(|| {
                let n = self.passages.iter().max_by_key(|n| n.feet[1]).unwrap();
                [
                    n.feet[0],
                    (n.feet[1] + 16).min(depth_limit.saturating_sub(1)),
                ]
            });
            // Reachable sampled ore becomes a local extraction area, not a new access shaft.
            if !development
                && terrain
                    .known_material(goal[0], goal[1])
                    .is_some_and(|id| id > 1)
            {
                if let Some((anchor, node)) = self
                    .passages
                    .iter()
                    .enumerate()
                    .rev()
                    .take(if access_blocked {
                        self.passages.len()
                    } else {
                        512
                    })
                    .find(|(_, n)| {
                        n.supported
                            && n.feet[0].abs_diff(goal[0]) <= 8
                            && goal[1] <= n.feet[1]
                            && n.feet[1] - goal[1] < 16
                    })
                {
                    let mut cells = vec![];
                    for x in node.feet[0].min(goal[0])..=node.feet[0].max(goal[0]) {
                        let roof = if x == goal[0] {
                            goal[1].min(node.feet[1].saturating_sub(7))
                        } else {
                            node.feet[1].saturating_sub(7)
                        };
                        for y in roof.max(PIT_ROWS)..=node.feet[1] {
                            cells.push([x, y]);
                        }
                    }
                    // Batch nearby samples into one bounded work area and one support task.
                    let mut roofs = BTreeMap::new();
                    for x in node.feet[0].saturating_sub(8)..=(node.feet[0] + 8).min(WIDTH - 1) {
                        if let Some(y) = (node.feet[1].saturating_sub(15).max(PIT_ROWS)
                            ..=node.feet[1])
                            .find(|&y| {
                                terrain.known_material(x, y).is_some_and(|id| id > 1)
                                    && !terrain.contains(x, y)
                            })
                        {
                            roofs.insert(x, y.min(node.feet[1].saturating_sub(7)));
                        }
                    }
                    let left = roofs
                        .keys()
                        .next()
                        .copied()
                        .unwrap_or(goal[0])
                        .min(node.feet[0]);
                    let right = roofs
                        .keys()
                        .next_back()
                        .copied()
                        .unwrap_or(goal[0])
                        .max(node.feet[0]);
                    let mut area = vec![];
                    for x in left..=right {
                        let roof = roofs
                            .get(&x)
                            .copied()
                            .unwrap_or(node.feet[1].saturating_sub(7));
                        for y in roof.max(PIT_ROWS)..=node.feet[1] {
                            area.push([x, y]);
                        }
                    }
                    if !area.iter().any(|c| {
                        crate::geometry::protects_ramp(c[0], c[1])
                            || protected(&self.floor_index, *c, &[])
                    }) {
                        cells = area;
                    }
                    let safe = !cells.iter().any(|c| {
                        crate::geometry::protects_ramp(c[0], c[1])
                            || protected(&self.floor_index, *c, &[])
                    });
                    if safe && cells.iter().any(|p| !terrain.contains(p[0], p[1])) {
                        self.active = anchor;
                        self.section = Some(Section {
                            from: anchor,
                            to: node.feet,
                            lift: false,
                            cells,
                            support_work: 0,
                        });
                        self.status = "Extracting sampled vein".into();
                        self.revision += 1;
                        return;
                    }
                }
            }
            if goal[1] < PIT_ROWS || depth_limit <= deepest + 1 && candidates.is_empty() {
                self.status = "Depth equipment required".into();
                self.blocked_at = Some(stamp);
                return;
            }
            let mut nearest: Vec<_> = self
                .passages
                .iter()
                .enumerate()
                .filter(|(_, n)| n.supported)
                .map(|(i, n)| (distance(n.feet, goal), i))
                .collect();
            nearest.sort_unstable();
            nearest.truncate(8);
            let mut search = Search {
                goal,
                origin: goal,
                open: BinaryHeap::new(),
                costs: BTreeMap::new(),
                previous: BTreeMap::new(),
                sources: BTreeMap::new(),
                expanded: 0,
            };
            for (_, i) in nearest {
                let p = self.passages[i].feet;
                // Include real existing haul length to discourage needless remote branches.
                let mut haul = 0;
                let mut j = i;
                while j > 0 {
                    let n = &self.passages[j];
                    haul += distance(n.feet, self.passages[n.parent].feet) as u64;
                    j = n.parent;
                }
                let cost = haul / 16;
                search.costs.insert(key(p), cost);
                search.sources.insert(key(p), i);
                search
                    .open
                    .push(Reverse((cost + estimate(p, goal), cost, key(p))));
            }
            self.search = Some(search);
            self.status = "Investigating signal / planning access".into();
        }
        let search = self.search.as_mut().unwrap();
        let mut reached = None;
        for _ in 0..settings().search_budget {
            let Some(Reverse((_, cost, k))) = search.open.pop() else {
                break;
            };
            if search.costs.get(&k) != Some(&cost) {
                continue;
            }
            search.expanded += 1;
            let p = point(k);
            // A target lies inside 2m passage clearance; do not chase its centre through a floor.
            if p[0].abs_diff(search.goal[0]) <= 3
                && p[1] >= search.goal[1]
                && p[1] - search.goal[1] < 8
                && !search.sources.contains_key(&k)
            {
                reached = Some(k);
                break;
            }
            let bottom = depth_limit.min(MAX_ROWS - 8).saturating_sub(1);
            let descend = bottom.saturating_sub(p[1]).min(16) as i32;
            let ascend = p[1].saturating_sub(PIT_ROWS - 1).min(16) as i32;
            for (dx, dy, lift) in [
                (-4, -1, false),
                (-4, 0, false),
                (-4, 1, false),
                (4, -1, false),
                (4, 0, false),
                (4, 1, false),
                (0, descend, true),
                (0, -ascend, true),
            ] {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = p[0] as i32 + dx;
                let ny = p[1] as i32 + dy;
                if nx < 8
                    || nx >= WIDTH as i32 - 8
                    || ny < PIT_ROWS as i32 - 1
                    || ny >= depth_limit as i32
                    || ny >= MAX_ROWS as i32 - 8
                {
                    continue;
                }
                let q = [nx as u32, ny as u32];
                if distance(q, search.origin) > settings().search_radius {
                    continue;
                }
                let Some(solid) = cut_cost(terrain, &self.floor_index, p, q, lift) else {
                    continue;
                };
                let step = solid
                    + if lift {
                        // A short terminal extension reuses the existing lift service.
                        (settings().lift_work * distance(p, q) as u64)
                            .div_ceil(settings().section_length as u64)
                    } else {
                        settings().passage_work
                    }
                    + distance(p, q) as u64;
                let nk = key(q);
                let nc = cost + step;
                if nc < search.costs.get(&nk).copied().unwrap_or(u64::MAX) {
                    search.costs.insert(nk, nc);
                    search.previous.insert(nk, (k, lift));
                    search
                        .open
                        .push(Reverse((nc + estimate(q, search.goal), nc, nk)));
                }
            }
            if search.expanded >= settings().search_limit {
                break;
            }
        }
        if let Some(mut k) = reached {
            let mut path = vec![];
            while let Some(&(prev, lift)) = search.previous.get(&k) {
                path.push((point(k), lift));
                k = prev;
            }
            let from = search.sources[&k];
            path.reverse();
            let start = self.passages[from].feet;
            let mode = path[0].1;
            let mut end = start;
            let first = path[0].0;
            let heading = [
                first[0] as i64 - start[0] as i64,
                first[1] as i64 - start[1] as i64,
            ];
            for (p, lift) in path {
                let delta = [p[0] as i64 - end[0] as i64, p[1] as i64 - end[1] as i64];
                if lift != mode
                    || delta != heading
                    || distance(start, p) > settings().section_length
                {
                    break;
                }
                end = p;
            }
            if end == start {
                self.deferred.insert(area(search.goal));
                self.search = None;
                return;
            }
            let mut cells = cut_cells(start, end, mode);
            cells.sort_unstable();
            cells.dedup();
            if self.passages.iter().any(|n| n.feet == end) {
                self.deferred.insert(area(search.goal));
                self.search = None;
                return;
            }
            if distance(end, search.goal) <= settings().sample_radius {
                self.exhausted.insert(key(search.goal));
            }
            self.active = from;
            self.section = Some(Section {
                from,
                to: end,
                lift: mode,
                cells,
                support_work: 0,
            });
            self.search = None;
            self.revision += 1;
        } else if search.open.is_empty() || search.expanded >= settings().search_limit {
            self.deferred.insert(area(search.goal));
            self.search = None;
        }
    }
    pub fn valid(&self) -> bool {
        if self
            .target_deposit
            .as_ref()
            .is_some_and(|id| id.len() > 128)
            || self.veins.len() > 25000
            || self.veins.iter().any(|(id, v)| {
                id.len() > 128 || v.anchor[0] >= WIDTH || v.anchor[1] >= MAX_ROWS || v.stage > 2
            })
            || self
                .target
                .is_some_and(|p| p[0] >= WIDTH || p[1] >= MAX_ROWS)
        {
            return false;
        }
        self.chambers.iter().all(|(i, y)| {
            *i < self.passages.len()
                && *y >= PIT_ROWS
                && *y <= self.passages[*i].feet[1]
                && self.passages[*i].feet[1] - *y < 16
        }) && self.passages.len() <= 200_000
            && self.signals.len() <= 100_000
            && self.deferred.len() <= 100_000
            && self
                .deferred
                .iter()
                .all(|k| *k < MAX_ROWS / 4 * (WIDTH / 4))
            && self.surveyed.len() <= 1_000_000
            && self.status.len() <= 128
            && self.signals.iter().all(|s| {
                s.centre[0] < WIDTH
                    && s.centre[1] < MAX_ROWS
                    && s.radius == 23
                    && (1..=2).contains(&s.confidence)
            })
            && (self.passages.is_empty() || self.active < self.passages.len())
            && self.passages.iter().enumerate().all(|(i, n)| {
                n.feet[0] < WIDTH
                    && n.feet[1] < MAX_ROWS
                    && (i == 0 && n.parent == 0 || n.parent < i)
            })
            && self.section.as_ref().is_none_or(|s| {
                s.from < self.passages.len()
                    && s.cells.len() <= 2048
                    && s.cells.iter().all(|p| p[0] < WIDTH && p[1] < MAX_ROWS)
                    && s.to[0] < WIDTH
                    && s.to[1] < MAX_ROWS
            })
            && self.search.as_ref().is_none_or(|s| {
                s.costs.len() <= 32768
                    && s.open.len() <= 65536
                    && s.previous.len() <= 32768
                    && s.sources.iter().all(|(k, i)| {
                        *i < self.passages.len()
                            && key(self.passages[*i].feet) == *k
                            && !s.previous.contains_key(k)
                    })
                    && s.goal[0] < WIDTH
                    && s.goal[1] < MAX_ROWS
                    && s.previous.iter().all(|(k, (prev, _))| {
                        s.costs
                            .get(prev)
                            .zip(s.costs.get(k))
                            .is_some_and(|(a, b)| a < b)
                    })
            })
    }
}
/// Same cells and safety rules as a committed cut, without allocating cells
/// or repeating chunk/floor lookups for every fine pixel during path search.
fn cut_cost(
    terrain: &Terrain,
    floors: &BTreeMap<u32, BTreeSet<u32>>,
    a: Point,
    b: Point,
    lift: bool,
) -> Option<u64> {
    let (left, right) = if lift {
        (a[0].saturating_sub(3), (a[0] + 3).min(WIDTH - 1))
    } else {
        (a[0].min(b[0]), a[0].max(b[0]))
    };
    let mut solid = 0;
    for x in left..=right {
        let (first, last) = if lift {
            (a[1].min(b[1]).saturating_sub(7), a[1].max(b[1]))
        } else {
            let feet = interpolate(a, b, x);
            (feet.saturating_sub(7), feet)
        };
        if !lift
            && floors.get(&x).is_some_and(|ys| {
                ys.range(first.saturating_sub(settings().pillar_width)..last)
                    .any(|&y| [x, y] != a && [x, y] != b)
            })
        {
            return None;
        }
        solid += (last - first + 1 - terrain.excavated_in_column(x, first, last)) as u64;
    }
    Some(solid)
}
fn protected(floors: &BTreeMap<u32, BTreeSet<u32>>, p: Point, except: &[Point]) -> bool {
    floors.get(&p[0]).is_some_and(|ys| {
        ys.range(p[1].saturating_sub(settings().pillar_width)..p[1])
            .any(|&y| !except.contains(&[p[0], y]))
    })
}
fn interpolate(a: Point, b: Point, x: u32) -> u32 {
    if a[0] == b[0] {
        return b[1];
    }
    let (left, right) = if a[0] < b[0] { (a, b) } else { (b, a) };
    (left[1] as i64
        + ((right[1] as i64 - left[1] as i64) * (x as i64 - left[0] as i64))
            .div_euclid((right[0] - left[0]) as i64)) as u32
}
pub fn cut_cells(a: Point, b: Point, lift: bool) -> Vec<Point> {
    let mut cells = vec![];
    if lift {
        for y in a[1].min(b[1]).saturating_sub(7)..=a[1].max(b[1]) {
            for x in a[0].saturating_sub(3)..=(a[0] + 3).min(WIDTH - 1) {
                cells.push([x, y]);
            }
        }
    } else {
        for x in a[0].min(b[0])..=a[0].max(b[0]) {
            let feet = interpolate(a, b, x);
            for y in feet.saturating_sub(7)..=feet {
                cells.push([x, y]);
            }
        }
    }
    cells
}
#[cfg(test)]
mod tests {
    use super::*;
    const PIT_ROWS: u32 = 192;
    #[test]
    fn deposit_order_survives_selected_cell_and_save_without_leaking_geometry() {
        let cat = crate::materials();
        let mut w = Workings::default();
        let anchor = (220..250)
            .map(|x| [x, 24])
            .find(|p| crate::geology::sample(42, 0, p[0], p[1], &cat) == 3)
            .unwrap();
        w.target = Some(anchor);
        w.prepare_deposit_order(42, 0, &cat);
        assert!(w.target_cells.len() > 100);
        let identity = w.target_deposit.clone();
        let encoded = serde_json::to_string(&w).unwrap();
        assert!(!encoded.contains("target_cells"));
        let mut restored: Workings = serde_json::from_str(&encoded).unwrap();
        assert!(restored.target_cells.is_empty());
        restored.prepare_deposit_order(42, 0, &cat);
        assert_eq!(restored.target_deposit, identity);
        assert_eq!(restored.target_cells, w.target_cells);
        let mut t = Terrain::default();
        for y in 0..=anchor[1] {
            assert!(t.excavate(anchor[0], y));
        }
        restored.initialise();
        restored.advance(&t, &[], "vein", 1200, 1, 0);
        assert_eq!(restored.target, Some(anchor));
        // No private target cell may become a search goal before revelation.
        if let Some(search) = &restored.search {
            assert!(!restored.target_cells.contains(&search.goal));
        }
    }
    #[test]
    fn accuracy_reveals_facing_edge_then_only_remaining_deposit() {
        let cat = crate::materials();
        let mut terrain = Terrain::default();
        let mut w = Workings::default();
        w.initialise();
        let anchor = (220..250)
            .map(|x| [x, 24])
            .find(|p| crate::geology::sample(42, 0, p[0], p[1], &cat) == 3)
            .unwrap();
        let id = crate::geology::deposit_id(42, 0, anchor, &cat).unwrap();
        let cells = crate::geology::deposit_cells(42, 0, anchor, &cat);
        w.veins.insert(id.clone(), VeinSurvey { anchor, stage: 0 });
        assert!(w.refine_survey(&mut terrain, 42, 0, &cat, 0).is_empty());
        assert!(terrain.visible.is_empty());
        w.refine_survey(&mut terrain, 42, 0, &cat, 1);
        let count = |t: &Terrain| {
            t.visible
                .values()
                .flatten()
                .filter(|&&id| id != 255)
                .count()
        };
        let edge = count(&terrain);
        assert!(edge > 0 && edge < cells.len());
        w.refine_survey(&mut terrain, 42, 0, &cat, 2);
        assert_eq!(count(&terrain), cells.len());
        assert!(cells
            .iter()
            .all(|p| terrain.known_material(p[0], p[1]) == Some(3)));
        assert!(!terrain.is_revealed(400, 24));
        let restored: Workings = serde_json::from_str(&serde_json::to_string(&w).unwrap()).unwrap();
        assert_eq!(restored.veins[&id].stage, 2);
    }
    #[test]
    fn column_search_cost_matches_pixel_cuts_across_chunks_ramps_and_supports() {
        for pattern in 0..3 {
            let mut terrain = Terrain::default();
            for id in 0..64 {
                terrain.chunks.insert(
                    id,
                    (0..512)
                        .map(|byte| match pattern {
                            0 => 0,
                            1 => 255,
                            _ => ((id * 17 + byte * 29) as u8) ^ 0x5a,
                        })
                        .collect(),
                );
            }
            for supports in [false, true] {
                let mut floors = BTreeMap::new();
                if supports {
                    for x in (8..WIDTH - 8).step_by(5) {
                        floors.insert(
                            x,
                            [184, 191, 192, 200, 248, 256, 300, 512]
                                .into_iter()
                                .collect(),
                        );
                    }
                }
                for x in [
                    8, 16, 31, 63, 64, 65, 128, 191, 192, 207, 208, 255, 256, 257, 500,
                ] {
                    for y in [191, 192, 193, 207, 223, 255, 256, 257, 300, 511, 512, 1023] {
                        let a = [x, y];
                        for (dx, dy, lift) in [
                            (-4, -1, false),
                            (-4, 0, false),
                            (-4, 1, false),
                            (4, -1, false),
                            (4, 0, false),
                            (4, 1, false),
                            (0, 16, true),
                            (0, -16, true),
                        ] {
                            let b = [(x as i32 + dx) as u32, (y as i32 + dy) as u32];
                            let cells = cut_cells(a, b, lift);
                            let unsafe_cut = cells
                                .iter()
                                .any(|p| crate::geometry::protects_ramp(p[0], p[1]))
                                || !lift && cells.iter().any(|p| protected(&floors, *p, &[a, b]));
                            let reference = (!unsafe_cut).then(|| {
                                cells
                                    .iter()
                                    .filter(|p| !terrain.contains(p[0], p[1]))
                                    .count() as u64
                            });
                            assert_eq!(cut_cost(&terrain, &floors, a, b, lift), reference,
                                "pattern {pattern}, supports {supports}, {a:?} -> {b:?}, lift {lift}");
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn planning_is_knowledge_limited_and_resumes_identically() {
        let mut a = Workings::default();
        a.initialise();
        a.passages[0].feet = [256, 191];
        a.signals.push(Signal {
            centre: [300, 205],
            radius: 23,
            confidence: 2,
        });
        let t = Terrain::from_columns(&(0..WIDTH).map(|_| PIT_ROWS).collect::<Vec<_>>());
        a.advance(&t, &[], "vein", 1200, 1, 0);
        let mut b: Workings = serde_json::from_str(&serde_json::to_string(&a).unwrap()).unwrap();
        for _ in 0..100 {
            a.advance(&t, &[], "vein", 1200, 1, 0);
            b.advance(&t, &[], "vein", 1200, 1, 0);
            if a.section.is_some() {
                break;
            }
        }
        assert!(a.section.is_some());
        assert_eq!(
            serde_json::to_value(a).unwrap(),
            serde_json::to_value(b).unwrap()
        );
    }
    #[test]
    fn lift_closes_short_gaps_at_equipment_limit_without_horizontal_detours() {
        for gap in [1, 7, 8, 15] {
            let mut workings = Workings::default();
            workings.initialise();
            let bottom = [WIDTH / 2, 1199 - gap];
            workings.passages.push(Passage {
                feet: bottom,
                parent: 0,
                lift: true,
                supported: true,
                column: false,
            });
            workings.active = 1;
            let mut terrain = Terrain::from_columns(&vec![PIT_ROWS; WIDTH as usize]);
            for p in cut_cells(workings.passages[0].feet, bottom, true) {
                terrain.excavate(p[0], p[1]);
            }
            for _ in 0..100 {
                workings.advance(&terrain, &[], "depth", 1200, 1, 0);
                if workings.section.is_some() {
                    break;
                }
            }
            let section = workings.section.as_ref().expect("reach equipment boundary");
            assert!(section.lift, "gap {gap} should continue its existing lift");
            assert_eq!(section.from, 1);
            assert_eq!(section.to, [WIDTH / 2, 1199]);
            assert!(section.cells.iter().all(|p| p[1] < 1200));
        }
    }
    #[test]
    fn depth_policy_develops_access_instead_of_chasing_shallow_signals() {
        let mut terrain = Terrain::from_columns(&vec![PIT_ROWS; WIDTH as usize]);
        let mut workings = Workings::default();
        workings.initialise();
        for p in cut_cells([256, 191], [256, 223], true) {
            terrain.excavate(p[0], p[1]);
        }
        workings.passages.push(Passage {
            feet: [256, 223],
            parent: 0,
            lift: true,
            supported: true,
            column: false,
        });
        workings.active = 1;
        workings.signals.push(Signal {
            centre: [272, 208],
            radius: 23,
            confidence: 2,
        });
        let mut vein = workings.clone();
        vein.advance(&terrain, &[], "vein", 1200, 1, 0);
        assert!(
            vein.search.as_ref().is_some_and(|s| s.goal == [272, 208])
                || vein.exhausted.contains(&key([272, 208]))
                || vein.deferred.contains(&area([272, 208]))
                || vein.section.as_ref().is_some_and(|s| s.to[1] <= 223)
        );
        workings.advance(&terrain, &[], "depth", 1200, 1, 0);
        assert_eq!(workings.section.as_ref().unwrap().to, [256, 239]);
    }
    #[test]
    fn nearby_sampled_ore_is_extracted_without_building_another_shaft() {
        let mut t = Terrain::from_columns(&vec![PIT_ROWS; WIDTH as usize]);
        let mut w = Workings::default();
        w.initialise();
        w.passages[0].feet = [256, 191];
        for p in cut_cells([256, 191], [256, 223], true) {
            t.excavate(p[0], p[1]);
        }
        w.passages.push(Passage {
            feet: [256, 223],
            parent: 0,
            lift: true,
            supported: true,
            column: false,
        });
        w.active = 1;
        t.reveal(42, 0, 260, 216, 0, &crate::materials());
        let id = crate::geometry::chunk_id(260, 216);
        let bit = crate::geometry::bit_index(260, 216);
        t.visible.get_mut(&id).unwrap()[bit] = 3;
        t.ore_frontiers
            .entry(3)
            .or_default()
            .insert(key([260, 216]));
        w.advance(&t, &[3], "vein", 1200, 1, 0);
        assert_eq!(w.section.as_ref().unwrap().to, [256, 223]);
        while let Some(k) = w.next_cell(&t) {
            assert!(t.excavate(k % WIDTH, k / WIDTH));
        }
        w.advance(&t, &[3], "vein", 1200, 100, 10);
        assert_eq!(w.passages.len(), 2);
        assert!(w.chambers.contains_key(&1));
        assert!(!t.contains(260, 224), "retain chamber floor");
    }
    #[test]
    fn equipment_gate_recovers_sampled_ore_from_older_workings() {
        let mut t = Terrain::from_columns(&vec![PIT_ROWS; WIDTH as usize]);
        let mut w = Workings::default();
        w.initialise();
        w.passages[0].feet = [256, 191];
        for i in 1..=80 {
            let feet = [256, 191 + i * 16];
            let parent = w.passages.len() - 1;
            for p in cut_cells(w.passages[parent].feet, feet, true) {
                t.excavate(p[0], p[1]);
            }
            w.passages.push(Passage {
                feet,
                parent,
                lift: true,
                supported: true,
                column: false,
            });
        }
        w.active = w.passages.len() - 1;
        t.reveal(42, 0, 260, 216, 0, &crate::materials());
        let id = crate::geometry::chunk_id(260, 216);
        let bit = crate::geometry::bit_index(260, 216);
        t.visible.get_mut(&id).unwrap()[bit] = 3;
        t.ore_frontiers
            .entry(3)
            .or_default()
            .insert(key([260, 216]));
        w.advance(&t, &[3], "depth", 1472, 1, 0);
        let section = w
            .section
            .as_ref()
            .expect("Revisit earlier sampled ore at equipment gate");
        assert_eq!(section.to, [256, 223]);
        assert!(section.cells.contains(&[260, 216]));
    }
    #[test]
    fn clear_approaches_are_supported_without_abandoning_unsampled_signals() {
        let mut terrain = Terrain::from_columns(&vec![PIT_ROWS; WIDTH as usize]);
        let mut w = Workings::default();
        w.initialise();
        w.passages[0].feet = [256, 191];
        for p in cut_cells([256, 191], [256, 223], true) {
            terrain.excavate(p[0], p[1]);
        }
        w.passages.push(Passage {
            feet: [256, 223],
            parent: 0,
            lift: true,
            supported: true,
            column: false,
        });
        w.active = 1;
        let goal = [272, 208];
        w.signals.push(Signal {
            centre: goal,
            radius: 23,
            confidence: 2,
        });
        for _ in 0..100 {
            w.advance(&terrain, &[], "vein", 1200, 1, 0);
            if w.section.is_some() {
                break;
            }
        }
        let section = w.section.as_ref().expect("Commission the clear approach");
        assert!(section.cells.iter().all(|p| terrain.contains(p[0], p[1])));
        assert!(
            !w.exhausted.contains(&key(goal)),
            "Signal remains until samples can reach it"
        );
        w.advance(&terrain, &[], "vein", 1200, 100, 10);
        assert!(w.passages.len() > 2);
    }
    #[test]
    fn failed_areas_resume_after_geometry_changes_and_survive_reload() {
        let mut terrain = Terrain::from_columns(&vec![PIT_ROWS; WIDTH as usize]);
        let mut w = Workings::default();
        w.initialise();
        w.passages[0].feet = [256, 191];
        w.signals.push(Signal {
            centre: [4, 192],
            radius: 23,
            confidence: 2,
        });
        for _ in 0..100 {
            w.advance(&terrain, &[], "vein", 1200, 1, 0);
            if w.deferred.contains(&area([4, 192])) {
                break;
            }
        }
        assert!(w.deferred.contains(&area([4, 192])));
        assert!(!w.exhausted.contains(&key([4, 192])));
        let mut restored: Workings =
            serde_json::from_str(&serde_json::to_string(&w).unwrap()).unwrap();
        restored.advance(&terrain, &[], "vein", 1200, 1, 0);
        w.advance(&terrain, &[], "vein", 1200, 1, 0);
        assert_eq!(
            serde_json::to_value(&w).unwrap(),
            serde_json::to_value(restored).unwrap()
        );
        terrain.excavate(256, 192);
        w.advance(&terrain, &[], "depth", 1200, 1, 0);
        assert!(!w.deferred.contains(&area([4, 192])));
    }
    #[test]
    fn reversed_slopes_cover_identical_cells() {
        for (a, b) in [
            ([160, 464], [152, 466]),
            ([256, 191], [272, 195]),
            ([272, 199], [256, 195]),
        ] {
            assert_eq!(cut_cells(a, b, false), cut_cells(b, a, false));
        }
    }
    #[test]
    fn sections_respect_clearance_and_gradient() {
        let cells = cut_cells([200, 200], [216, 204], false);
        for x in 200..=216 {
            assert_eq!(cells.iter().filter(|c| c[0] == x).count(), 8);
        }
        assert!(cells.contains(&[216, 204]));
        assert!(!cells.contains(&[216, 205]));
    }
}

#[cfg(test)]
mod development_tests {
    use super::*;
    fn fixture(seed: u64) -> crate::Game {
        let mut g = crate::Game::new(seed, 1);
        g.workers = 16;
        g.housing = 32;
        g.policy = "vein".into();
        g.priorities = vec![3, 4, 19];
        for (id, n) in [
            ("shaft", 3),
            ("supports", 1),
            ("drill", 3),
            ("conveyor", 3),
            ("sorter", 5),
            ("power", 5),
            ("capacity", 5),
            ("survey", 1),
        ] {
            g.levels.insert(id.into(), n);
        }
        g
    }
    #[test]
    fn shallow_vein_branch_keeps_developing_from_surface() {
        let mut g = fixture(79246);
        let cat = crate::materials();
        for _ in 0..60 {
            g.second(&cat, true);
        }
        let mined = g.excavated;
        for _ in 0..120 {
            g.second(&cat, true);
        }
        assert!(g.excavated > mined);
        assert!(g.workings.passages.len() > 5);
        assert!(!crate::navigation::underground(&g.terrain, &g.heights, &g.workings).is_empty());
        g.validate().unwrap();
    }
    #[test]
    fn thirty_seeds_develop_connected_nonperiodic_workings() {
        let cat = crate::materials();
        for seed in 42..72 {
            let mut g = fixture(seed);
            // Allow two minutes for authored support-construction work.
            for tick in 0..2400 {
                g.tick(&cat, true);
                if tick % 100 == 0 && !g.workings.passages.is_empty() {
                    assert!(
                        !crate::navigation::underground(&g.terrain, &g.heights, &g.workings)
                            .is_empty(),
                        "seed {seed}, tick {tick}"
                    );
                }
            }
            assert!(g.workings.passages.len() > 4, "seed {seed}");
            assert!(g
                .workings
                .passages
                .iter()
                .any(|n| n.feet[1] % 96 != 7 && n.feet[0] != WIDTH / 2));
            assert!(g.validate().is_ok(), "seed {seed}: {:?}", g.validate());
        }
    }
    #[test]
    fn equipment_gate_idle_skip_preserves_survey_clock_and_state() {
        let cat = crate::materials();
        let mut a = fixture(49);
        a.levels.remove("supports");
        a.workings.initialise();
        for i in 1..=75 {
            let feet = [WIDTH / 2, PIT_ROWS - 1 + 16 * i];
            let parent = a.workings.passages.len() - 1;
            for p in cut_cells(a.workings.passages[parent].feet, feet, true) {
                a.terrain.excavate(p[0], p[1]);
            }
            a.workings.passages.push(Passage {
                feet,
                parent,
                lift: true,
                supported: true,
                column: false,
            });
        }
        a.heights[WIDTH as usize / 2] = 1200;
        a.workings.active = 75;
        a.workings
            .survey(&mut a.terrain, a.seed, a.profile, &cat, true, false);
        a.workings
            .exhausted
            .extend(a.terrain.ore_frontiers.values().flatten().copied());
        a.workings
            .exhausted
            .extend(a.workings.signals.iter().map(|s| key(s.centre)));
        a.tick(&cat, true);
        assert!(a.workings.blocked_at.is_some());
        let mut b = a.clone();
        a.last_saved = 100;
        a.advance_offline(220, &cat);
        for _ in 0..1200 {
            b.tick(&cat, true);
        }
        assert_eq!(
            serde_json::to_value(a.workings).unwrap(),
            serde_json::to_value(b.workings).unwrap()
        );
        assert_eq!(
            serde_json::to_value(a.transport).unwrap(),
            serde_json::to_value(b.transport).unwrap()
        );
        assert_eq!(a.ticks, b.ticks);
        assert_eq!(a.dig_progress, b.dig_progress);
    }
    #[test]
    fn offline_surveys_plans_supports_and_cargo_match_fixed_steps_after_reload() {
        let cat = crate::materials();
        let mut offline = fixture(49);
        for _ in 0..130 {
            offline.tick(&cat, true);
        }
        let mut stepped: crate::Game =
            serde_json::from_str(&serde_json::to_string(&offline).unwrap()).unwrap();
        stepped.migrate().unwrap();
        offline.last_saved = 100;
        offline.advance_offline(220, &cat);
        for _ in 0..1200 {
            stepped.tick(&cat, true);
        }
        assert_eq!(offline.terrain.chunks, stepped.terrain.chunks);
        assert_eq!(offline.terrain.visible, stepped.terrain.visible);
        assert_eq!(offline.credits, stepped.credits);
        assert_eq!(offline.ore, stepped.ore);
        assert_eq!(
            serde_json::to_value(&offline.transport).unwrap(),
            serde_json::to_value(&stepped.transport).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&offline.workings).unwrap(),
            serde_json::to_value(&stepped.workings).unwrap()
        );
    }
}
