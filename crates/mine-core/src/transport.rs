//! Material ownership moves through finite station buffers and vehicle batches.
use crate::{
    geometry::UNITS,
    logistics::{order_cargo, Leg},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Serialize, Deserialize)]
pub struct Station {
    pub id: String,
    pub name: String,
    pub level: u32,
    pub capacity: u64,
    pub cargo: BTreeMap<usize, u64>,
    pub preferred: bool,
    pub incoming: u64,
    pub outgoing: u64,
    pub quote: String,
    #[serde(default)]
    pub routing: Vec<RoutedCargo>,
}
impl Station {
    pub fn stored(&self) -> u64 {
        self.cargo.values().sum()
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct RoutedCargo {
    pub material: usize,
    pub amount: u64,
    pub route: u64,
}
fn put(lots: &mut Vec<RoutedCargo>, material: usize, amount: u64, route: u64) {
    if amount == 0 {
        return;
    }
    if let Some(lot) = lots
        .iter_mut()
        .find(|lot| lot.material == material && lot.route == route)
    {
        lot.amount += amount;
    } else {
        lots.push(RoutedCargo {
            material,
            amount,
            route,
        });
    }
}
fn take(lots: &mut Vec<RoutedCargo>, material: usize, mut amount: u64) -> Vec<RoutedCargo> {
    let mut result = vec![];
    for lot in lots.iter_mut().filter(|l| l.material == material) {
        let n = lot.amount.min(amount);
        lot.amount -= n;
        amount -= n;
        if n > 0 {
            result.push(RoutedCargo {
                material,
                amount: n,
                route: lot.route,
            });
        }
        if amount == 0 {
            break;
        }
    }
    lots.retain(|l| l.amount > 0);
    result
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Batch {
    #[serde(default)]
    pub route: u64,
    pub material: usize,
    pub amount: u64,
    pub remaining_ms: u32,
    pub duration_ms: u32,
    pub legs: Vec<Leg>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Segment {
    pub name: String,
    pub batches: Vec<Batch>,
    pub capacity: u64,
    pub duration_ms: u32,
    pub legs: Vec<Leg>,
    pub rate: u64,
    pub blocked: bool,
    #[serde(default)]
    pub blocker: String,
    pub demand: u32,
    pub utilisation: f64,
    pub time_fraction: u64,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Network {
    #[serde(default)]
    pub routes: BTreeMap<u64, Vec<Vec<Leg>>>,
    #[serde(default)]
    pub source: Vec<RoutedCargo>,
    #[serde(default)]
    pub current_route: u64,
    #[serde(skip)]
    configured_legs: Option<Vec<Leg>>,
    pub stations: Vec<Station>,
    pub segments: Vec<Segment>,
    pub express: usize,
}
#[derive(Clone, Serialize)]
pub struct VisualCargo {
    pub material: usize,
    pub amount: u64,
    pub remaining: f64,
    pub duration: f64,
    pub mode: String,
    pub depth: u32,
    pub path: Vec<[i64; 2]>,
    pub legs: Vec<Leg>,
}
impl Default for Network {
    fn default() -> Self {
        let stations = [
            "Loading bay",
            "Underground transfer",
            "Shaft transfer",
            "Surface depot",
            "Processing intake",
        ]
        .into_iter()
        .enumerate()
        .map(|(index, name)| Station {
            id: index.to_string(),
            name: name.into(),
            level: 0,
            capacity: crate::pacing::get().station_units[index] * UNITS,
            cargo: BTreeMap::new(),
            preferred: false,
            incoming: 0,
            outgoing: 0,
            quote: "60".into(),
            routing: vec![],
        })
        .collect();
        let segments = [
            "Short haul",
            "Underground freight",
            "Shaft service",
            "Surface dispatch",
        ]
        .into_iter()
        .map(|name| Segment {
            name: name.into(),
            batches: vec![],
            capacity: crate::pacing::get().transit_units * UNITS,
            duration_ms: 2000,
            legs: vec![],
            rate: UNITS,
            blocked: false,
            blocker: String::new(),
            demand: 0,
            utilisation: 0.,
            time_fraction: 0,
        })
        .collect();
        Self {
            routes: BTreeMap::new(),
            source: vec![],
            current_route: 0,
            configured_legs: None,
            stations,
            segments,
            express: 0,
        }
    }
}
impl Network {
    pub fn configure(&mut self, legs: &[Leg], rate: u64, global_capacity: u32) {
        for (i, station) in self.stations.iter_mut().enumerate() {
            station.capacity = (crate::pacing::get().station_units[i] * UNITS
                + 5 * UNITS * global_capacity as u64)
                * (4 + station.level as u64)
                / 4;
            station.quote = (crate::pacing::get().station_cost as f64
                * crate::pacing::get()
                    .capacity_growth
                    .powi(station.level as i32))
            .ceil()
            .to_string();
        }
        for segment in &mut self.segments {
            segment.rate = rate;
            segment.capacity =
                (crate::pacing::get().transit_units + global_capacity as u64 * 5) * UNITS;
        }
        if self.configured_legs.as_deref() == Some(legs)
            && self.routes.contains_key(&self.current_route)
        {
            return;
        }
        self.configured_legs = Some(legs.to_vec());
        for segment in &mut self.segments {
            segment.legs.clear();
            segment.demand = 0;
        }
        let underground = legs
            .first()
            .is_some_and(|leg| leg.from[1] >= crate::geometry::PIT_ROWS as i32);
        let mut reached_shaft = false;
        let mut loaded = false;
        for leg in legs {
            let surface = leg.from[1] < 0 && leg.to[1] < 0;
            let vertical = leg.from[0] == leg.to[0];
            let index = if surface {
                3
            } else if underground {
                if !reached_shaft && !vertical && leg.from[1] >= crate::geometry::PIT_ROWS as i32 {
                    1
                } else {
                    reached_shaft = true;
                    2
                }
            } else if leg.to[1] < 0 {
                2
            } else {
                0
            };
            let mut cargo_leg = leg.clone();
            if index == 1 && !loaded {
                loaded = true;
                let distance = leg.from[0].abs_diff(leg.to[0]);
                if distance > 8 && leg.milliseconds > 1 {
                    let dx = (leg.to[0] - leg.from[0]).signum() * 8;
                    let point = [leg.from[0] + dx, leg.from[1]];
                    let time = (leg.milliseconds as u64 * 8 / distance as u64)
                        .max(1)
                        .min((leg.milliseconds - 1) as u64) as u32;
                    let mut first = leg.clone();
                    first.to = point;
                    first.milliseconds = time;
                    self.segments[0].legs.push(first);
                    cargo_leg.from = point;
                    cargo_leg.milliseconds -= time;
                }
            }
            self.segments[index].legs.push(cargo_leg);
        }
        for segment in &mut self.segments {
            segment.demand = u32::from(
                segment
                    .legs
                    .iter()
                    .any(|l| matches!(l.mode.as_str(), "conveyor" | "train" | "lift")),
            );
        }
        for segment in &mut self.segments {
            segment.duration_ms = 2000 + segment.legs.iter().map(|l| l.milliseconds).sum::<u32>();
            segment.capacity =
                (crate::pacing::get().transit_units + global_capacity as u64 * 5) * UNITS;
        }
        let itinerary: Vec<_> = self.segments.iter().map(|s| s.legs.clone()).collect();
        if self.routes.get(&self.current_route) != Some(&itinerary) {
            self.current_route += 1;
            self.routes.insert(self.current_route, itinerary);
        }
    }
    pub fn register_source(&mut self, material: usize, amount: u64) {
        put(&mut self.source, material, amount, self.current_route);
    }
    pub fn mass(&self) -> u64 {
        self.stations.iter().map(Station::stored).sum::<u64>()
            + self
                .segments
                .iter()
                .flat_map(|s| &s.batches)
                .map(|b| b.amount)
                .sum::<u64>()
    }
    pub fn visual(&self) -> Vec<VisualCargo> {
        self.segments
            .iter()
            .flat_map(|s| &s.batches)
            .filter(|b| !b.legs.is_empty())
            .map(|b| VisualCargo {
                material: b.material,
                amount: b.amount,
                remaining: b.remaining_ms as f64 / 1000.,
                duration: b.duration_ms as f64 / 1000.,
                mode: b
                    .legs
                    .first()
                    .map(|l| l.mode.clone())
                    .unwrap_or_else(|| "carrying".into()),
                depth: b
                    .legs
                    .first()
                    .map(|l| crate::geometry::depth(l.from[1].max(0) as i64))
                    .unwrap_or(0),
                path: vec![],
                legs: b.legs.clone(),
            })
            .collect()
    }
    /// One 50ms authoritative step. A blocked vehicle keeps ownership of its cargo.
    pub fn tick(
        &mut self,
        tick: u64,
        ore: &mut BTreeMap<usize, u64>,
        output: &mut BTreeMap<usize, u64>,
        output_cap: u64,
        preferred: &[usize],
        global_priority: bool,
        boost: bool,
        power_permille: u64,
    ) -> u64 {
        for (&id, &q) in ore.iter().filter(|(_, q)| **q > 0) {
            let tracked: u64 = self
                .source
                .iter()
                .filter(|l| l.material == id)
                .map(|l| l.amount)
                .sum();
            if q > tracked {
                put(&mut self.source, id, q - tracked, self.current_route);
            }
        }
        for station in &mut self.stations {
            for (&id, &q) in station.cargo.iter().filter(|(_, q)| **q > 0) {
                let tracked: u64 = station
                    .routing
                    .iter()
                    .filter(|l| l.material == id)
                    .map(|l| l.amount)
                    .sum();
                if q > tracked {
                    put(&mut station.routing, id, q - tracked, self.current_route);
                }
            }
        }
        if tick % 20 == 1 {
            for s in &mut self.stations {
                s.incoming = 0;
                s.outgoing = 0;
            }
        }
        // Drain processing intake before advancing upstream arrivals.
        let last = self.stations.len() - 1;
        let mut room = output_cap.saturating_sub(output.values().sum());
        let mut dispatched = 0;
        let destination = &mut self.stations[last];
        for (&id, q) in &mut destination.cargo {
            let n = (*q).min(room);
            *q -= n;
            *output.entry(id).or_default() += n;
            room -= n;
            dispatched += n;
            take(&mut destination.routing, id, n);
        }
        self.stations[last].outgoing += dispatched;
        for index in (0..self.segments.len()).rev() {
            let segment = &mut self.segments[index];
            let station = &mut self.stations[index + 1];
            let mut space = station.capacity.saturating_sub(station.stored());
            let speed = if boost && self.express == index { 2 } else { 1 };
            segment.time_fraction += 50
                * speed
                * if segment.demand > 0 {
                    power_permille
                } else {
                    1000
                };
            let elapsed = (segment.time_fraction / 1000) as u32;
            segment.time_fraction %= 1000;
            segment.blocked = false;
            segment.blocker.clear();
            for batch in &mut segment.batches {
                batch.remaining_ms = batch.remaining_ms.saturating_sub(elapsed);
                if batch.remaining_ms == 0 {
                    let n = batch.amount.min(space);
                    batch.amount -= n;
                    space -= n;
                    *station.cargo.entry(batch.material).or_default() += n;
                    station.incoming += n;
                    put(&mut station.routing, batch.material, n, batch.route);
                    segment.blocked |= batch.amount > 0;
                    if batch.amount > 0 {
                        segment.blocker = format!("{} full", station.name);
                    }
                }
            }
            segment.batches.retain(|b| b.amount > 0);
        }
        let mut space = self.stations[0]
            .capacity
            .saturating_sub(self.stations[0].stored());
        let initial = space;
        let mut ids: Vec<_> = ore
            .iter()
            .filter(|(_, q)| **q > 0)
            .map(|(&id, _)| id)
            .collect();
        order_cargo(
            &mut ids,
            tick,
            preferred,
            global_priority || self.stations[0].preferred,
        );
        for id in ids {
            let q = ore.get_mut(&id).unwrap();
            let n = (*q).min(space);
            *q -= n;
            space -= n;
            *self.stations[0].cargo.entry(id).or_default() += n;
            for lot in take(&mut self.source, id, n) {
                put(&mut self.stations[0].routing, id, lot.amount, lot.route);
            }
        }
        self.stations[0].incoming += initial - space;
        for index in (0..self.segments.len()).rev() {
            let station = &mut self.stations[index];
            let segment = &mut self.segments[index];
            let boost = if boost && self.express == index { 2 } else { 1 };
            let used: u64 = segment.batches.iter().map(|b| b.amount).sum();
            let mut budget = (segment.rate * boost / 20).min(segment.capacity.saturating_sub(used));
            let initial_budget = budget;
            let mut ids: Vec<_> = station
                .cargo
                .iter()
                .filter(|(_, q)| **q > 0)
                .map(|(&id, _)| id)
                .collect();
            order_cargo(
                &mut ids,
                tick,
                preferred,
                station.preferred || global_priority,
            );
            for id in ids {
                let n = station.cargo[&id].min(budget);
                if n == 0 {
                    continue;
                }
                let lots = take(&mut station.routing, id, n);
                let mut loaded = 0;
                for lot in lots {
                    let legs = self
                        .routes
                        .get(&lot.route)
                        .and_then(|r| r.get(index))
                        .unwrap_or(&segment.legs);
                    let duration = 2000 + legs.iter().map(|l| l.milliseconds).sum::<u32>();
                    if let Some(batch) = segment.batches.last_mut().filter(|b| {
                        b.material == id
                            && b.route == lot.route
                            && b.duration_ms == duration
                            && b.remaining_ms / 100 == duration / 100
                    }) {
                        batch.amount += lot.amount;
                    } else if segment.batches.len() < 512 {
                        segment.batches.push(Batch {
                            route: lot.route,
                            material: id,
                            amount: lot.amount,
                            remaining_ms: duration,
                            duration_ms: duration,
                            legs: legs.clone(),
                        });
                    } else {
                        put(&mut station.routing, id, lot.amount, lot.route);
                        continue;
                    }
                    loaded += lot.amount;
                }
                *station.cargo.get_mut(&id).unwrap() -= loaded;
                station.outgoing += loaded;
                budget -= loaded;
            }
            segment.utilisation = if initial_budget > 0 {
                (initial_budget - budget) as f64 / initial_budget as f64
            } else {
                0.
            };
            segment.blocked |= used >= segment.capacity || segment.batches.len() >= 512;
            if segment.blocked && segment.blocker.is_empty() {
                segment.blocker = "Vehicle capacity full".into();
            }
        }
        let mut used = std::collections::BTreeSet::from([self.current_route]);
        used.extend(self.source.iter().map(|l| l.route));
        used.extend(
            self.stations
                .iter()
                .flat_map(|s| s.routing.iter().map(|l| l.route)),
        );
        used.extend(
            self.segments
                .iter()
                .flat_map(|s| s.batches.iter().map(|b| b.route)),
        );
        self.routes.retain(|id, _| used.contains(id));
        initial - space
    }
    pub fn sources_match(&self, ore: &BTreeMap<usize, u64>) -> bool {
        routing_matches(&self.source, ore, &self.routes)
    }
    pub fn valid(&self, materials: usize) -> bool {
        self.current_route < 1_000_000_000_000
            && self.source.iter().all(|l| {
                l.material < materials
                    && l.amount > 0
                    && l.amount <= 1_000_000_000_000
                    && self.routes.contains_key(&l.route)
            })
            && self.routes.len() <= 8192
            && self.source.len() <= 32768
            && self
                .routes
                .values()
                .all(|r| r.len() == 4 && r.iter().all(|legs| valid_legs(legs)))
            && self.stations.iter().all(|s| {
                routing_matches(&s.routing, &s.cargo, &self.routes)
                    && s.routing.len() <= 32768
                    && s.routing.iter().all(|l| {
                        l.material < materials
                            && l.amount <= 1_000_000_000_000
                            && self.routes.contains_key(&l.route)
                    })
            })
            && self.stations.len() == 5
            && self.segments.len() == 4
            && self.express < 4
            && self.stations.iter().enumerate().all(|(index, s)| {
                s.id == index.to_string()
                    && s.name.len() <= 64
                    && s.quote.len() <= 32
                    && s.quote.parse::<u64>().is_ok_and(|quote| quote > 0)
                    && s.level <= 50
                    && s.capacity <= 1_000_000_000_000
                    && s.cargo
                        .iter()
                        .all(|(id, q)| *id < materials && *q <= 1_000_000_000_000)
            })
            && self.segments.iter().all(|s| {
                s.name.len() <= 64
                    && s.capacity <= 1_000_000_000_000
                    && s.rate <= 1_000_000_000_000
                    && s.duration_ms > 0
                    && s.duration_ms <= 100_000_000
                    && s.demand <= 1
                    && s.utilisation.is_finite()
                    && (0. ..=1.).contains(&s.utilisation)
                    && valid_legs(&s.legs)
                    && s.batches.len() <= 512
                    && s.time_fraction < 1000
                    && s.batches.iter().all(|b| {
                        b.material < materials
                            && self.routes.contains_key(&b.route)
                            && b.amount <= 1_000_000_000_000
                            && b.remaining_ms <= b.duration_ms
                            && b.duration_ms > 0
                            && b.duration_ms <= 100_000_000
                            && valid_legs(&b.legs)
                    })
            })
    }
}
fn routing_matches(
    lots: &[RoutedCargo],
    cargo: &BTreeMap<usize, u64>,
    routes: &BTreeMap<u64, Vec<Vec<Leg>>>,
) -> bool {
    let mut totals = BTreeMap::<usize, u64>::new();
    for lot in lots {
        if lot.amount == 0 || lot.amount > 1_000_000_000_000 || !routes.contains_key(&lot.route) {
            return false;
        }
        let entry = totals.entry(lot.material).or_default();
        let Some(next) = entry.checked_add(lot.amount) else {
            return false;
        };
        *entry = next;
    }
    cargo
        .iter()
        .all(|(id, q)| totals.get(id).copied().unwrap_or(0) == *q)
        && totals.keys().all(|id| cargo.contains_key(id))
}
fn valid_legs(legs: &[Leg]) -> bool {
    legs.len() <= 100_000
        && legs.iter().all(|leg| {
            leg.milliseconds > 0
                && leg.milliseconds <= 100_000_000
                && matches!(
                    leg.mode.as_str(),
                    "carrying" | "wheelbarrow" | "conveyor" | "minecart" | "train" | "lift"
                )
                && [leg.from, leg.to].iter().all(|p| {
                    (-crate::geometry::MAX_X..crate::geometry::MAX_X).contains(&(p[0] as i64))
                        && (-24..crate::geometry::MAX_ROWS as i32).contains(&p[1])
                })
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expanded_signed_routes_survive_save_validation() {
        let (legs, _) = crate::logistics::route(
            &[[-2048, 256], [4096, 256], [256, 256], [256, 0]],
            &BTreeMap::new(),
            1.,
        );
        let mut network = Network::default();
        network.configure(&legs, UNITS, 1);
        assert!(network.valid(crate::materials().len()));
        let restored: Network =
            serde_json::from_str(&serde_json::to_string(&network).unwrap()).unwrap();
        assert!(restored.valid(crate::materials().len()));
        let mut invalid = restored;
        invalid.segments[0].legs.push(Leg {
            from: [crate::geometry::MAX_X as i32, 0],
            to: [256, 0],
            mode: "carrying".into(),
            milliseconds: 1000,
        });
        assert!(!invalid.valid(crate::materials().len()));
    }
    #[test]
    fn cached_configuration_matches_rebuilding_through_upgrades_and_reload() {
        let mut cached = Network::default();
        let mut rebuilt = cached.clone();
        let mut ore_a = BTreeMap::from([(3, 20 * UNITS), (4, 10 * UNITS)]);
        let mut ore_b = ore_a.clone();
        let mut out_a = BTreeMap::new();
        let mut out_b = BTreeMap::new();
        for tick in 1..=240 {
            let legs = [Leg {
                from: [100 + tick / 80 * 16, 200],
                to: [256, 200],
                mode: "conveyor".into(),
                milliseconds: 3000,
            }];
            if tick == 100 {
                cached.stations[1].level = 2;
                rebuilt.stations[1].level = 2;
            }
            if tick == 150 {
                cached = serde_json::from_str(&serde_json::to_string(&cached).unwrap()).unwrap();
            }
            rebuilt.configured_legs = None;
            let rate = UNITS * (1 + tick as u64 / 60);
            let capacity = (tick / 60) as u32;
            cached.configure(&legs, rate, capacity);
            rebuilt.configure(&legs, rate, capacity);
            for (network, ore, out) in [
                (&mut cached, &mut ore_a, &mut out_a),
                (&mut rebuilt, &mut ore_b, &mut out_b),
            ] {
                network.tick(
                    tick as u64,
                    ore,
                    out,
                    100 * UNITS,
                    &[4],
                    true,
                    tick > 180,
                    750,
                );
            }
            assert_eq!(
                serde_json::to_value(&cached).unwrap(),
                serde_json::to_value(&rebuilt).unwrap(),
                "tick {tick}"
            );
            assert_eq!(ore_a, ore_b);
            assert_eq!(out_a, out_b);
        }
    }
    #[test]
    fn import_rejects_unknown_routes_and_unlabelled_station_material() {
        let mut n = Network::default();
        n.configure(&[], UNITS, 0);
        n.tick(
            1,
            &mut BTreeMap::from([(3, 1000)]),
            &mut BTreeMap::new(),
            UNITS,
            &[],
            false,
            false,
            1000,
        );
        assert!(n.valid(56));
        let route = n.segments[0].batches[0].route;
        n.segments[0].batches[0].route = u64::MAX;
        assert!(!n.valid(56));
        n.segments[0].batches[0].route = route;
        n.stations[0].cargo.insert(3, 1000);
        assert!(!n.valid(56));
    }
    #[test]
    fn cargo_keeps_its_branch_after_active_route_changes() {
        let mut n = Network::default();
        let old = Leg {
            from: [100, 200],
            to: [120, 200],
            mode: "carrying".into(),
            milliseconds: 200,
        };
        n.configure(&[old.clone()], UNITS, 0);
        let original = n.current_route;
        let mut ore = BTreeMap::from([(3, UNITS)]);
        let mut out = BTreeMap::new();
        n.tick(1, &mut ore, &mut out, 100 * UNITS, &[], false, false, 1000);
        n.configure(
            &[Leg {
                from: [300, 300],
                to: [256, 300],
                mode: "minecart".into(),
                milliseconds: 500,
            }],
            UNITS,
            0,
        );
        assert_ne!(n.current_route, original);
        let saved = serde_json::to_string(&n).unwrap();
        let mut n: Network = serde_json::from_str(&saved).unwrap();
        for tick in 2..500 {
            n.tick(
                tick,
                &mut ore,
                &mut out,
                100 * UNITS,
                &[],
                false,
                false,
                1000,
            );
            for batch in n.segments.iter().flat_map(|s| &s.batches) {
                assert_eq!(batch.route, original);
            }
        }
        assert_eq!(out[&3], UNITS);
        assert_eq!(n.mass(), 0);
        assert!(!n.routes.contains_key(&original));
    }
    #[test]
    fn imported_quotes_are_safe_for_integer_display_and_affordability() {
        let mut network = Network::default();
        assert!(network.valid(56));
        for quote in ["", "NaN", "-1", "2.5", "0", "18446744073709551616"] {
            network.stations[0].quote = quote.into();
            assert!(!network.valid(56), "Reject malformed quote {quote:?}");
        }
    }
    #[test]
    fn buffers_block_upstream_split_arrivals_and_conserve_material() {
        let mut n = Network::default();
        let mut ore = BTreeMap::from([(3, 100 * UNITS)]);
        let mut output = BTreeMap::new();
        n.configure(&[], UNITS, 0);
        for tick in 1..400 {
            n.tick(tick, &mut ore, &mut output, UNITS, &[3], true, false, 1000);
        }
        assert_eq!(output[&3], UNITS);
        assert!(n.stations.iter().any(|s| s.stored() > 0));
        assert_eq!(
            ore.values().sum::<u64>() + output.values().sum::<u64>() + n.mass(),
            100 * UNITS
        );
        let bytes = serde_json::to_string(&n).unwrap();
        let mut restored: Network = serde_json::from_str(&bytes).unwrap();
        for tick in 400..600 {
            restored.tick(
                tick,
                &mut ore,
                &mut output,
                100 * UNITS,
                &[3],
                false,
                false,
                1000,
            );
        }
        assert!(output[&3] > UNITS);
        assert_eq!(
            ore.values().sum::<u64>() + output.values().sum::<u64>() + restored.mass(),
            100 * UNITS
        );
    }
    #[test]
    fn preferred_cargo_cannot_starve_other_materials() {
        let mut network = Network::default();
        network.configure(&[], UNITS, 0);
        let mut ore = BTreeMap::from([(3, 1000 * UNITS), (4, 1000 * UNITS)]);
        let mut output = BTreeMap::new();
        for tick in 1..=1200 {
            network.tick(
                tick,
                &mut ore,
                &mut output,
                2000 * UNITS,
                &[3],
                true,
                false,
                1000,
            );
        }
        assert!(ore[&3] > 0, "preferred demand stays continuous");
        assert!(output.get(&3).copied().unwrap_or(0) > 0);
        assert!(
            output.get(&4).copied().unwrap_or(0) >= UNITS,
            "fair service reaches final intake"
        );
        assert_eq!(
            ore.values().sum::<u64>() + output.values().sum::<u64>() + network.mass(),
            2000 * UNITS
        );
    }
    #[test]
    fn express_only_accelerates_selected_segment() {
        let mut n = Network::default();
        n.express = 2;
        for s in &mut n.segments {
            s.batches.push(Batch {
                route: 0,
                material: 0,
                amount: UNITS,
                remaining_ms: 1000,
                duration_ms: 1000,
                legs: vec![],
            });
        }
        n.tick(
            1,
            &mut BTreeMap::new(),
            &mut BTreeMap::new(),
            0,
            &[],
            false,
            true,
            1000,
        );
        assert_eq!(n.segments[0].batches[0].remaining_ms, 950);
        assert_eq!(n.segments[2].batches[0].remaining_ms, 900);
    }
    #[test]
    fn underground_handoffs_preserve_physical_route_order() {
        let path = [[128, 295], [256, 295], [256, 191], [207, 191], [16, 0]];
        let levels = BTreeMap::from([
            ("shaft".into(), 1),
            ("minecart".into(), 1),
            ("conveyor".into(), 1),
        ]);
        let (legs, _) = crate::logistics::route(&path, &levels, 1.);
        let mut network = Network::default();
        network.configure(&legs, UNITS, 0);
        let ordered: Vec<_> = network
            .segments
            .iter()
            .flat_map(|s| s.legs.iter())
            .collect();
        assert_eq!(ordered.first().unwrap().from, legs.first().unwrap().from);
        assert_eq!(ordered.last().unwrap().to, legs.last().unwrap().to);
        assert!(ordered.windows(2).all(|pair| pair[0].to == pair[1].from));
        assert_eq!(
            ordered.iter().map(|l| l.milliseconds).sum::<u32>(),
            legs.iter().map(|l| l.milliseconds).sum::<u32>()
        );
    }
}
