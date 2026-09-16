use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Crew {
    pub diggers: u32,
    pub haulers: u32,
    pub operators: u32,
    pub engineers: u32,
    pub prospectors: u32,
    pub reclaimers: u32,
}
impl Crew {
    pub fn prioritise(total: u32, levels: &BTreeMap<String, u32>, priority: &str) -> Self {
        let mut crew = Self::assign(total, levels);
        if priority == "digging" {
            let extra = crew.haulers.saturating_sub(1);
            crew.haulers -= extra;
            crew.diggers += extra;
            return crew;
        }
        let extra = (crew.diggers.saturating_sub(1) + 1) / 2;
        let target = match priority {
            "hauling" => Some(&mut crew.haulers),
            "engineering" if levels.get("supports").copied().unwrap_or(0) > 0 => {
                Some(&mut crew.engineers)
            }
            "prospecting" if levels.get("survey").copied().unwrap_or(0) > 0 => {
                Some(&mut crew.prospectors)
            }
            "refining" if levels.get("furnace").copied().unwrap_or(0) > 0 => {
                Some(&mut crew.operators)
            }
            "reclaiming" if levels.get("reclaimer").copied().unwrap_or(0) > 0 => {
                Some(&mut crew.reclaimers)
            }
            _ => None,
        };
        if let Some(role) = target {
            *role += extra;
            crew.diggers -= extra;
        }
        crew
    }
    pub fn assign(total: u32, levels: &BTreeMap<String, u32>) -> Self {
        let mut c = Self {
            haulers: (total / 4).max(1),
            ..Self::default()
        };
        let mut left = total.saturating_sub(c.haulers);
        for (role, building) in [
            (&mut c.operators, "furnace"),
            (&mut c.engineers, "supports"),
            (&mut c.prospectors, "survey"),
            (&mut c.reclaimers, "reclaimer"),
        ] {
            if left > 1 && levels.get(building).copied().unwrap_or(0) > 0 {
                *role = 1;
                left -= 1;
            }
        }
        c.diggers = left;
        c
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Leg {
    pub from: [i32; 2],
    pub to: [i32; 2],
    pub mode: String,
    pub milliseconds: u32,
}
/// Existing systems serve appropriate segments rather than being replaced globally.
pub fn route(
    path: &[[u32; 2]],
    levels: &BTreeMap<String, u32>,
    terrain_factor: f64,
) -> (Vec<Leg>, u32) {
    if path.is_empty() {
        return (vec![], 2);
    }
    let mut points: Vec<[i32; 2]> = path.iter().map(|p| [p[0] as i32, p[1] as i32]).collect();
    let exit = *points.last().unwrap();
    points.push([exit[0], -3]);
    points.push([72, -3]);
    let level = |id: &str| levels.get(id).copied().unwrap_or(0);
    let mut legs = Vec::new();
    for pair in points.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        let vertical = from[0] == to[0];
        let surface = from[1] < 0 && to[1] < 0;
        let (kind, building, speed) = if vertical && level("shaft") > 0 {
            ("lift", "shaft", 30.)
        } else if surface && level("conveyor") > 0 {
            ("conveyor", "conveyor", 35.)
        } else if !vertical && !surface && level("train") > 0 {
            ("train", "train", 100.)
        } else if !vertical && !surface && level("minecart") > 0 {
            ("minecart", "minecart", 55.)
        } else if !vertical && level("conveyor") > 0 {
            ("conveyor", "conveyor", 35.)
        } else if level("wheelbarrow") > 0 {
            ("wheelbarrow", "wheelbarrow", 18.)
        } else {
            ("carrying", "", 10.)
        };
        let distance = (from[0].abs_diff(to[0]) + from[1].abs_diff(to[1])) as f64 * 2.;
        let factor = if surface { 1. } else { terrain_factor };
        let milliseconds = (1000. * distance * factor
            / (speed * (1. + 0.12 * level(building) as f64)))
            .ceil()
            .max(1.) as u32;
        legs.push(Leg {
            from,
            to,
            mode: kind.into(),
            milliseconds,
        });
    }
    let duration = (2000 + legs.iter().map(|leg| leg.milliseconds).sum::<u32>()).div_ceil(1000);
    (legs, duration)
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Shipment {
    pub material: usize,
    pub amount: u64,
    pub remaining: u32,
    pub duration: u32,
    pub depth: u32,
    pub mode: String,
    #[serde(default)]
    pub path: Vec<[u32; 2]>,
    #[serde(default)]
    pub legs: Vec<Leg>,
}
pub fn mode(levels: &BTreeMap<String, u32>) -> (&'static str, u32) {
    for (key, name, speed) in [
        ("train", "train", 100),
        ("minecart", "minecart", 55),
        ("conveyor", "conveyor", 35),
        ("wheelbarrow", "wheelbarrow", 18),
    ] {
        if levels.get(key).copied().unwrap_or(0) > 0 {
            return (name, speed);
        }
    }
    ("carrying", 10)
}
pub fn arrive(shipments: &mut Vec<Shipment>, hauled: &mut BTreeMap<usize, u64>, capacity: u64) {
    arrive_at_speed(shipments, hauled, capacity, 1);
}
pub fn arrive_at_speed(
    shipments: &mut Vec<Shipment>,
    hauled: &mut BTreeMap<usize, u64>,
    capacity: u64,
    speed: u32,
) {
    let mut space = capacity.saturating_sub(hauled.values().sum());
    for s in shipments.iter_mut() {
        s.remaining = s.remaining.saturating_sub(speed);
        if s.remaining == 0 {
            let n = space.min(s.amount);
            *hauled.entry(s.material).or_default() += n;
            s.amount -= n;
            space -= n;
        }
    }
    shipments.retain(|s| s.amount > 0);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roles_never_create_or_lose_workers() {
        for n in 3..200 {
            let levels = [
                ("furnace".into(), 1),
                ("supports".into(), 1),
                ("survey".into(), 1),
                ("reclaimer".into(), 1),
            ]
            .into_iter()
            .collect();
            let c = Crew::assign(n, &levels);
            assert_eq!(
                n,
                c.diggers + c.haulers + c.operators + c.engineers + c.prospectors + c.reclaimers
            );
            assert!(c.diggers > 0)
        }
    }
    #[test]
    fn priorities_preserve_population_and_a_working_front() {
        let levels = [("furnace".into(), 1), ("reclaimer".into(), 1)]
            .into_iter()
            .collect();
        for total in 3..100 {
            for priority in ["balanced", "digging", "hauling", "refining", "reclaiming"] {
                let c = Crew::prioritise(total, &levels, priority);
                assert_eq!(
                    total,
                    c.diggers
                        + c.haulers
                        + c.operators
                        + c.engineers
                        + c.prospectors
                        + c.reclaimers
                );
                assert!(c.diggers > 0 && c.haulers > 0);
            }
        }
    }
    #[test]
    fn transport_supplements_route_segments() {
        let levels = [
            ("shaft".into(), 1),
            ("minecart".into(), 1),
            ("conveyor".into(), 1),
        ]
        .into_iter()
        .collect();
        let (legs, duration) = route(&[[40, 30], [32, 30], [32, 0]], &levels, 1.);
        assert_eq!(legs[0].mode, "minecart");
        assert_eq!(legs[1].mode, "lift");
        assert_eq!(legs.last().unwrap().mode, "conveyor");
        assert!(duration >= 2);
        assert_eq!(legs.windows(2).filter(|p| p[0].to != p[1].from).count(), 0);
    }
    #[test]
    fn blocked_arrivals_preserve_cargo() {
        let mut shipments = vec![Shipment {
            material: 2,
            amount: 1000,
            remaining: 1,
            duration: 1,
            depth: 100,
            mode: "carrying".into(),
            path: vec![],
            legs: vec![],
        }];
        let mut hauled = BTreeMap::new();
        arrive(&mut shipments, &mut hauled, 250);
        assert_eq!(hauled[&2], 250);
        assert_eq!(shipments[0].amount, 750);
        hauled.clear();
        arrive(&mut shipments, &mut hauled, 1000);
        assert!(shipments.is_empty());
        assert_eq!(hauled[&2], 750);
    }
}
