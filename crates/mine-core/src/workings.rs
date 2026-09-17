//! Knowledge-limited underground planning. Coordinates are fine terrain cells.
use crate::{
    geometry::{MAX_ROWS, PIT_ROWS, WIDTH},
    terrain::Terrain,
};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
pub type Point = [u32; 2];
fn key(p: Point) -> u32 {
    p[1] * WIDTH + p[0]
}
fn point(k: u32) -> Point {
    [k % WIDTH, k / WIDTH]
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
    pub passages: Vec<Passage>,
    pub signals: Vec<Signal>,
    pub surveyed: BTreeSet<u32>,
    pub exhausted: BTreeSet<u32>,
    pub section: Option<Section>,
    pub search: Option<Search>,
    pub active: usize,
    pub revision: u64,
    pub status: String,
    pub survey_work: u64,
}
impl Workings {
    pub fn initialise(&mut self) {
        if self.passages.is_empty() {
            self.passages.push(Passage {
                feet: [WIDTH / 2, PIT_ROWS - 1],
                parent: 0,
                lift: false,
                supported: true,
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
        let radius = if upgraded || directed { 96 } else { 48 };
        let mut found = terrain.reveal(seed, profile, at[0], at[1], 8, cat);
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
                for oy in [4, 12, 20, 28] {
                    for ox in [4, 12, 20, 28] {
                        let id =
                            crate::geology::sample(seed, profile, bx * 32 + ox, by * 32 + oy, cat);
                        if id > 1 {
                            hits += 1;
                        }
                    }
                }
                if hits >= 2 {
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
        self.signals
            .retain(|s| !self.exhausted.contains(&key(s.centre)));
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
        self.passages.iter().enumerate().any(|(i, n)| {
            i > 0
                && n.lift
                && ((n.feet == a && self.passages[n.parent].feet == b)
                    || (n.feet == b && self.passages[n.parent].feet == a))
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
        if let Some(s) = &mut self.section {
            if s.cells.iter().any(|p| !terrain.contains(p[0], p[1])) {
                self.status = "Opening access / extracting vein".into();
                return;
            }
            self.status = "Waiting for supports".into();
            s.support_work += 80 * engineers.max(1) as u64 * (1 + supports) as u64;
            if s.support_work < 1000 {
                return;
            }
            let s = self.section.take().unwrap();
            self.passages.push(Passage {
                feet: s.to,
                parent: s.from,
                lift: s.lift,
                supported: true,
            });
            self.active = self.passages.len() - 1;
            self.revision += 1;
        }
        if self.search.is_none() {
            let deepest = self.passages.iter().map(|n| n.feet[1]).max().unwrap();
            let mut candidates: Vec<(i64, Point)> = vec![];
            // Targets come only from locally sampled frontier cells.
            for (&id, faces) in &terrain.ore_frontiers {
                if id <= 1 {
                    continue;
                }
                for &k in faces {
                    let p = point(k);
                    if p[1] < PIT_ROWS
                        || p[1] >= depth_limit
                        || self.exhausted.contains(&k)
                        || p[1] + 256 < deepest
                    {
                        continue;
                    }
                    let near = self
                        .passages
                        .iter()
                        .rev()
                        .take(128)
                        .map(|n| distance(n.feet, p))
                        .min()
                        .unwrap_or(u32::MAX);
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
                    || s.centre[1] + 256 < deepest
                {
                    continue;
                }
                let near = self
                    .passages
                    .iter()
                    .rev()
                    .take(128)
                    .map(|n| distance(n.feet, s.centre))
                    .min()
                    .unwrap_or(u32::MAX);
                if near <= 256 {
                    candidates.push((near as i64 - 12 * s.confidence as i64, s.centre));
                }
            }
            if policy == "depth" {
                for (score, p) in &mut candidates {
                    *score -= (p[1].saturating_sub(deepest)) as i64;
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
            if goal[1] < PIT_ROWS || depth_limit <= deepest + 1 && candidates.is_empty() {
                self.status = "Depth equipment required".into();
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
        for _ in 0..128 {
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
            for (dx, dy, lift) in [
                (-4, -1, false),
                (-4, 0, false),
                (-4, 1, false),
                (4, -1, false),
                (4, 0, false),
                (4, 1, false),
                (0, 4, true),
                (0, -4, true),
            ] {
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
                if distance(q, search.origin) > 256 {
                    continue;
                }
                let cells = cut_cells(p, q, lift);
                // Never cut an existing passage floor away; junctions are anchored at nodes.
                if self.passages.iter().rev().take(256).any(|n| {
                    !n.lift
                        && !(lift && n.feet[0]==p[0])
                        && n.feet != p
                        && n.feet != q
                        && cells.contains(&[n.feet[0], n.feet[1] + 1])
                }) {
                    continue;
                }
                let solid = cells
                    .iter()
                    .filter(|c| !terrain.contains(c[0], c[1]))
                    .count() as u64;
                let step = solid + if lift { 52 } else { 8 } + distance(p, q) as u64;
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
            if search.expanded >= 4096 {
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
                if lift != mode || delta != heading || distance(start, p) > 16 {
                    break;
                }
                end = p;
            }
            if end == start {
                self.exhausted.insert(key(search.goal));
                self.search = None;
                return;
            }
            let mut cells = cut_cells(start, end, mode);
            // Widen only sampled ore above the passage. Keep floors and 2m pillars between chambers.
            if !mode && policy != "depth" {
                for x in start[0].min(end[0])..=start[0].max(end[0]) {
                    let floor = interpolate(start, end, x);
                    for y in floor.saturating_sub(15)..floor.saturating_sub(7) {
                        if terrain.known_material(x, y).is_some_and(|id| {
                            id > 1 && (priorities.is_empty() || priorities.contains(&id))
                        }) && self.passages.iter().all(|n| distance(n.feet, [x, y]) >= 8)
                        {
                            for cy in y..floor.saturating_sub(7) {
                                cells.push([x, cy]);
                            }
                        }
                    }
                }
            }
            if distance(end, search.goal) < 24 {
                self.exhausted.insert(key(search.goal));
            }
            cells.sort_unstable();
            cells.dedup();
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
        } else if search.open.is_empty() || search.expanded >= 4096 {
            self.exhausted.insert(key(search.goal));
            self.search = None;
        }
    }
    pub fn valid(&self) -> bool {
        self.passages.len() <= 200_000
            && self.signals.len() <= 100_000
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
fn interpolate(a: Point, b: Point, x: u32) -> u32 {
    if a[0] == b[0] {
        return b[1];
    }
    (a[1] as i64
        + (b[1] as i64 - a[1] as i64) * (x as i64 - a[0] as i64) / (b[0] as i64 - a[0] as i64))
        as u32
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
    #[test]
    fn planning_is_knowledge_limited_and_resumes_identically() {
        let mut a = Workings::default();
        a.initialise();
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
    fn sections_respect_clearance_and_gradient() {
        let cells = cut_cells([200, 200], [216, 204], false);
        for x in 200..=216 {
            assert_eq!(cells.iter().filter(|c| c[0] == x).count(), 8);
        }
        assert!(cells.contains(&[216, 204]));
        assert!(!cells.contains(&[216, 205]));
    }
}
