//! Mineral-specific, finite world-space bodies. One body owns connected lobes,
//! branches and coherent companion zones, including across chunk boundaries.
use super::{hash, noise, reserves, tier, Deposit};
use crate::{geometry, sites, Material};
use std::cell::RefCell;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Family {
    Bed,
    Blanket,
    Lode,
    Lens,
    Stockwork,
    Pegmatite,
    Pipe,
    Pocket,
}
#[derive(Clone, Copy)]
struct Profile {
    family: Family,
    length: [f64; 2],
    width: [f64; 2],
    weight: u32,
    companions: &'static [usize],
}
const fn p(
    family: Family,
    length: [f64; 2],
    width: [f64; 2],
    weight: u32,
    companions: &'static [usize],
) -> Profile {
    Profile {
        family,
        length,
        width,
        weight,
        companions,
    }
}
use Family::*;
// Explicit identities: distributions follow deposit setting, never name matching.
// Dimensions describe game-scaled ore-bearing rock, not pure-mineral grade.
const PROFILES: [Profile; 56] = [
    p(Blanket, [90., 180.], [10., 24.], 0, &[]), // Dirt (host only)
    p(Lens, [80., 180.], [12., 30.], 0, &[]),    // Stone (host only)
    p(Lode, [140., 420.], [5., 15.], 12, &[19, 12]), // Quartz
    p(Bed, [150., 360.], [12., 28.], 16, &[7, 28]), // Hematite
    p(Pocket, [45., 120.], [12., 28.], 9, &[8]), // Malachite alteration
    p(Bed, [220., 560.], [9., 24.], 16, &[10]),  // Coal
    p(Bed, [240., 600.], [18., 42.], 15, &[11]), // Limestone
    p(Lens, [110., 290.], [18., 46.], 10, &[3, 34]), // Magnetite
    p(Stockwork, [130., 360.], [12., 32.], 11, &[25, 26]), // Chalcopyrite
    p(Lode, [120., 320.], [4., 13.], 6, &[2, 30, 31]), // Cassiterite
    p(Blanket, [160., 380.], [12., 30.], 10, &[12]), // Clay
    p(Bed, [180., 430.], [10., 26.], 8, &[13, 6]), // Gypsum
    p(Pegmatite, [90., 240.], [15., 38.], 9, &[2, 19]), // Feldspar
    p(Bed, [230., 580.], [16., 40.], 9, &[14, 11]), // Halite
    p(Bed, [160., 390.], [8., 20.], 7, &[13]),   // Potash
    p(Bed, [180., 410.], [10., 24.], 7, &[6]),   // Phosphate rock
    p(Lode, [120., 300.], [6., 18.], 6, &[17, 2]), // Fluorite
    p(Lode, [100., 260.], [7., 20.], 6, &[16, 23]), // Barite
    p(Lens, [100., 250.], [15., 34.], 6, &[37]), // Talc
    p(Pegmatite, [100., 270.], [12., 32.], 8, &[2, 12]), // Mica
    p(Bed, [170., 400.], [8., 22.], 8, &[2]),    // Graphite
    p(Pocket, [60., 160.], [10., 24.], 6, &[11]), // Sulfur
    p(Lens, [130., 320.], [14., 32.], 7, &[23, 40]), // Sphalerite
    p(Lens, [110., 280.], [12., 28.], 7, &[22, 40]), // Galena
    p(Blanket, [210., 480.], [14., 32.], 10, &[10, 3]), // Bauxite
    p(Lens, [130., 330.], [15., 36.], 7, &[8, 26]), // Pentlandite
    p(Lens, [100., 260.], [12., 30.], 6, &[25, 8]), // Cobalt ore
    p(Lens, [160., 390.], [10., 28.], 7, &[7, 41]), // Chromite cumulate
    p(Bed, [150., 360.], [10., 26.], 7, &[3]),   // Manganese ore
    p(Stockwork, [120., 300.], [10., 28.], 5, &[8, 2]), // Molybdenite
    p(Lode, [130., 340.], [5., 16.], 6, &[9, 2]), // Scheelite
    p(Lode, [140., 360.], [4., 14.], 6, &[9, 2]), // Wolframite
    p(Lens, [150., 370.], [12., 32.], 7, &[33, 38]), // Ilmenite
    p(Lens, [120., 310.], [10., 26.], 6, &[32, 38]), // Rutile
    p(Lens, [150., 380.], [12., 30.], 6, &[7]),  // Vanadium ore
    p(Pipe, [75., 180.], [18., 42.], 4, &[53]),  // Pyrochlore carbonatite
    p(Pegmatite, [90., 240.], [12., 30.], 4, &[42, 43, 2]), // Tantalite
    p(Lens, [140., 340.], [15., 36.], 7, &[18]), // Magnesite
    p(Lens, [100., 260.], [9., 24.], 5, &[32, 33]), // Zircon
    p(Lode, [180., 460.], [3., 10.], 4, &[2, 46, 47]), // Native gold
    p(Lode, [140., 380.], [4., 13.], 4, &[23, 22]), // Silver ore
    p(Lens, [140., 350.], [8., 22.], 3, &[27, 25]), // Platinum concentrate
    p(Pegmatite, [140., 330.], [18., 42.], 6, &[2, 12, 19, 36]), // Spodumene
    p(Pegmatite, [100., 260.], [14., 34.], 5, &[2, 12, 52]), // Lepidolite
    p(Pegmatite, [85., 210.], [10., 25.], 4, &[2, 12, 19]), // Beryl
    p(Bed, [170., 410.], [12., 28.], 6, &[13, 11]), // Borates
    p(Lode, [110., 290.], [4., 12.], 4, &[2, 39]), // Stibnite
    p(Lode, [90., 250.], [5., 14.], 4, &[2, 30]), // Bismuth ore
    p(Pocket, [50., 130.], [12., 30.], 5, &[2]), // Garnet
    p(Pocket, [45., 115.], [10., 26.], 4, &[19]), // Corundum
    p(Pipe, [120., 290.], [16., 38.], 3, &[48]), // Diamond rock / kimberlite
    p(Pocket, [35., 90.], [9., 22.], 3, &[2, 44, 49]), // Gem pocket
    p(Pegmatite, [80., 220.], [12., 28.], 3, &[43, 2, 12]), // Pollucite
    p(Pipe, [100., 250.], [18., 42.], 4, &[54, 35, 16]), // Bastnaesite
    p(Lens, [100., 260.], [14., 34.], 4, &[53, 55]), // Monazite
    p(Lode, [100., 270.], [5., 17.], 3, &[54, 2]), // Xenotime
];

#[derive(Clone, Copy)]
struct Body {
    x: f64,
    y: f64,
    length: f64,
    width: f64,
    cos: f64,
    sin: f64,
    extent_x: f64,
    extent_y: f64,
    seed: u64,
    material: usize,
    family: Family,
    lobes: f64,
}
impl Body {
    fn new(seed: u64, material: usize, x: f64, y: f64, reserve: bool) -> Self {
        let profile = PROFILES[material];
        let fraction = |shift: u32| ((seed >> shift) & 1023u64) as f64 / 1023.;
        let mut length = profile.length[0] + fraction(12) * (profile.length[1] - profile.length[0]);
        let mut width = profile.width[0] + fraction(26) * (profile.width[1] - profile.width[0]);
        let angle = match profile.family {
            Bed | Blanket => (fraction(38) - 0.5) * 0.22,
            Pipe => 1.57 + (fraction(38) - 0.5) * 0.3,
            Lode => (fraction(38) - 0.5) * 2.4,
            _ => (fraction(38) - 0.5) * 1.4,
        };
        // Accessible reserves stay within their authored depth band. Their
        // silhouettes still use the mineral family, with enough primary feed.
        let angle = if reserve {
            (fraction(38) - 0.5) * 0.08
        } else {
            angle
        };
        if reserve {
            length = 175. + fraction(12) * 35.;
            width = 15. + fraction(26) * 7.;
        }
        let (sin, cos) = angle.sin_cos();
        let extra = width
            * if matches!(profile.family, Stockwork | Lode) {
                2.1
            } else {
                1.45
            };
        Self {
            x,
            y,
            length,
            width,
            sin,
            cos,
            extent_x: cos.abs() * length + sin.abs() * extra + 2.,
            extent_y: sin.abs() * length + cos.abs() * extra + 2.,
            seed,
            material,
            family: profile.family,
            lobes: 2. + ((seed >> 50) % 3) as f64,
        }
    }
    fn identity(&self) -> String {
        format!(
            "6:{:016x}:{:x}:{:x}",
            self.seed,
            self.x.to_bits(),
            self.y.to_bits()
        )
    }
    /// Normalized distance from connected ore skeleton. Continuous thickness
    /// keeps merged lobes connected; fine texture only breaks outer margins.
    fn normalized(&self, x: i64, y: i64) -> Option<(f64, f64)> {
        let dx = x as f64 - self.x;
        let dy = y as f64 - self.y;
        if dx.abs() > self.extent_x || dy.abs() > self.extent_y {
            return None;
        }
        let u = dx * self.cos + dy * self.sin;
        let v = -dx * self.sin + dy * self.cos;
        let t = u / self.length;
        if t.abs() >= 1. {
            return None;
        }
        let wave = (t * std::f64::consts::PI * self.lobes + (self.seed % 17) as f64).sin();
        let taper = (1. - t * t).sqrt();
        let (centre, thickness) = match self.family {
            Bed => (
                wave * self.width * 0.30,
                self.width * (0.85 + 0.15 * wave) * taper,
            ),
            Blanket => (
                wave * self.width * 0.16,
                self.width * (0.92 + 0.08 * wave) * taper,
            ),
            Lode => (
                wave * self.width * 0.55,
                self.width * (0.7 + 0.3 * wave) * taper,
            ),
            Lens => (
                wave * self.width * 0.18,
                self.width * (0.65 + 0.35 * wave) * taper,
            ),
            Stockwork => (
                wave * self.width * 0.35,
                self.width * (0.30 + 0.05 * wave) * taper,
            ),
            Pegmatite => (
                wave * self.width * 0.22,
                self.width * (0.72 + 0.28 * wave) * (1. - t.abs()).sqrt(),
            ),
            Pipe => (
                wave * self.width * 0.18,
                self.width * (0.8 + 0.2 * t) * taper,
            ),
            Pocket => (0., self.width * (0.94 + 0.06 * wave) * taper),
        };
        // Weathering follows a flatter upper surface and an uneven basal contact.
        let thickness = if self.family == Blanket && v < centre {
            self.width * 0.8 * taper
        } else {
            thickness
        };
        let mut normalized = (v - centre).abs() / thickness.max(0.01);
        // Linked splays merge into parent vein; no separate deposit identities.
        if matches!(self.family, Lode | Stockwork) && t > 0. && t < 0.68 {
            let branch = centre + self.width * 1.4 * (t * std::f64::consts::PI / 0.68).sin();
            normalized = normalized.min((v - branch).abs() / (thickness * 0.45).max(0.01));
        }
        if self.family == Stockwork && t.abs() < 0.75 {
            // Three ore strings merge at both ends into the main skeleton.
            let offset = self.width * (t * std::f64::consts::PI / 0.75).cos().abs();
            for sign in [-1., 1.] {
                normalized = normalized
                    .min((v - centre - sign * offset).abs() / (thickness * 0.8).max(0.01));
            }
        }
        if normalized > 1. {
            return None;
        }
        if normalized > 0.88 && noise(self.seed, x.div_euclid(3), y.div_euclid(3)) < 0.28 {
            return None;
        }
        Some((t, normalized))
    }
    fn material_at(&self, x: i64, y: i64, cat: &[Material]) -> Option<usize> {
        let (along, distance) = self.normalized(x, y)?;
        let depth_tier = tier(y / geometry::CELLS_PER_METRE);
        if cat[self.material].tier > depth_tier {
            return None;
        }
        let companions = PROFILES[self.material].companions;
        // Continuous margins, lenses and concentric pegmatite zones. Primary
        // core stays intact, guaranteeing useful feed rather than confetti.
        let boundary = 0.78 + 0.08 * (along * 8. + (self.seed % 11) as f64).sin();
        if distance > boundary && !companions.is_empty() {
            let zone = ((along + 1.) * 1.5 + (self.seed % 7) as f64).floor() as usize;
            let companion = companions[zone % companions.len()];
            if cat[companion].tier <= depth_tier {
                return Some(companion);
            }
        }
        Some(self.material)
    }
}
#[derive(Clone, Copy)]
enum Found {
    Starter(usize, Deposit),
    Body(Body, usize),
}
impl Found {
    fn material(self) -> usize {
        match self {
            Self::Starter(id, _) | Self::Body(_, id) => id,
        }
    }
    fn identity(self) -> String {
        match self {
            Self::Starter(_, d) => d.identity(),
            Self::Body(d, _) => d.identity(),
        }
    }
}
struct Region {
    key: (u64, usize, i64, i64),
    bodies: Vec<Body>,
}
// Eight bounded regions retain neighbouring chunks while scanning whole bodies.
// No shared mutex, unbounded world cache, or sampling-order-dependent geology.
thread_local! { static REGION: RefCell<Vec<Region>> = const { RefCell::new(Vec::new()) }; }
fn choose(key: u64, profile: usize, band: u32, cat: &[Material]) -> usize {
    let mut weights = [0u32; 56];
    let mut total = 0;
    for material in cat.iter().skip(2) {
        if material.tier <= band {
            let weight = PROFILES[material.id].weight
                * if sites()[profile].focus.contains(&material.id) {
                    3
                } else {
                    1
                };
            weights[material.id] = weight;
            total += weight;
        }
    }
    let mut choice = (key % total as u64) as u32;
    for (id, weight) in weights.into_iter().enumerate() {
        if choice < weight {
            return id;
        }
        choice -= weight;
    }
    unreachable!()
}
fn region(seed: u64, profile: usize, tx: i64, ty: i64, cat: &[Material]) -> Region {
    let mut bodies = Vec::with_capacity(60);
    for (index, reserve) in reserves().iter().enumerate() {
        let y = reserve.depth_metres as i64 * 4 + 4;
        if (ty * 512 + 256).abs_diff(y) > 768 || tx.abs() > 2 {
            continue;
        }
        let key = hash(seed ^ (index as u64 + 1).wrapping_mul(0xa0761d6478bd642f));
        bodies.push(Body::new(
            key,
            reserve.feed,
            192. + (key % 129) as f64,
            y as f64,
            true,
        ));
    }
    for gy in ty - 2..=ty + 2 {
        for gx in tx - 2..=tx + 2 {
            if gy < 0 {
                continue;
            }
            for slot in 0..2u64 {
                let key = hash(
                    seed ^ (gx as u64).wrapping_mul(0x9e3779b97f4a7c15)
                        ^ (gy as u64).rotate_left(32)
                        ^ slot.wrapping_mul(0xd6e8feb86659fd93),
                );
                let x = gx * 512 + (key % 512) as i64;
                let y = gy * 512 + ((key >> 9) % 512) as i64;
                let id = choose(hash(key), profile, tier(y / 4), cat);
                bodies.push(Body::new(key, id, x as f64, y as f64, false));
            }
        }
    }
    Region {
        key: (seed, profile, tx, ty),
        bodies,
    }
}
fn locate(seed: u64, profile: usize, x: i64, y: i64, cat: &[Material]) -> Option<Found> {
    if !geometry::valid_cell(x, y) {
        return None;
    }
    // Finite authored starter patches remain byte-for-byte compatible.
    if y < 150 {
        for (index, (id, cy, radius)) in [(3, 24., 35.), (5, 76., 43.), (6, 130., 48.)]
            .into_iter()
            .enumerate()
        {
            let d = Deposit {
                x: 256. + (index as f64 - 1.) * 21.,
                y: cy,
                length: radius,
                width: 9.,
                slope: 0.08,
                shape: 1,
                seed: hash(seed + index as u64),
            };
            if d.contains(x as f64, y as f64) {
                return Some(Found::Starter(id, d));
            }
        }
    }
    let key = (seed, profile, x.div_euclid(512), y.div_euclid(512));
    REGION.with(|cache| {
        let mut cache = cache.borrow_mut();
        let index = if let Some(index) = cache.iter().position(|r| r.key == key) {
            index
        } else {
            if cache.len() == 8 {
                cache.remove(0);
            }
            cache.push(region(seed, profile, key.2, key.3, cat));
            cache.len() - 1
        };
        cache[index]
            .bodies
            .iter()
            .find_map(|body| body.material_at(x, y, cat).map(|id| Found::Body(*body, id)))
    })
}
pub(super) fn sample(seed: u64, profile: usize, x: i64, y: i64, cat: &[Material]) -> usize {
    locate(seed, profile, x, y, cat).map_or(if y < 32 { 0 } else { 1 }, Found::material)
}
pub(super) fn deposit_id(
    seed: u64,
    profile: usize,
    p: [i64; 2],
    cat: &[Material],
) -> Option<String> {
    locate(seed, profile, p[0], p[1], cat).map(Found::identity)
}
pub(super) fn deposit_cells(
    seed: u64,
    profile: usize,
    p: [i64; 2],
    cat: &[Material],
) -> Vec<[i64; 2]> {
    let Some(found) = locate(seed, profile, p[0], p[1], cat) else {
        return vec![];
    };
    if matches!(found, Found::Starter(..)) {
        return super::deposit_cells(seed, profile, p, cat);
    }
    let Found::Body(body, _) = found else {
        unreachable!()
    };
    let mut cells = Vec::new();
    for y in (body.y - body.extent_y).max(0.) as i64
        ..=(body.y + body.extent_y).min((geometry::MAX_ROWS - 1) as f64) as i64
    {
        for x in (body.x - body.extent_x).max(-geometry::MAX_X as f64) as i64
            ..=(body.x + body.extent_x).min((geometry::MAX_X - 1) as f64) as i64
        {
            if body.material_at(x, y, cat).is_some()
                && matches!(locate(seed,profile,x,y,cat),Some(Found::Body(d,_)) if d.seed==body.seed && d.x==body.x && d.y==body.y)
            {
                cells.push([x, y]);
            }
        }
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, VecDeque};
    #[test]
    fn profiles_differ_by_mineral_and_cover_catalogue() {
        let cat = crate::materials();
        assert_eq!(cat.len(), PROFILES.len());
        assert_eq!(PROFILES[5].family, Bed);
        assert_eq!(PROFILES[39].family, Lode);
        assert_eq!(PROFILES[24].family, Blanket);
        assert_eq!(PROFILES[42].family, Pegmatite);
        assert_eq!(PROFILES[50].family, Pipe);
        assert_eq!(PROFILES[51].family, Pocket);
        assert!(PROFILES[5].length[1] > PROFILES[51].length[1] * 5.);
        assert!(PROFILES[39].width[1] < PROFILES[42].width[0]);
        for profile in PROFILES.iter().skip(2) {
            assert!(profile.weight > 0);
            assert!(profile
                .companions
                .iter()
                .all(|id| *id < cat.len() && *id > 1));
        }
    }
    #[test]
    fn region_cache_is_bounded_and_eviction_cannot_change_samples() {
        let cat = crate::materials();
        let first = sample(42, 0, -512, 6400, cat);
        for region in -32..32 {
            sample(42, 0, region * 512, 6400, cat);
        }
        REGION.with(|cache| assert_eq!(cache.borrow().len(), 8));
        assert_eq!(sample(42, 0, -512, 6400, cat), first);
    }
    #[test]
    fn signed_chunks_sampling_order_and_depth_tiers_are_stable() {
        let cat = crate::materials();
        let points: Vec<_> = (-700..700)
            .step_by(13)
            .flat_map(|x| [64, 511, 512, 1199, 1200, 6000, 12000].map(move |y| [x, y]))
            .collect();
        for profile in 0..3 {
            let first: Vec<_> = points
                .iter()
                .map(|p| sample(42, profile, p[0], p[1], cat))
                .collect();
            let back: Vec<_> = points
                .iter()
                .rev()
                .map(|p| sample(42, profile, p[0], p[1], cat))
                .collect();
            assert_eq!(first, back.into_iter().rev().collect::<Vec<_>>());
            for (point, id) in points.iter().zip(first) {
                assert!(cat[id].tier <= tier(point[1] / 4));
            }
        }
    }
    #[test]
    fn merged_lobes_and_mixed_zones_share_connected_body() {
        let cat = crate::materials();
        for material in [5, 24, 39, 7, 8, 42, 50, 51] {
            let body = Body::new(42, material, -64., 16000., false);
            let mut cells = BTreeSet::new();
            let mut primary = 0;
            let mut mixed = BTreeSet::new();
            for y in (body.y - body.extent_y) as i64..=(body.y + body.extent_y) as i64 {
                for x in (body.x - body.extent_x) as i64..=(body.x + body.extent_x) as i64 {
                    if let Some(id) = body.material_at(x, y, cat) {
                        cells.insert([x, y]);
                        if id == material {
                            primary += 1;
                        } else {
                            mixed.insert(id);
                        }
                    }
                }
            }
            assert!(primary > cells.len() * 7 / 10);
            assert!(!mixed.is_empty());
            assert!(mixed
                .iter()
                .all(|id| PROFILES[material].companions.contains(id)));
            let anchor = *cells
                .iter()
                .min_by_key(|p| p[0].abs_diff(body.x as i64) + p[1].abs_diff(body.y as i64))
                .unwrap();
            let mut reached = BTreeSet::from([anchor]);
            let mut queue = VecDeque::from([anchor]);
            while let Some([x, y]) = queue.pop_front() {
                for next in [[x - 1, y], [x + 1, y], [x, y - 1], [x, y + 1]] {
                    if cells.contains(&next) && reached.insert(next) {
                        queue.push_back(next);
                    }
                }
            }
            assert!(
                reached.len() * 100 >= cells.len() * 98,
                "{} of {} connected for material {material}",
                reached.len(),
                cells.len()
            );
            let chunks: BTreeSet<_> = cells
                .iter()
                .map(|p| geometry::chunk_id(p[0], p[1]))
                .collect();
            assert!(chunks.len() >= 2);
        }
    }
    #[test]
    fn authored_reserves_keep_meaningful_primary_feed_for_all_seeds() {
        let cat = crate::materials();
        for seed in 42..72 {
            for reserve in reserves() {
                let cy = reserve.depth_metres as i64 * 4 + 4;
                for profile in 0..3 {
                    let count = (32..geometry::WIDTH - 32)
                        .step_by(4)
                        .flat_map(|x| (cy - 64..=cy + 64).step_by(4).map(move |y| [x, y]))
                        .filter(|p| sample(seed, profile, p[0], p[1], cat) == reserve.feed)
                        .count();
                    assert!(
                        count * 16 >= 1500,
                        "seed {seed}, profile {profile}, feed {}, depth {}: {} primary cells",
                        reserve.feed,
                        reserve.depth_metres,
                        count * 16
                    );
                }
            }
        }
    }
    #[test]
    fn whole_deposit_identity_spans_companions_without_duplicate_cells() {
        let cat = crate::materials();
        let reserve = &reserves()[0];
        let key = hash(42 ^ 0xa0761d6478bd642f);
        let anchor = [
            (192. + (key % 129) as f64) as i64,
            reserve.depth_metres as i64 * 4 + 4,
        ];
        let id = deposit_id(42, 0, anchor, cat).unwrap();
        let cells = deposit_cells(42, 0, anchor, cat);
        assert!(cells.len() > 1500);
        assert_eq!(
            cells.iter().copied().collect::<BTreeSet<_>>().len(),
            cells.len()
        );
        let mut mixed = false;
        for p in cells {
            assert_eq!(deposit_id(42, 0, p, cat).as_deref(), Some(id.as_str()));
            mixed |= sample(42, 0, p[0], p[1], cat) != reserve.feed;
        }
        assert!(mixed);
    }
}
