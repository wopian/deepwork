//! Independent exposed-ore work fronts. Only revealed, reachable geology may
//! become a front; deposit identities group one vein across chunk boundaries.
use crate::{
    geology,
    geometry::{cell_point, UNITS, WIDTH},
    terrain::Terrain,
    workings::Workings,
    Material,
};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

pub type Point = [i64; 2];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MiningFront {
    pub id: String,
    pub deposit: String,
    pub material: usize,
    pub face: Point,
    /// Last fully excavated work position. Render workers here while the next
    /// full-height cut is still solid.
    #[serde(default)]
    pub position: Point,
    pub crew: u32,
    pub haulers: u32,
    pub progress: u64,
    /// Work needed to atomically finish the current full-clearance cut.
    #[serde(default)]
    pub cut_work: u64,
    #[serde(default)]
    pub work_remainder: u64,
    #[serde(default)]
    pub transfer_remainder: u64,
    pub stockpile: u64,
    /// Exact material ownership for full-clearance cuts. `stockpile` remains
    /// the total used by capacity and public telemetry.
    #[serde(default)]
    pub stockpiles: BTreeMap<usize, u64>,
    pub capacity: u64,
    pub route: Vec<Point>,
    #[serde(default)]
    pub route_id: u64,
    pub selected: bool,
    pub status: String,
    pub blocker: String,
}

/// Seven cells wide by eight high: enough for the five-pixel worker body and
/// pick swing, and identical to commissioned passage/lift clearance.
pub fn clearance_cells(face: Point) -> Vec<Point> {
    let mut cells = Vec::with_capacity(56);
    for x in face[0].saturating_sub(3)..=face[0].saturating_add(3) {
        for y in face[1].saturating_sub(7).max(0)..=face[1] {
            cells.push([x, y]);
        }
    }
    // Frontier excavation must start at the exposed face. Increasing distance
    // then guarantees every later cell touches an already opened cell.
    cells.sort_by_key(|cell| {
        (
            cell[0].abs_diff(face[0]) + cell[1].abs_diff(face[1]),
            Reverse(cell[1]),
            cell[0],
        )
    });
    cells
}

#[derive(Clone)]
struct Candidate {
    id: String,
    deposit: String,
    material: usize,
    face: Point,
    route: Vec<Point>,
    selected: bool,
    required: bool,
    priority: usize,
    distance: u64,
}

fn candidates(
    terrain: &Terrain,
    workings: &Workings,
    seed: u64,
    profile: usize,
    catalogue: &[Material],
    required: &BTreeSet<usize>,
    priorities: &[usize],
) -> Vec<Candidate> {
    // Sixteen-cell face buckets permit several crews around a large vein while
    // keeping identities stable as each face advances through fine cells.
    let mut groups = BTreeMap::<(String, i64, i64), (usize, Vec<Point>)>::new();
    for (&material, cells) in &terrain.ore_frontiers {
        if material <= 1 {
            continue;
        }
        for &key in cells {
            let face = cell_point(key);
            let Some(deposit) = geology::deposit_id(seed, profile, face, catalogue) else {
                continue;
            };
            groups
                .entry((deposit, face[0].div_euclid(16), face[1].div_euclid(16)))
                .or_insert_with(|| (material, Vec::new()))
                .1
                .push(face);
        }
    }
    let selected = workings.target_deposit.as_deref();
    groups
        .into_iter()
        .filter_map(|((deposit, bx, by), (material, faces))| {
            let face = faces
                .into_iter()
                .min_by_key(|face| (face[1], face[0].abs_diff(WIDTH / 2), face[0]))?;
            let route = workings.route_near(face);
            if route.is_empty() {
                return None;
            }
            Some(Candidate {
                id: format!("{deposit}:{bx}:{by}"),
                selected: selected == Some(deposit.as_str()),
                required: required.contains(&material),
                priority: priorities
                    .iter()
                    .position(|id| *id == material)
                    .unwrap_or(usize::MAX),
                distance: face[1] as u64 + face[0].abs_diff(WIDTH / 2),
                deposit,
                material,
                face,
                route,
            })
        })
        .collect()
}

/// Retain progress and stockpiles while rebuilding visible, reachable faces.
/// Selection is diversity-first so one large deposit cannot consume every crew.
pub fn refresh(
    fronts: &mut Vec<MiningFront>,
    terrain: &Terrain,
    workings: &Workings,
    seed: u64,
    profile: usize,
    catalogue: &[Material],
    required: &BTreeSet<usize>,
    priorities: &[usize],
    max_fronts: usize,
    buffer_level: u32,
) {
    let mut candidates = candidates(
        terrain, workings, seed, profile, catalogue, required, priorities,
    );
    candidates.sort_by_key(|c| {
        (
            !c.selected,
            !c.required,
            c.priority,
            c.distance,
            c.material,
            c.id.clone(),
        )
    });

    let mut chosen = Vec::new();
    let mut materials = BTreeSet::new();
    // First pass gives distinct feeds independent work ownership.
    for candidate in &candidates {
        if chosen.len() >= max_fronts {
            break;
        }
        if materials.insert(candidate.material) {
            chosen.push(candidate.clone());
        }
    }
    for candidate in candidates {
        if chosen.len() >= max_fronts {
            break;
        }
        if !chosen.iter().any(|c| c.id == candidate.id) {
            chosen.push(candidate);
        }
    }

    let mut old: BTreeMap<_, _> = fronts
        .drain(..)
        .map(|front| (front.id.clone(), front))
        .collect();
    let capacity = 20 * UNITS * (4 + buffer_level as u64) / 4;
    for candidate in chosen {
        let nearest = old
            .iter()
            .filter(|(_, front)| {
                front.deposit == candidate.deposit && front.material == candidate.material
            })
            .min_by_key(|(_, front)| {
                front.face[0].abs_diff(candidate.face[0])
                    + front.face[1].abs_diff(candidate.face[1])
            })
            .map(|(id, _)| id.clone());
        let mut front = old
            .remove(&candidate.id)
            .or_else(|| nearest.and_then(|id| old.remove(&id)))
            .unwrap_or_else(|| MiningFront {
                id: candidate.id.clone(),
                deposit: candidate.deposit.clone(),
                material: candidate.material,
                face: candidate.face,
                position: candidate.route.last().copied().unwrap_or(candidate.face),
                crew: 0,
                haulers: 0,
                progress: 0,
                cut_work: 0,
                work_remainder: 0,
                transfer_remainder: 0,
                stockpile: 0,
                stockpiles: BTreeMap::new(),
                capacity,
                route: candidate.route.clone(),
                route_id: 0,
                selected: candidate.selected,
                status: "Awaiting crew".into(),
                blocker: String::new(),
            });
        if front.stockpiles.is_empty() && front.stockpile > 0 {
            front.stockpiles.insert(front.material, front.stockpile);
        }
        if front.position == [0, 0] {
            front.position = front.route.last().copied().unwrap_or(front.face);
        }
        let existing_face = front.face;
        let keep_existing_face = !terrain.contains(existing_face[0], existing_face[1])
            && terrain.known_material(existing_face[0], existing_face[1])
                == Some(candidate.material)
            && geology::deposit_id(seed, profile, existing_face, catalogue).as_deref()
                == Some(candidate.deposit.as_str());
        front.id = candidate.id;
        front.deposit = candidate.deposit;
        front.material = candidate.material;
        let next_face = if keep_existing_face {
            existing_face
        } else {
            candidate.face
        };
        if next_face != front.face {
            front.cut_work = 0;
        }
        front.face = next_face;
        // Keep cargo ownership stable while a face advances through one vein.
        // Route replacement happens after existing cargo drains or the route
        // ceases to exist, avoiding a new network itinerary per mined cell.
        let route_stale = front.route.last().is_none_or(|end| {
            end[0].abs_diff(candidate.face[0]) + end[1].abs_diff(candidate.face[1]) > 64
        });
        if front.route.is_empty() || route_stale {
            front.route = candidate.route;
            front.route_id = 0;
        }
        front.selected = candidate.selected;
        front.capacity = capacity;
        fronts.push(front);
    }
    // A depleted face may still own cargo. Keep it until its work-face stockpile drains.
    for mut front in old.into_values().filter(|front| front.stockpile > 0) {
        front.crew = 0;
        front.haulers = 0;
        front.status = "Deposit exhausted".into();
        front.blocker.clear();
        fronts.push(front);
    }
}

/// Automatic allocation. A selected whole vein receives at least 60% of
/// diggers; remaining workers keep access and critical feeds moving.
pub fn assign(fronts: &mut [MiningFront], diggers: u32, haulers: u32, access_pending: bool) -> u32 {
    for front in fronts.iter_mut() {
        front.crew = 0;
        front.haulers = 0;
    }
    if diggers == 0 {
        return 0;
    }
    let selected: Vec<_> = fronts
        .iter()
        .enumerate()
        .filter(|(_, front)| front.selected && front.stockpile < front.capacity)
        .map(|(index, _)| index)
        .collect();
    let available: Vec<_> = fronts
        .iter()
        .enumerate()
        .filter(|(_, front)| front.stockpile < front.capacity)
        .map(|(index, _)| index)
        .collect();
    let mut access_crew = 0;
    let mut remaining = diggers;
    if !selected.is_empty() {
        let target = (diggers * 3).div_ceil(5);
        for n in 0..target {
            fronts[selected[n as usize % selected.len()]].crew += 1;
            remaining -= 1;
        }
    } else if access_pending {
        access_crew = 1;
        remaining -= 1;
    }
    let secondary: Vec<_> = available
        .iter()
        .copied()
        .filter(|index| !fronts[*index].selected)
        .collect();
    let pool = if secondary.is_empty() {
        &available
    } else {
        &secondary
    };
    for n in 0..remaining {
        if pool.is_empty() {
            access_crew += 1;
        } else {
            fronts[pool[n as usize % pool.len()]].crew += 1;
        }
    }
    if access_pending && access_crew == 0 && selected.is_empty() && available.is_empty() {
        access_crew = diggers;
    }

    let hauling: Vec<_> = fronts
        .iter()
        .enumerate()
        .filter(|(_, front)| front.stockpile > 0 || front.crew > 0)
        .map(|(index, _)| index)
        .collect();
    for n in 0..haulers {
        if !hauling.is_empty() {
            fronts[hauling[n as usize % hauling.len()]].haulers += 1;
        }
    }
    for front in fronts {
        if front.stockpile >= front.capacity {
            front.status = "Paused".into();
            front.blocker = "Work-face stockpile full".into();
        } else if front.crew > 0 {
            front.status = "Excavating".into();
            front.blocker.clear();
        } else if front.stockpile > 0 && front.haulers == 0 {
            front.status = "Waiting for haul crew".into();
            front.blocker = "No haul crew assigned".into();
        } else {
            front.status = "Awaiting crew".into();
            front.blocker.clear();
        }
    }
    access_crew
}

/// Continue along revealed ore from same world-space deposit. Hidden cells are
/// never candidates, so automatic crews cannot use private geology as radar.
pub fn next_face(
    front: &MiningFront,
    terrain: &Terrain,
    seed: u64,
    profile: usize,
    catalogue: &[Material],
    claimed: &BTreeSet<Point>,
) -> Option<Point> {
    let matches = |face: &Point| {
        !claimed.contains(face)
            && geology::deposit_id(seed, profile, *face, catalogue).as_deref()
                == Some(front.deposit.as_str())
    };
    if let Some(face) = Terrain::neighbors(front.face[0], front.face[1])
        .map(|(x, y)| [x, y])
        .find(|face| {
            terrain
                .ore_frontiers
                .get(&front.material)
                .is_some_and(|cells| cells.contains(&crate::geometry::cell_key(face[0], face[1])))
                && matches(face)
        })
    {
        return Some(face);
    }
    terrain
        .ore_frontiers
        .get(&front.material)?
        .iter()
        .map(|key| cell_point(*key))
        .filter(matches)
        .min_by_key(|face| {
            (
                face[0].abs_diff(front.face[0]) + face[1].abs_diff(front.face[1]),
                face[1],
                face[0],
            )
        })
}

pub fn valid(fronts: &[MiningFront], materials: usize, max_fronts: usize) -> bool {
    fronts.len() <= max_fronts + 16
        && fronts.iter().all(|front| {
            front.material < materials
                && !front.id.is_empty()
                && !front.deposit.is_empty()
                && crate::geometry::valid_cell(front.face[0], front.face[1])
                && crate::geometry::valid_cell(front.position[0], front.position[1])
                && front.progress <= 1_000_000_000
                && front.cut_work <= 56_000
                && front.work_remainder < 20
                && front.transfer_remainder < 20
                && front.stockpile <= front.capacity
                && (front.stockpiles.is_empty()
                    || front.stockpiles.values().sum::<u64>() == front.stockpile)
                && front
                    .stockpiles
                    .keys()
                    .all(|material| *material < materials)
                && front.capacity <= 1_000_000_000_000
                && front.route.len() <= 1_000_000
                && front.route_id < 1_000_000_000_000
                && front
                    .route
                    .iter()
                    .all(|p| crate::geometry::valid_cell(p[0], p[1]))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn front(id: &str, material: usize, selected: bool) -> MiningFront {
        MiningFront {
            id: id.into(),
            deposit: id.into(),
            material,
            face: [256, 1],
            position: [256, 1],
            crew: 0,
            haulers: 0,
            progress: 0,
            cut_work: 0,
            work_remainder: 0,
            transfer_remainder: 0,
            stockpile: 0,
            stockpiles: BTreeMap::new(),
            capacity: 20 * UNITS,
            route: vec![[256, 0]],
            route_id: 0,
            selected,
            status: String::new(),
            blocker: String::new(),
        }
    }

    #[test]
    fn whole_vein_gets_sixty_percent_without_starving_other_feeds() {
        let mut fronts = vec![
            front("iron", 3, true),
            front("coke", 5, false),
            front("lime", 6, false),
        ];
        let access = assign(&mut fronts, 5, 2, true);
        assert_eq!(fronts[0].crew, 3);
        assert_eq!(fronts[1].crew + fronts[2].crew, 2);
        assert_eq!(access, 0);
        assert_eq!(fronts.iter().map(|f| f.haulers).sum::<u32>(), 2);
    }

    #[test]
    fn no_selected_vein_spreads_diggers_across_materials_and_access() {
        let mut fronts = vec![
            front("iron", 3, false),
            front("coke", 5, false),
            front("lime", 6, false),
        ];
        let access = assign(&mut fronts, 4, 1, true);
        assert_eq!(access, 1);
        assert!(fronts.iter().all(|f| f.crew == 1));
    }

    #[test]
    fn full_workface_pauses_locally_while_other_fronts_keep_mining() {
        let mut fronts = vec![front("blocked", 3, false), front("flowing", 5, false)];
        fronts[0].stockpile = fronts[0].capacity;
        let access = assign(&mut fronts, 3, 1, false);
        assert_eq!(access, 0);
        assert_eq!(fronts[0].crew, 0);
        assert_eq!(fronts[0].blocker, "Work-face stockpile full");
        assert_eq!(fronts[1].crew, 3);
        assert_eq!(fronts.iter().map(|front| front.haulers).sum::<u32>(), 1);
    }

    #[test]
    fn extraction_clearance_fits_worker_without_thin_suspended_strips() {
        let cells = clearance_cells([256, 200]);
        assert_eq!(cells.len(), 56);
        for x in 253..=259 {
            let mut column: Vec<_> = cells.iter().filter(|cell| cell[0] == x).collect();
            column.sort_by_key(|cell| cell[1]);
            assert_eq!(column.len(), 8);
            assert_eq!(column.first().unwrap()[1], 193);
            assert_eq!(column.last().unwrap()[1], 200);
        }
    }
}
