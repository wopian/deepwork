//! Stateless world-space deposits. A sample does not depend on generated chunks.
use crate::{sites, Material};
fn hash(mut v: u64) -> u64 {
    v = (v ^ (v >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    v = (v ^ (v >> 27)).wrapping_mul(0x94d049bb133111eb);
    v ^ (v >> 31)
}
fn noise(seed: u64, x: i64, y: i64) -> f64 {
    (hash(seed ^ (x as u64).wrapping_mul(0x9e3779b97f4a7c15) ^ (y as u64).rotate_left(29)) % 10000)
        as f64
        / 10000.
}
fn tier(depth: i64) -> u32 {
    match depth {
        ..=99 => 0,
        100..=299 => 1,
        300..=699 => 2,
        700..=1499 => 3,
        1500..=2999 => 4,
        _ => 5,
    }
}
#[derive(Clone, Copy)]
struct Deposit {
    x: f64,
    y: f64,
    length: f64,
    width: f64,
    slope: f64,
    shape: u64,
    seed: u64,
}
impl Deposit {
    fn contains(&self, x: f64, y: f64) -> bool {
        let (mut u, mut v) = (x - self.x, y - self.y);
        if self.shape % 3 == 0 && self.seed & 8 != 0 {
            std::mem::swap(&mut u, &mut v);
        }
        let along = u / self.length;
        if along.abs() >= 1. {
            return false;
        }
        let ripple = (u / 39. + (self.seed % 19) as f64).sin();
        let centre = match self.shape % 3 {
            0 => self.slope * u + ripple * self.width * 1.3,
            1 => self.slope * u * 0.25,
            _ => (x / 93. + (self.seed % 31) as f64).sin() * 13.,
        };
        let taper = (1. - along * along).sqrt();
        let thickness = self.width * taper * (0.8 + 0.2 * ripple);
        let branch = self.shape % 3 == 0
            && u > 0.
            && u < self.length * 0.7
            && (v - centre - u * 0.22).abs() < thickness * 0.45;
        if (v - centre).abs() > thickness && !branch {
            return false;
        }
        // Fine host inclusions and broken margins do not align with chunk edges.
        let grain = noise(self.seed, x as i64, y as i64);
        grain > 0.08 && (grain > 0.25 || (v - centre).abs() < thickness * 0.7)
    }
}
pub fn sample(seed: u64, profile: usize, x: u32, y: u32, catalogue: &[Material]) -> usize {
    // Irregular, reachable starter reserves; no repeating vertical stripes.
    for (index, (id, cy, radius)) in [(3, 24., 35.), (5, 76., 43.), (6, 130., 48.)]
        .into_iter()
        .enumerate()
    {
        let starter = Deposit {
            x: 256. + (index as f64 - 1.) * 21.,
            y: cy,
            length: radius,
            width: 9.,
            slope: 0.08,
            shape: 1,
            seed: hash(seed + index as u64),
        };
        if starter.contains(x as f64, y as f64) {
            return id;
        }
    }
    let tx = x as i64 / 256;
    let ty = y as i64 / 256;
    // Maximum descriptor reach fits within neighbouring macro tiles, even diagonal branches.
    for gy in (ty - 2)..=(ty + 2) {
        for gx in (tx - 2)..=(tx + 2) {
            if gy < 0 {
                continue;
            }
            let key = hash(
                seed ^ (gx as u64).wrapping_mul(0x9e3779b97f4a7c15) ^ (gy as u64).rotate_left(32),
            );
            let cx = gx * 256 + (key % 256) as i64;
            let cy = gy * 256 + ((key >> 8) % 256) as i64;
            let candidates: Vec<_> = catalogue
                .iter()
                .filter(|m| m.id > 1 && m.tier <= tier(cy / 4))
                .collect();
            if candidates.is_empty() {
                continue;
            }
            let focus: Vec<_> = candidates
                .iter()
                .copied()
                .filter(|m| sites()[profile].focus.contains(&m.id))
                .collect();
            let pool = if key & 3 == 0 && !focus.is_empty() {
                &focus
            } else {
                &candidates
            };
            let material = pool[((key >> 16) as usize) % pool.len()];
            let shape = if material.name.contains("coal")
                || material.name.contains("salt")
                || material.name.contains("Gypsum")
            {
                2
            } else {
                (key >> 28) % 3
            };
            let deposit = Deposit {
                x: cx as f64,
                y: cy as f64,
                length: 48. + ((key >> 20) % 209) as f64,
                width: 3. + ((key >> 36) % 15) as f64,
                slope: ((key >> 40) % 7) as f64 / 8. - 0.375,
                shape,
                seed: key,
            };
            if deposit.contains(x as f64, y as f64) {
                return material.id;
            }
        }
    }
    if y < 32 {
        0
    } else {
        1
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deposits_cross_chunks_without_rectangular_fill() {
        let deposit = Deposit {
            x: 64.,
            y: 64.,
            length: 160.,
            width: 9.,
            slope: 0.15,
            shape: 0,
            seed: 42,
        };
        let mut chunks = std::collections::BTreeMap::new();
        for y in 0..128 {
            for x in 0..256 {
                if deposit.contains(x as f64, y as f64) {
                    *chunks.entry((x / 64, y / 64)).or_insert(0) += 1;
                }
            }
        }
        assert!(chunks.len() >= 3);
        assert!(chunks.values().all(|count| *count < 2048));
    }
    #[test]
    fn sample_order_cannot_change_geology_and_starters_exist() {
        let cat = crate::materials();
        for profile in 0..sites().len() {
            let points: Vec<_> = (0..512).step_by(5).map(|x| (x, x / 2)).collect();
            let first: Vec<_> = points
                .iter()
                .map(|&(x, y)| sample(42, profile, x, y, &cat))
                .collect();
            let reverse: Vec<_> = points
                .iter()
                .rev()
                .map(|&(x, y)| sample(42, profile, x, y, &cat))
                .collect();
            assert_eq!(first, reverse.into_iter().rev().collect::<Vec<_>>());
            for (id, cy) in [(3, 24), (5, 76), (6, 130)] {
                let reserve = (190..320)
                    .flat_map(|x| (cy - 9..cy + 10).map(move |y| (x, y)))
                    .filter(|&(x, y)| sample(42, profile, x, y, &cat) == id)
                    .count();
                assert!(reserve > 300);
            }
        }
    }
}
