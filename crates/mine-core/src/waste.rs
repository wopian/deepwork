//! Bounded presentation profile. Economic contents remain in the game ledgers.
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
pub struct WasteProfile {
    pub heights: Vec<u64>, // 1/1024 display pixel
    pub pitch: u32,
    pub origin: i32,
    pub discharge: usize,
    #[serde(default)]
    pub placed_area: u64,
}
impl Default for WasteProfile {
    fn default() -> Self {
        Self {
            heights: vec![0; 128],
            pitch: 2,
            origin: -128,
            discharge: 64,
            placed_area: 0,
        }
    }
}
impl WasteProfile {
    fn slope(&self) -> u64 {
        self.pitch as u64 * 691
    } // tan(34 degrees), fixed point
    pub fn reconcile(&mut self, contents: u64) {
        let area = (contents as u128 * 12288 / crate::geometry::UNITS as u128) as u64;
        let old_area = self.heights.iter().sum::<u64>() * self.pitch as u64;
        let adding = area >= old_area;
        if adding {
            self.placed_area = self
                .placed_area
                .saturating_add((area - old_area) / self.pitch as u64 * self.pitch as u64);
        }
        // Shift the delivery point between nearby pads as each stockpile section fills.
        // The existing profile remains the foundation for every later deposition.
        let pads = [0, 24, -16, 40, -32, 8, -40, 32];
        let pad = pads[(self.placed_area / (80 * 12288)) as usize % pads.len()];
        for _ in 0..24 {
            let desired = area / self.pitch as u64;
            let current = self.heights.iter().sum::<u64>();
            self.discharge = ((pad - self.origin).max(0) as usize / self.pitch as usize)
                .clamp(1, self.heights.len() - 2);
            if current == desired {
                return;
            }
            let slope = self.slope();
            let mut next;
            if desired > current {
                // Adding a slope-limited cone atop an already stable surface preserves
                // old shoulders. Binary search deposits exactly the requested volume.
                let distance: Vec<_> = (0..self.heights.len())
                    .map(|i| i.abs_diff(self.discharge) as u64 * slope)
                    .collect();
                let (mut low, mut high) = (
                    0,
                    desired.saturating_add(slope * self.heights.len() as u64 + 1),
                );
                while low + 1 < high {
                    let middle = low + (high - low) / 2;
                    let total: u128 = self
                        .heights
                        .iter()
                        .zip(&distance)
                        .map(|(&h, &d)| h.max(middle.saturating_sub(d)) as u128)
                        .sum();
                    if total <= desired as u128 {
                        low = middle;
                    } else {
                        high = middle;
                    }
                }
                next = self
                    .heights
                    .iter()
                    .zip(&distance)
                    .map(|(&h, &d)| h.max(low.saturating_sub(d)))
                    .collect::<Vec<_>>();
                let mut remainder = desired - next.iter().sum::<u64>();
                for ((height, &old), &d) in next.iter_mut().zip(&self.heights).zip(&distance) {
                    if remainder > 0 && d <= low && old <= low - d {
                        *height += 1;
                        remainder -= 1;
                    }
                }
                if next[0] > slope || *next.last().unwrap() > slope {
                    // Expand before committing the deposit, so overflow can settle on
                    // neighbouring ground rather than retaining an artificial wall.
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
                    continue;
                }
            } else {
                // Lowering all exposed surfaces preserves shoulders and stable slopes;
                // columns that reach ground disappear naturally at the pile margins.
                let (mut low, mut high) = (0, self.heights.iter().copied().max().unwrap_or(0) + 1);
                while low + 1 < high {
                    let middle = low + (high - low) / 2;
                    let total: u64 = self.heights.iter().map(|h| h.saturating_sub(middle)).sum();
                    if total >= desired {
                        low = middle;
                    } else {
                        high = middle;
                    }
                }
                next = self
                    .heights
                    .iter()
                    .map(|h| h.saturating_sub(low))
                    .collect::<Vec<_>>();
                let mut excess = next.iter().sum::<u64>() - desired;
                for height in &mut next {
                    if excess > 0 && *height > 0 {
                        *height -= 1;
                        excess -= 1;
                    }
                }
            }
            self.heights = next;
            return;
        }
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

#[cfg(test)]
mod history_tests {
    use super::*;
    #[test]
    fn delivery_history_survives_settling_save_and_reclamation() {
        let mut staged = WasteProfile::default();
        for units in (40..=1000).step_by(40) {
            staged.reconcile(units * crate::geometry::UNITS);
        }
        let mut single = WasteProfile::default();
        single.reconcile(1000 * crate::geometry::UNITS);
        assert_ne!(staged.heights, single.heights);
        let before = staged.heights.clone();
        staged.reconcile(1000 * crate::geometry::UNITS);
        assert_eq!(staged.heights, before);
        let mut loaded: WasteProfile =
            serde_json::from_str(&serde_json::to_string(&staged).unwrap()).unwrap();
        loaded.reconcile(500 * crate::geometry::UNITS);
        assert!(loaded.heights.iter().zip(&before).all(|(a, b)| a <= b));
        assert!(loaded
            .heights
            .windows(2)
            .all(|p| p[0].abs_diff(p[1]) <= loaded.slope() + 2));
        let area = loaded.heights.iter().sum::<u64>() * loaded.pitch as u64;
        assert!(area.abs_diff(500 * 12288) < loaded.pitch as u64);
    }
    #[test]
    fn repeated_delivery_and_cleanup_preserve_volume_and_repose() {
        let mut pile = WasteProfile::default();
        let mut random = 42u64;
        let mut contents = 0u64;
        for _ in 0..2000 {
            random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
            let amount = random % (100 * crate::geometry::UNITS);
            contents = if random & 8 == 0 {
                contents.saturating_sub(amount)
            } else {
                contents + amount
            };
            pile.reconcile(contents);
            assert!(pile.valid());
            assert!(pile
                .heights
                .windows(2)
                .all(|p| p[0].abs_diff(p[1]) <= pile.slope() + 2));
            let area = pile.heights.iter().sum::<u64>() * pile.pitch as u64;
            let expected = (contents as u128 * 12288 / crate::geometry::UNITS as u128) as u64;
            assert!(area.abs_diff(expected) < pile.pitch as u64);
            let old = pile.placed_area;
            pile.reconcile(contents);
            assert_eq!(pile.placed_area, old);
        }
    }
}
