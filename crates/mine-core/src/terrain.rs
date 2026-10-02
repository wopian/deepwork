//! Sparse excavation masks. Each chunk covers 64 × 64 cells; solid geology is seeded.
use crate::geometry::{
    bit_index, cell_key, cell_point, chunk_id, chunk_origin, valid_cell, valid_chunk,
};
pub use crate::geometry::{MAX_ROWS, WIDTH};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::{collections::HashMap, sync::Arc};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Terrain {
    /// One bit per excavated cell, encoded as bytes to stay exact across JSON/JS.
    pub chunks: BTreeMap<i64, Vec<u8>>,
    pub revision: u64,
    #[serde(default)]
    pub revealed: BTreeMap<i64, Vec<u8>>,
    /// 255 means unknown. This is the only mineral data sent to the renderer.
    #[serde(default)]
    pub visible: BTreeMap<i64, Vec<u8>>,
    #[serde(skip)]
    pub frontier: BTreeSet<i64>,
    #[serde(skip)]
    pub access_frontier: BTreeSet<i64>,
    #[serde(skip)]
    pub ore_frontiers: BTreeMap<usize, BTreeSet<i64>>,
    /// Cleared worker-height connectivity, shared by snapshots and rebuilt on load.
    #[serde(skip)]
    worker_paths: Arc<HashMap<i64, i64>>,
}
impl Default for Terrain {
    fn default() -> Self {
        Self {
            chunks: BTreeMap::new(),
            revision: 0,
            revealed: BTreeMap::new(),
            visible: BTreeMap::new(),
            frontier: (0..WIDTH).map(|x| cell_key(x, 0)).collect(),
            access_frontier: (0..WIDTH)
                .map(|x| cell_key(x, 0))
                .filter(|&key| Self::access_cell(key))
                .collect(),
            ore_frontiers: BTreeMap::new(),
            worker_paths: Arc::new(HashMap::from([(
                cell_key(WIDTH / 2, 0),
                cell_key(WIDTH / 2, 0),
            )])),
        }
    }
}
impl Terrain {
    fn worker_neighbors(point: [i64; 2]) -> impl Iterator<Item = [i64; 2]> {
        (-1..=1)
            .flat_map(move |dx| {
                (-1..=1).filter_map(move |dy| {
                    (dx != 0 || dy != 0).then_some([point[0] + dx, point[1] + dy])
                })
            })
            .filter(|p| valid_cell(p[0], p[1]))
    }
    fn extend_worker_paths(&mut self, seeds: Vec<[i64; 2]>) {
        let mut queue = VecDeque::new();
        for point in seeds {
            let key = cell_key(point[0], point[1]);
            if self.worker_paths.contains_key(&key)
                || !self.column_clear(point[0], point[1].saturating_sub(7).max(0), point[1])
            {
                continue;
            }
            if let Some(parent) = Self::worker_neighbors(point)
                .map(|p| cell_key(p[0], p[1]))
                .find(|key| self.worker_paths.contains_key(key))
            {
                Arc::make_mut(&mut self.worker_paths).insert(key, parent);
                queue.push_back(point);
            }
        }
        while let Some(point) = queue.pop_front() {
            for next in Self::worker_neighbors(point) {
                let key = cell_key(next[0], next[1]);
                if self.worker_paths.contains_key(&key)
                    || !self.column_clear(next[0], next[1].saturating_sub(7).max(0), next[1])
                {
                    continue;
                }
                Arc::make_mut(&mut self.worker_paths).insert(key, cell_key(point[0], point[1]));
                queue.push_back(next);
            }
        }
    }
    pub fn worker_reachable(&self, point: [i64; 2]) -> bool {
        self.worker_paths
            .contains_key(&cell_key(point[0], point[1]))
    }
    pub fn worker_path(&self, from: [i64; 2], to: [i64; 2]) -> Option<Vec<[i64; 2]>> {
        let mut at = cell_key(from[0], from[1]);
        let mut left = Vec::new();
        let mut ancestors = HashMap::new();
        loop {
            ancestors.insert(at, left.len());
            left.push(cell_point(at));
            let parent = *self.worker_paths.get(&at)?;
            if parent == at {
                break;
            }
            at = parent;
        }
        let mut right = Vec::new();
        at = cell_key(to[0], to[1]);
        while !ancestors.contains_key(&at) {
            right.push(cell_point(at));
            at = *self.worker_paths.get(&at)?;
        }
        left.truncate(ancestors[&at] + 1);
        right.reverse();
        left.extend(right);
        Some(left)
    }
    fn access_cell(key: i64) -> bool {
        let (x, y) = {
            let [x, y] = cell_point(key);
            (x, y)
        };
        if y < crate::geometry::PIT_ROWS {
            x >= 16 + y && x < WIDTH - (16 + (y / 24) * 24)
        } else {
            false
        }
    }
    fn index_frontier(&mut self, key: i64) {
        if !self.frontier.contains(&key) {
            return;
        }
        if Self::access_cell(key) {
            self.access_frontier.insert(key);
        } else {
            let (x, y) = {
                let [x, y] = cell_point(key);
                (x, y)
            };
            if y >= crate::geometry::PIT_ROWS {
                if let Some(id) = self.known_material(x, y) {
                    self.ore_frontiers.entry(id).or_default().insert(key);
                }
            }
        }
    }
    fn rebuild_work_index(&mut self) {
        self.access_frontier.clear();
        self.ore_frontiers.clear();
        for key in self.frontier.iter().copied().collect::<Vec<_>>() {
            self.index_frontier(key);
        }
    }
    /// Count an inclusive vertical cut with one map lookup per intersected chunk.
    pub(crate) fn excavated_in_column(&self, x: i64, first: i64, last: i64) -> i64 {
        if !valid_cell(x, first) || first > last {
            return 0;
        }
        let last = last.min(MAX_ROWS - 1);
        let mut y = first;
        let mut count = 0;
        while y <= last {
            let end = last.min((y / 64 + 1) * 64 - 1);
            if let Some(bytes) = self.chunks.get(&chunk_id(x, y)) {
                for row in y..=end {
                    let bit = bit_index(x, row);
                    count += i64::from(bytes[bit / 8] & (1 << (bit % 8)) != 0);
                }
            }
            y = end + 1;
        }
        count
    }
    pub fn contains(&self, x: i64, y: i64) -> bool {
        if !valid_cell(x, y) {
            return false;
        }
        let index = bit_index(x, y);
        self.chunks
            .get(&chunk_id(x, y))
            .is_some_and(|bytes| bytes[index / 8] & (1 << (index % 8)) != 0)
    }
    /// Check lift clearance with one map lookup per chunk, not per vertical cell.
    pub fn column_clear(&self, x: i64, first: i64, last: i64) -> bool {
        if !valid_cell(x, first) || first > last || last >= MAX_ROWS {
            return false;
        }
        let byte_x = (x.rem_euclid(64) / 8) as usize;
        let mask = 1 << (x.rem_euclid(8));
        for chunk_y in first / 64..=last / 64 {
            let Some(bytes) = self.chunks.get(&chunk_id(x, chunk_y * 64)) else {
                return false;
            };
            let start = first.saturating_sub(chunk_y * 64).max(0);
            let end = (last - chunk_y * 64).min(63);
            if (start..=end).any(|row| bytes[row as usize * 8 + byte_x] & mask == 0) {
                return false;
            }
        }
        true
    }
    pub fn known_material(&self, x: i64, y: i64) -> Option<usize> {
        self.visible
            .get(&chunk_id(x, y))
            .and_then(|pixels| pixels.get(bit_index(x, y)))
            .filter(|&&id| id != 255)
            .map(|&id| id as usize)
    }
    pub fn is_revealed(&self, x: i64, y: i64) -> bool {
        let index = bit_index(x, y);
        self.revealed
            .get(&chunk_id(x, y))
            .is_some_and(|mask| mask[index / 8] & (1 << (index % 8)) != 0)
    }
    pub fn reveal(
        &mut self,
        seed: u64,
        profile: usize,
        x: i64,
        y: i64,
        radius: i64,
        cat: &[crate::Material],
    ) -> Vec<usize> {
        self.reveal_versioned(seed, profile, 5, x, y, radius, cat)
    }
    pub fn reveal_versioned(
        &mut self,
        seed: u64,
        profile: usize,
        version: u32,
        x: i64,
        y: i64,
        radius: i64,
        cat: &[crate::Material],
    ) -> Vec<usize> {
        let mut found = std::collections::BTreeSet::new();
        for py in y.saturating_sub(radius).max(0)..=y.saturating_add(radius).min(MAX_ROWS - 1) {
            for px in x.saturating_sub(radius)..=x.saturating_add(radius) {
                if !valid_cell(px, py)
                    || px.abs_diff(x).pow(2) + py.abs_diff(y).pow(2) > radius.pow(2) as u64
                    || self.is_revealed(px, py)
                {
                    continue;
                }
                let id = chunk_id(px, py);
                let index = bit_index(px, py);
                self.revealed.entry(id).or_insert_with(|| vec![0; 512])[index / 8] |=
                    1 << (index % 8);
                let material =
                    crate::geology::sample_versioned(seed, profile, version, px, py, cat);
                self.visible.entry(id).or_insert_with(|| vec![255; 4096])[index] = material as u8;
                self.index_frontier(cell_key(px, py));
                found.insert(material);
                self.revision += 1;
            }
        }
        found.into_iter().collect()
    }
    pub fn neighbors(x: i64, y: i64) -> impl Iterator<Item = (i64, i64)> {
        [
            (x.checked_sub(1), Some(y)),
            (x.checked_add(1), Some(y)),
            (Some(x), y.checked_sub(1).filter(|v| *v >= 0)),
            (Some(x), y.checked_add(1).filter(|v| *v < MAX_ROWS)),
        ]
        .into_iter()
        .filter_map(|(x, y)| x.zip(y))
        .filter(|(x, y)| valid_cell(*x, *y))
    }
    pub fn excavate(&mut self, x: i64, y: i64) -> bool {
        let key = cell_key(x, y);
        if !valid_cell(x, y) || !self.frontier.remove(&key) {
            return false;
        }
        self.access_frontier.remove(&key);
        if let Some(id) = self.known_material(x, y) {
            if let Some(frontier) = self.ore_frontiers.get_mut(&id) {
                frontier.remove(&key);
            }
        }
        let index = bit_index(x, y);
        let bytes = self
            .chunks
            .entry(chunk_id(x, y))
            .or_insert_with(|| vec![0; 512]);
        bytes[index / 8] |= 1 << (index % 8);
        for (nx, ny) in Self::neighbors(x, y) {
            if !self.contains(nx, ny) {
                self.frontier.insert(cell_key(nx, ny));
                self.index_frontier(cell_key(nx, ny));
            }
        }
        self.revision += 1;
        self.extend_worker_paths(
            (y..=(y + 7).min(MAX_ROWS - 1))
                .map(|feet| [x, feet])
                .collect(),
        );
        true
    }
    pub fn rebuild(&mut self) -> Result<(), String> {
        if self
            .chunks
            .iter()
            .any(|(id, v)| !valid_chunk(*id) || v.len() != 512)
        {
            return Err("Invalid terrain chunk".into());
        }
        self.frontier = (0..WIDTH)
            .filter(|x| !self.contains(*x, 0))
            .map(|x| cell_key(x, 0))
            .collect();
        let mut opened = Vec::new();
        for (&chunk, bytes) in &self.chunks {
            for (i, &byte) in bytes.iter().enumerate() {
                for bit in 0..8 {
                    if byte & (1 << bit) != 0 {
                        let index = i * 8 + bit;
                        let (cx, cy) = chunk_origin(chunk);
                        let x = cx + (index % 64) as i64;
                        let y = cy + (index / 64) as i64;
                        if y >= MAX_ROWS {
                            return Err("Terrain exceeds supported depth".into());
                        }
                        opened.push((x, y));
                    }
                }
            }
        }
        let mut routes: BTreeMap<_, _> = opened
            .iter()
            .map(|(x, y)| (cell_key(*x, *y), (i64::MAX, cell_key(*x, *y))))
            .collect();
        let starts: VecDeque<_> = opened
            .iter()
            .filter(|(_, y)| *y == 0)
            .map(|(x, _)| cell_key(*x, 0))
            .collect();
        for key in &starts {
            routes.insert(*key, (0, *key));
        }
        Self::relax_routes(&mut routes, starts);
        if routes.values().any(|(distance, _)| *distance == i64::MAX) {
            return Err("Excavation disconnected from surface".into());
        }
        for (x, y) in opened {
            for (nx, ny) in Self::neighbors(x, y) {
                if !self.contains(nx, ny) {
                    self.frontier.insert(cell_key(nx, ny));
                    self.index_frontier(cell_key(nx, ny));
                }
            }
        }
        self.rebuild_work_index();
        let entrance = cell_key(WIDTH / 2, 0);
        self.worker_paths = Arc::new(HashMap::from([(entrance, entrance)]));
        self.extend_worker_paths(Self::worker_neighbors([WIDTH / 2, 0]).collect());
        Ok(())
    }
    pub fn from_columns(heights: &[i64]) -> Self {
        Self::from_heights(
            &heights
                .iter()
                .enumerate()
                .map(|(x, h)| (x as i64, *h))
                .collect(),
        )
    }
    pub fn from_heights(heights: &BTreeMap<i64, i64>) -> Self {
        let mut t = Self::default();
        for (&x, &height) in heights.iter() {
            for y in 0..height {
                t.excavate(x as i64, y);
            }
        }
        t
    }
    fn relax_routes(routes: &mut BTreeMap<i64, (i64, i64)>, mut queue: VecDeque<i64>) {
        while let Some(key) = queue.pop_front() {
            let distance = routes[&key].0;
            for (x, y) in Self::neighbors(cell_point(key)[0], cell_point(key)[1]) {
                let next = cell_key(x, y);
                if let Some(route) = routes.get_mut(&next) {
                    if route.0 > distance + 1 {
                        *route = (distance + 1, key);
                        queue.push_back(next);
                    } else if route.0 == distance + 1 && key < route.1 {
                        route.1 = key;
                    }
                }
            }
        }
    }
    /// Connectivity diagnostic; gameplay uses the coarse floor/lift portal route.
    /// No per-cell route table is retained during foreground excavation.
    pub fn surface_route(&self, start: [i64; 2]) -> Vec<[i64; 2]> {
        if !self.contains(start[0], start[1]) {
            return vec![];
        }
        let origin = cell_key(start[0], start[1]);
        let mut parents = BTreeMap::from([(origin, origin)]);
        let mut queue = VecDeque::from([origin]);
        let mut exit = None;
        while let Some(key) = queue.pop_front() {
            if cell_point(key)[1] == 0 {
                exit = Some(key);
                break;
            }
            for (x, y) in Self::neighbors(cell_point(key)[0], cell_point(key)[1]) {
                let next = cell_key(x, y);
                if self.contains(x, y) && !parents.contains_key(&next) {
                    parents.insert(next, key);
                    queue.push_back(next);
                }
            }
        }
        let Some(mut key) = exit else {
            return vec![];
        };
        let mut path = vec![cell_point(key)];
        while key != origin {
            key = parents[&key];
            path.push(cell_point(key));
        }
        path.reverse();
        let mut turns = vec![start];
        let mut direction = None;
        for pair in path.windows(2) {
            let d = (
                pair[1][0] as i64 - pair[0][0] as i64,
                pair[1][1] as i64 - pair[0][1] as i64,
            );
            if direction.is_some_and(|old| old != d) {
                turns.push(pair[0]);
            }
            direction = Some(d);
        }
        if path.last() != Some(&start) {
            turns.push(*path.last().unwrap());
        }
        turns
    }
    pub fn count(&self) -> u64 {
        self.chunks
            .values()
            .flat_map(|v| v.iter())
            .map(|b| b.count_ones() as u64)
            .sum()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn excavation_crosses_both_surface_edges_and_restores_signed_frontiers() {
        let mut t = Terrain::default();
        for x in (0..=256).rev() {
            assert!(t.excavate(x, 0));
        }
        for x in (-130..0).rev() {
            assert!(t.excavate(x, 0));
        }
        for x in 257..650 {
            assert!(t.excavate(x, 0));
        }
        for y in 1..90 {
            assert!(t.excavate(-65, y));
            assert!(t.excavate(600, y));
        }
        let mass = t.count();
        let frontier = t.frontier.clone();
        t.reveal(42, 0, -65, 64, 8, &crate::materials());
        assert!(t.is_revealed(-65, 64));
        assert!(!t.is_revealed(65, 64));
        let mut copy: Terrain = serde_json::from_str(&serde_json::to_string(&t).unwrap()).unwrap();
        copy.rebuild().unwrap();
        assert_eq!(copy.frontier, frontier);
        assert_eq!(copy.count(), mass);
        assert_eq!(copy.surface_route([-65, 89]).last().unwrap()[1], 0);
        assert!(copy.column_clear(-65, 0, 89));
    }
    #[test]
    fn new_daylight_connection_shortens_existing_routes() {
        let mut t = Terrain::default();
        for y in 0..6 {
            t.excavate(30, y);
        }
        for x in 31..36 {
            t.excavate(x, 5);
        }
        assert_eq!(t.surface_route([34, 5]).last(), Some(&[30, 0]));
        for y in (0..5).rev() {
            t.excavate(35, y);
        }
        assert_eq!(t.surface_route([34, 5]).last(), Some(&[35, 0]));
        let mut restored: Terrain =
            serde_json::from_str(&serde_json::to_string(&t).unwrap()).unwrap();
        restored.rebuild().unwrap();
        assert_eq!(restored.surface_route([34, 5]).last(), Some(&[35, 0]));
        for y in 0..6 {
            for x in 30..36 {
                assert_eq!(t.surface_route([x, y]), restored.surface_route([x, y]));
            }
        }
    }
    #[test]
    fn route_turns_around_solid_rock() {
        let mut t = Terrain::default();
        for y in 0..5 {
            assert!(t.excavate(30, y));
        }
        for x in 31..35 {
            assert!(t.excavate(x, 4));
        }
        assert_eq!(t.surface_route([34, 4]), vec![[34, 4], [30, 4], [30, 0]]);
        assert!(t.surface_route([34, 3]).is_empty());
    }
    #[test]
    fn only_reachable_faces_can_be_removed() {
        let mut t = Terrain::default();
        assert!(!t.excavate(30, 20));
        assert!(t.excavate(30, 0));
        assert!(t.excavate(30, 1));
        assert!(t.excavate(31, 1));
        assert!(!t.excavate(31, 1));
        assert!(!t.contains(31, 0));
    }
    #[test]
    fn mask_crosses_chunk_boundary_and_restores_frontier() {
        let mut t = Terrain::default();
        for y in 0..130 {
            assert!(t.excavate(32, y));
        }
        assert_eq!(t.chunks.len(), 3);
        let mut restored: Terrain =
            serde_json::from_str(&serde_json::to_string(&t).unwrap()).unwrap();
        restored.rebuild().unwrap();
        assert_eq!(restored.frontier, t.frontier);
        assert_eq!(restored.access_frontier, t.access_frontier);
        assert_eq!(restored.ore_frontiers, t.ore_frontiers);
        assert_eq!(restored.count(), 130);
    }
    #[test]
    fn migration_preserves_column_voids() {
        let heights: Vec<_> = (0..64).map(|x| x % 12).collect();
        let t = Terrain::from_columns(&heights);
        assert_eq!(t.count(), heights.iter().map(|h| *h as u64).sum::<u64>());
        for (x, h) in heights.iter().enumerate() {
            assert!(!t.contains(x as i64, *h));
        }
    }
}

#[cfg(test)]
mod reveal_tests {
    use super::*;
    #[test]
    fn reveal_crosses_chunks_and_does_not_leak_hidden_materials() {
        let mut t = Terrain::default();
        assert!(t.visible.is_empty());
        t.reveal(42, 0, 64, 64, 4, &crate::materials());
        assert!(t.revealed.len() >= 3);
        assert!(t.is_revealed(63, 64) && t.is_revealed(65, 64));
        assert!(!t.is_revealed(69, 64));
        for (&id, pixels) in &t.visible {
            let (cx, cy) = chunk_origin(id);
            for (index, &material) in pixels.iter().enumerate() {
                assert_eq!(
                    material != 255,
                    t.is_revealed(cx + index as i64 % 64, cy + index as i64 / 64)
                );
            }
        }
        let encoded = serde_json::to_string(&t).unwrap();
        let restored: Terrain = serde_json::from_str(&encoded).unwrap();
        assert_eq!(restored.visible, t.visible);
    }
}

#[cfg(test)]
mod clearance_tests {
    use super::*;
    #[test]
    fn column_clear_matches_cell_checks_across_chunk_edges_and_gaps() {
        let mut t = Terrain::from_columns(&vec![201; WIDTH as usize]);
        for x in [0, 63, 64, 255, 256, 511] {
            for missing in [0, 63, 64, 127, 128, 200] {
                let index = bit_index(x, missing);
                t.chunks.get_mut(&chunk_id(x, missing)).unwrap()[index / 8] &= !(1 << (index % 8));
                for first in [0, 63, 64, 127, 128, 200] {
                    for last in first..=200 {
                        assert_eq!(
                            t.column_clear(x, first, last),
                            (first..=last).all(|y| t.contains(x, y))
                        );
                    }
                }
                t.chunks.get_mut(&chunk_id(x, missing)).unwrap()[index / 8] |= 1 << (index % 8);
            }
        }
        assert!(!t.column_clear(WIDTH, 0, 1));
        assert!(!t.column_clear(0, 2, 1));
        assert!(!t.column_clear(0, 0, MAX_ROWS));
    }
}
