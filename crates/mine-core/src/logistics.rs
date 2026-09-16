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
pub struct Shipment {
    pub material: usize,
    pub amount: u64,
    pub remaining: u32,
    pub duration: u32,
    pub depth: u32,
    pub mode: String,
    #[serde(default)]
    pub path: Vec<[u32; 2]>,
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
    let mut space = capacity.saturating_sub(hauled.values().sum());
    for s in shipments.iter_mut() {
        s.remaining = s.remaining.saturating_sub(1);
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
    fn blocked_arrivals_preserve_cargo() {
        let mut shipments = vec![Shipment {
            material: 2,
            amount: 1000,
            remaining: 1,
            duration: 1,
            depth: 100,
            mode: "carrying".into(),
            path: vec![],
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
