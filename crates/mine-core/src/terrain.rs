//! Sparse excavation masks. Each chunk covers 64 × 64 cells; solid geology is seeded.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;
pub const WIDTH: u32 = 64;
pub const MAX_ROWS: u32 = 100_000;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Terrain {
    /// One bit per excavated cell, encoded as bytes to stay exact across JSON/JS.
    pub chunks: BTreeMap<u32, Vec<u8>>,
    pub revision: u64,
    #[serde(skip)]
    pub frontier: BTreeSet<u32>,
    /// (distance, parent); snapshots share this immutable cache without copying it.
    #[serde(skip)]
    routes: Arc<BTreeMap<u32, (u32, u32)>>,
}
impl Default for Terrain {
    fn default() -> Self {
        Self {
            chunks: BTreeMap::new(),
            revision: 0,
            frontier: (0..WIDTH).collect(),
            routes: Arc::default(),
        }
    }
}
impl Terrain {
    pub fn contains(&self, x: u32, y: u32) -> bool {
        if x >= WIDTH || y >= MAX_ROWS {
            return false;
        }
        let index = ((y % 64) * 64 + x) as usize;
        self.chunks
            .get(&(y / 64))
            .is_some_and(|bytes| bytes[index / 8] & (1 << (index % 8)) != 0)
    }
    pub fn neighbors(x: u32, y: u32) -> impl Iterator<Item = (u32, u32)> {
        [
            (x.checked_sub(1), Some(y)),
            (x.checked_add(1).filter(|v| *v < WIDTH), Some(y)),
            (Some(x), y.checked_sub(1)),
            (Some(x), y.checked_add(1).filter(|v| *v < MAX_ROWS)),
        ]
        .into_iter()
        .filter_map(|(x, y)| x.zip(y))
    }
    pub fn excavate(&mut self, x: u32, y: u32) -> bool {
        let key = y.saturating_mul(WIDTH).saturating_add(x);
        if x >= WIDTH || y >= MAX_ROWS || !self.frontier.remove(&key) {
            return false;
        }
        let index = ((y % 64) * 64 + x) as usize;
        let bytes = self.chunks.entry(y / 64).or_insert_with(|| vec![0; 512]);
        bytes[index / 8] |= 1 << (index % 8);
        for (nx, ny) in Self::neighbors(x, y) {
            if !self.contains(nx, ny) {
                self.frontier.insert(ny * WIDTH + nx);
            }
        }
        let routes = Arc::make_mut(&mut self.routes);
        let (distance, parent) = if y == 0 {
            (0, key)
        } else {
            Self::neighbors(x, y)
                .filter_map(|(nx, ny)| {
                    let next = ny * WIDTH + nx;
                    routes.get(&next).map(|(distance, _)| (distance + 1, next))
                })
                .min()
                .expect("reachable cell has a route")
        };
        routes.insert(key, (distance, parent));
        Self::relax_routes(routes, VecDeque::from([key]));
        self.revision += 1;
        true
    }
    pub fn rebuild(&mut self) -> Result<(), String> {
        if self
            .chunks
            .iter()
            .any(|(id, v)| *id >= (MAX_ROWS + 63) / 64 || v.len() != 512)
        {
            return Err("Invalid terrain chunk".into());
        }
        self.frontier = (0..WIDTH).filter(|x| !self.contains(*x, 0)).collect();
        let mut opened = Vec::new();
        for (&chunk, bytes) in &self.chunks {
            for (i, &byte) in bytes.iter().enumerate() {
                for bit in 0..8 {
                    if byte & (1 << bit) != 0 {
                        let index = i * 8 + bit;
                        let x = (index % 64) as u32;
                        let y = chunk * 64 + (index / 64) as u32;
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
            .map(|(x, y)| (y * WIDTH + x, (u32::MAX, y * WIDTH + x)))
            .collect();
        let starts: VecDeque<_> = opened
            .iter()
            .filter(|(_, y)| *y == 0)
            .map(|(x, _)| *x)
            .collect();
        for key in &starts {
            routes.insert(*key, (0, *key));
        }
        Self::relax_routes(&mut routes, starts);
        if routes.values().any(|(distance, _)| *distance == u32::MAX) {
            return Err("Excavation disconnected from surface".into());
        }
        self.routes = Arc::new(routes);
        for (x, y) in opened {
            for (nx, ny) in Self::neighbors(x, y) {
                if !self.contains(nx, ny) {
                    self.frontier.insert(ny * WIDTH + nx);
                }
            }
        }
        Ok(())
    }
    pub fn from_columns(heights: &[u32]) -> Self {
        let mut t = Self::default();
        for (x, &height) in heights.iter().enumerate() {
            for y in 0..height {
                t.excavate(x as u32, y);
            }
        }
        t
    }
    fn relax_routes(routes: &mut BTreeMap<u32, (u32, u32)>, mut queue: VecDeque<u32>) {
        while let Some(key) = queue.pop_front() {
            let distance = routes[&key].0;
            for (x, y) in Self::neighbors(key % WIDTH, key / WIDTH) {
                let next = y * WIDTH + x;
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
    /// Shortest open-cell route to daylight, compressed at changes of direction.
    pub fn surface_route(&self, start: [u32; 2]) -> Vec<[u32; 2]> {
        let mut key = start[1].saturating_mul(WIDTH).saturating_add(start[0]);
        if start[0] >= WIDTH || !self.routes.contains_key(&key) {
            return vec![];
        }
        let mut turns = vec![start];
        let mut previous = start;
        let mut direction = None;
        while key >= WIDTH {
            key = self.routes[&key].1;
            let next = [key % WIDTH, key / WIDTH];
            let d = (
                next[0] as i64 - previous[0] as i64,
                next[1] as i64 - previous[1] as i64,
            );
            if direction.is_some_and(|old| old != d) {
                turns.push(previous);
            }
            direction = Some(d);
            previous = next;
        }
        if previous != start {
            turns.push(previous);
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
        assert_eq!(restored.count(), 130);
    }
    #[test]
    fn migration_preserves_column_voids() {
        let heights: Vec<_> = (0..64).map(|x| x % 12).collect();
        let t = Terrain::from_columns(&heights);
        assert_eq!(t.count(), heights.iter().map(|h| *h as u64).sum::<u64>());
        for (x, h) in heights.iter().enumerate() {
            assert!(!t.contains(x as u32, *h));
        }
    }
}
