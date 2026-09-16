//! Bounded presentation profile. Economic contents remain in the game ledgers.
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
pub struct WasteProfile {
    pub heights: Vec<u64>, // 1/1024 display pixel
    pub pitch: u32,
    pub origin: i32,
    pub discharge: usize,
}
impl Default for WasteProfile {
    fn default() -> Self {
        Self {
            heights: vec![0; 128],
            pitch: 2,
            origin: -128,
            discharge: 64,
        }
    }
}
impl WasteProfile {
    fn slope(&self) -> u64 {
        self.pitch as u64 * 691
    } // tan(34 degrees), fixed point
    pub fn reconcile(&mut self, contents: u64) {
        // Solve a slope-limited equilibrium surface directly. This avoids iterating
        // thousands of particle avalanches after a long offline interval.
        for _ in 0..24 {
            let desired = ((contents as u128 * 12288)
                / (crate::geometry::UNITS as u128 * self.pitch as u128))
                as u64;
            let centre = self.heights.len() / 2;
            let slope = self.slope();
            let mut distance = vec![0u64; self.heights.len()];
            for i in centre + 1..distance.len() {
                distance[i] = distance[i - 1] + slope * (80 + (i as u64 * 17 % 21)) / 100;
            }
            for i in (0..centre).rev() {
                distance[i] = distance[i + 1] + slope * (80 + (i as u64 * 31 % 21)) / 100;
            }
            // Secondary discharge lobe produces an irregular shoulder rather than
            // a fixed triangular silhouette. Both slopes obey the same repose limit.
            let secondary = centre + 12;
            for (i, cost) in distance.iter_mut().enumerate() {
                *cost = (*cost).min(i.abs_diff(secondary) as u64 * slope * 9 / 10 + 8 * slope);
            }
            let (mut low, mut high) = (0, desired + slope * self.heights.len() as u64 + 1);
            while low + 1 < high {
                let middle = low + (high - low) / 2;
                let total: u128 = distance
                    .iter()
                    .map(|d| middle.saturating_sub(*d) as u128)
                    .sum();
                if total <= desired as u128 {
                    low = middle;
                } else {
                    high = middle;
                }
            }
            self.heights = distance.iter().map(|d| low.saturating_sub(*d)).collect();
            let mut remainder = desired - self.heights.iter().sum::<u64>();
            for (index, height) in self.heights.iter_mut().enumerate() {
                if remainder > 0 && distance[index] <= low {
                    *height += 1;
                    remainder -= 1;
                }
            }
            if self.heights[0] <= slope && *self.heights.last().unwrap() <= slope {
                break;
            }
            if self.heights.len() < 512 {
                self.heights.splice(0..0, std::iter::repeat_n(0, 32));
                self.heights.extend(std::iter::repeat_n(0, 32));
                self.origin -= 32 * self.pitch as i32;
            } else {
                self.heights = self
                    .heights
                    .chunks_exact(2)
                    .map(|pair| (pair[0] + pair[1]) / 2)
                    .collect();
                self.pitch *= 2;
            }
        }
        self.discharge = self.heights.len() / 2;
    }
    pub fn valid(&self) -> bool {
        (128..=512).contains(&self.heights.len())
            && self.heights.len() % 2 == 0
            && self.pitch >= 2
            && self.pitch <= 1_048_576
            && self.pitch.is_power_of_two()
            && self.discharge < self.heights.len()
            && self.heights.iter().all(|h| *h <= 1_000_000_000_000)
            && (self.origin as i64).abs() <= 1_000_000_000
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pile_spreads_and_removal_resettles_without_cliffs() {
        let mut pile = WasteProfile::default();
        for units in [10, 1000, 100000, 1000000, 500, 0] {
            pile.reconcile(units * crate::geometry::UNITS);
            assert!(pile.valid());
            assert!(pile
                .heights
                .windows(2)
                .all(|p| p[0].abs_diff(p[1]) <= pile.slope() + 1));
            assert!(pile.heights[0] <= pile.slope() + 1);
            assert!(*pile.heights.last().unwrap() <= pile.slope() + 1);
            let area = pile.heights.iter().sum::<u64>() * pile.pitch as u64;
            assert!(area.abs_diff(units * 12288) <= 512 * pile.pitch as u64);
        }
        assert!(pile.heights.iter().all(|h| *h == 0));
    }
}
