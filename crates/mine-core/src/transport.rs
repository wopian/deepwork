//! Material ownership moves through finite station buffers and vehicle batches.
use crate::{
    geometry::UNITS,
    logistics::{order_cargo, Leg},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
const LOADING_MS: u32 = 1000;
const MAX_ROUTES: usize = 1024;
const MAX_ROUTE_LEGS: usize = 250_000;
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
    pub id: u64,
    #[serde(default)]
    pub route: u64,
    pub material: usize,
    pub amount: u64,
    pub remaining_ms: u32,
    pub duration_ms: u32,
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
    #[serde(default)]
    next_route: u64,
    #[serde(skip)]
    configured_legs: Option<Vec<Leg>>,
    #[serde(skip)]
    configured_rate: u64,
    #[serde(skip)]
    configured_capacity: u32,
    #[serde(skip)]
    protected_routes: BTreeSet<u64>,
    pub stations: Vec<Station>,
    pub segments: Vec<Segment>,
    pub express: usize,
    #[serde(default)]
    pub next_batch: u64,
}
#[derive(Clone, Serialize)]
pub struct VisualCargo {
    pub id: String,
    pub route: u64,
    pub elapsed_ms: i64,
    pub speed: f64,
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
            duration_ms: LOADING_MS,
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
            next_route: 1,
            configured_legs: None,
            configured_rate: 0,
            configured_capacity: 0,
            protected_routes: BTreeSet::new(),
            stations,
            segments,
            express: 0,
            next_batch: 0,
        }
    }
}
impl Network {
    pub fn initialise_batch_ids(&mut self) {
        self.next_batch = self.next_batch.max(
            self.segments
                .iter()
                .flat_map(|s| &s.batches)
                .map(|b| b.id)
                .max()
                .unwrap_or(0),
        );
        for batch in self.segments.iter_mut().flat_map(|s| &mut s.batches) {
            if batch.id == 0 {
                self.next_batch += 1;
                batch.id = self.next_batch;
            }
        }
    }

    pub fn configure(&mut self, legs: &[Leg], rate: u64, global_capacity: u32) {
        for (i, station) in self.stations.iter_mut().enumerate() {
            let seconds = [20, 30, 30, 30, 60][i];
            station.capacity = (crate::pacing::get().station_units[i] * UNITS)
                .max(rate.saturating_mul(seconds))
                .saturating_mul(4 + global_capacity as u64 + station.level as u64)
                / 4;
            station.quote = (crate::pacing::get().station_cost as f64
                * crate::pacing::get()
                    .capacity_growth
                    .powi(station.level as i32))
            .ceil()
            .to_string();
        }
        if self.configured_legs.as_deref() == Some(legs)
            && self.configured_rate == rate
            && self.configured_capacity == global_capacity
            && self.routes.contains_key(&self.current_route)
        {
            return;
        }
        self.configured_legs = Some(legs.to_vec());
        self.configured_rate = rate;
        self.configured_capacity = global_capacity;
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
                    .any(|l| matches!(l.mode.as_str(), "train" | "lift")),
            );
        }
        for segment in &mut self.segments {
            segment.duration_ms =
                LOADING_MS + segment.legs.iter().map(|l| l.milliseconds).sum::<u32>();
            // Installed transport modes carry only their matching segment. Long
            // routes receive enough in-transit ownership for 30% headroom over
            // one full cycle instead of retaining a fixed shallow-mine cap.
            let floor = segment
                .legs
                .iter()
                .map(|leg| match leg.mode.as_str() {
                    "train" => 12 * UNITS,
                    "minecart" | "lift" => 8 * UNITS,
                    "conveyor" => 4 * UNITS,
                    "wheelbarrow" => 2 * UNITS,
                    _ => UNITS,
                })
                .min()
                .unwrap_or(rate);
            segment.rate = rate.max(floor);
            let cycle = segment
                .rate
                .saturating_mul(segment.duration_ms as u64)
                .div_ceil(1000);
            segment.capacity = (crate::pacing::get().transit_units * UNITS)
                .max(cycle.saturating_mul(13).div_ceil(10))
                .saturating_mul(4 + global_capacity as u64)
                / 4;
        }
        let itinerary: Vec<_> = self.segments.iter().map(|s| s.legs.clone()).collect();
        if self.routes.get(&self.current_route) != Some(&itinerary) {
            if let Some((&route, _)) = self.routes.iter().find(|(_, saved)| *saved == &itinerary) {
                self.current_route = route;
            } else {
                self.next_route = self.next_route.max(
                    self.routes
                        .keys()
                        .next_back()
                        .copied()
                        .unwrap_or(0)
                        .saturating_add(1),
                );
                self.current_route = self.next_route;
                self.next_route = self.next_route.saturating_add(1);
                self.routes.insert(self.current_route, itinerary);
            }
        }
    }
    pub fn register_source(&mut self, material: usize, amount: u64) {
        put(&mut self.source, material, amount, self.current_route);
    }
    pub fn register_source_route(&mut self, material: usize, amount: u64, route: u64) {
        if self.routes.contains_key(&route) {
            put(&mut self.source, material, amount, route);
        }
    }
    pub fn protect_routes(&mut self, routes: impl IntoIterator<Item = u64>) {
        self.protected_routes.clear();
        self.protected_routes.extend(
            routes
                .into_iter()
                .filter(|route| self.routes.contains_key(route)),
        );
    }
    pub fn has_route(&self, route: u64) -> bool {
        self.routes.contains_key(&route)
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
    /// Idle hauling uses finite flow budgets, without spawning or moving cargo actors.
    /// Existing transit ownership drains first. Remaining batches keep their identities.
    pub fn advance_bulk(
        &mut self,
        tick: u64,
        ticks: u64,
        ore: &mut BTreeMap<usize, u64>,
        output: &mut BTreeMap<usize, u64>,
        output_cap: u64,
        preferred: &[usize],
        global_priority: bool,
        power_permille: u64,
    ) -> u64 {
        let terminal = self.stations.len() - 1;
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
        for station in self.stations.iter_mut().take(terminal) {
            for (&id, &q) in &station.cargo {
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
        for station in &mut self.stations {
            station.incoming = 0;
            station.outgoing = 0;
        }
        let mut budgets: Vec<_> = self
            .segments
            .iter()
            .map(|s| {
                // Transit capacity still bounds sustained throughput over one loading/travel cycle.
                let rate = s
                    .rate
                    .min(s.capacity.saturating_mul(1000) / s.duration_ms.max(1) as u64);
                rate.saturating_mul(ticks) / 20
            })
            .collect();
        let mut powered_budgets: Vec<_> = budgets
            .iter()
            .map(|n| n.saturating_mul(power_permille) / 1000)
            .collect();
        self.drain_intake(output, output_cap);
        // Release cargo already in transit using flow budgets, without advancing leg clocks.
        for index in (0..self.segments.len()).rev() {
            let station = &mut self.stations[index + 1];
            let mut room = station.capacity.saturating_sub(station.stored());
            let segment = &mut self.segments[index];
            for batch in &mut segment.batches {
                let powered = self
                    .routes
                    .get(&batch.route)
                    .and_then(|r| r.get(index))
                    .is_some_and(|legs| {
                        legs.iter()
                            .any(|l| matches!(l.mode.as_str(), "lift" | "train"))
                    });
                if powered && power_permille == 0 {
                    continue;
                }
                let amount = batch.amount.min(room).min(budgets[index]).min(if powered {
                    powered_budgets[index]
                } else {
                    u64::MAX
                });
                batch.amount -= amount;
                room -= amount;
                budgets[index] -= amount;
                if powered {
                    powered_budgets[index] -= amount;
                }
                *station.cargo.entry(batch.material).or_default() += amount;
                station.incoming += amount;
                if index + 1 < terminal {
                    put(&mut station.routing, batch.material, amount, batch.route);
                }
            }
            segment.batches.retain(|b| b.amount > 0);
        }
        self.drain_intake(output, output_cap);
        let mut room = self.stations[0]
            .capacity
            .saturating_sub(self.stations[0].stored());
        let initial = room;
        let mut ids: Vec<_> = ore
            .iter()
            .filter(|(_, q)| **q > 0)
            .map(|(&id, _)| id)
            .collect();
        order_cargo(
            &mut ids,
            tick / 20,
            preferred,
            global_priority || self.stations[0].preferred,
        );
        for id in ids {
            let q = ore.get_mut(&id).unwrap();
            let amount = (*q).min(room);
            *q -= amount;
            room -= amount;
            *self.stations[0].cargo.entry(id).or_default() += amount;
            self.stations[0].incoming += amount;
            for lot in take(&mut self.source, id, amount) {
                put(&mut self.stations[0].routing, id, lot.amount, lot.route);
            }
        }
        // Fluid flow can cross several stations in one interval. Each stage spends its budget once.
        for (index, budget) in budgets.iter_mut().enumerate() {
            let (upstream, downstream) = self.stations.split_at_mut(index + 1);
            let source = &mut upstream[index];
            let destination = &mut downstream[0];
            let mut space = destination.capacity.saturating_sub(destination.stored());
            let mut ids: Vec<_> = source
                .cargo
                .iter()
                .filter(|(_, q)| **q > 0)
                .map(|(&id, _)| id)
                .collect();
            order_cargo(
                &mut ids,
                tick,
                preferred,
                global_priority || source.preferred,
            );
            let before = *budget;
            let mut power_blocked = false;
            for id in ids {
                // Retain blocked route lots even when another route remains usable.
                for lot in source.routing.iter_mut().filter(|l| l.material == id) {
                    let powered = self
                        .routes
                        .get(&lot.route)
                        .and_then(|r| r.get(index))
                        .is_some_and(|legs| {
                            legs.iter()
                                .any(|l| matches!(l.mode.as_str(), "lift" | "train"))
                        });
                    if powered && power_permille == 0 {
                        power_blocked = true;
                        continue;
                    }
                    let amount = lot.amount.min(*budget).min(space).min(if powered {
                        powered_budgets[index]
                    } else {
                        u64::MAX
                    });
                    if powered {
                        powered_budgets[index] -= amount;
                    }
                    lot.amount -= amount;
                    *source.cargo.get_mut(&id).unwrap() -= amount;
                    *destination.cargo.entry(id).or_default() += amount;
                    if index + 1 < terminal {
                        put(&mut destination.routing, id, amount, lot.route);
                    }
                    *budget -= amount;
                    space -= amount;
                }
            }
            source.routing.retain(|l| l.amount > 0);
            source.outgoing += before - *budget;
            destination.incoming += before - *budget;
            let segment = &mut self.segments[index];
            segment.blocked = source.stored() > 0 && (space == 0 || power_blocked);
            segment.blocker = if space == 0 {
                "Destination buffer full"
            } else if segment.blocked {
                "Power supply limited"
            } else {
                ""
            }
            .into();
            segment.utilisation = if before == 0 {
                0.
            } else {
                (before - *budget) as f64 / before as f64
            };
        }
        self.drain_intake(output, output_cap);
        self.retain_used_routes();
        initial - room
    }
    fn drain_intake(&mut self, output: &mut BTreeMap<usize, u64>, cap: u64) {
        let destination = self.stations.last_mut().unwrap();
        let mut room = cap.saturating_sub(output.values().sum());
        for (&id, q) in &mut destination.cargo {
            let amount = (*q).min(room);
            *q -= amount;
            *output.entry(id).or_default() += amount;
            room -= amount;
            destination.outgoing += amount;
        }
    }
    fn retain_used_routes(&mut self) {
        let mut used: BTreeSet<_> = self.source.iter().map(|l| l.route).collect();
        used.insert(self.current_route);
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
        used.extend(self.protected_routes.iter().copied());
        self.routes.retain(|id, _| used.contains(id));
    }
    pub fn visual(&self) -> Vec<VisualCargo> {
        self.segments
            .iter()
            .enumerate()
            .flat_map(|(index, segment)| {
                segment.batches.iter().filter_map(move |batch| {
                    self.routes
                        .get(&batch.route)?
                        .get(index)
                        .map(|legs| (batch, legs))
                })
            })
            .filter(|(_, legs)| !legs.is_empty())
            .map(|(b, legs)| {
                // Render only the occupied leg. Sending every historic shaft edge
                // per batch makes IPC grow with both mine depth and vehicle count.
                let mut elapsed = b
                    .duration_ms
                    .saturating_sub(b.remaining_ms)
                    .saturating_sub(LOADING_MS);
                let mut leg = legs.last().unwrap();
                for next in legs {
                    leg = next;
                    if elapsed <= next.milliseconds {
                        break;
                    }
                    elapsed -= next.milliseconds;
                }
                let elapsed = elapsed.min(leg.milliseconds);
                VisualCargo {
                    id: b.id.to_string(),
                    route: b.route,
                    elapsed_ms: elapsed as i64
                        - LOADING_MS.saturating_sub(b.duration_ms - b.remaining_ms) as i64,
                    speed: 1.,
                    material: b.material,
                    amount: b.amount,
                    remaining: (leg.milliseconds - elapsed) as f64 / 1000.,
                    // Preserve renderer loading offset locally.
                    duration: (leg.milliseconds + LOADING_MS) as f64 / 1000.,
                    mode: leg.mode.clone(),
                    depth: crate::geometry::depth(leg.from[1].max(0) as i64),
                    path: vec![],
                    legs: legs
                        .iter()
                        .skip_while(|l| *l != leg)
                        .take(2)
                        .cloned()
                        .collect(),
                }
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
        let terminal = self.stations.len() - 1;
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
        for station in self.stations.iter_mut().take(terminal) {
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
        let last = terminal;
        let mut room = output_cap.saturating_sub(output.values().sum());
        let mut dispatched = 0;
        let destination = &mut self.stations[last];
        for (&id, q) in &mut destination.cargo {
            let n = (*q).min(room);
            *q -= n;
            *output.entry(id).or_default() += n;
            room -= n;
            dispatched += n;
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
                    if index + 1 < terminal {
                        put(&mut station.routing, batch.material, n, batch.route);
                    }
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
                    let duration = LOADING_MS + legs.iter().map(|l| l.milliseconds).sum::<u32>();
                    if let Some(batch) = segment.batches.last_mut().filter(|b| {
                        b.material == id
                            && b.route == lot.route
                            && b.duration_ms == duration
                            && b.remaining_ms / 100 == duration / 100
                    }) {
                        batch.amount += lot.amount;
                    } else if segment.batches.len() < 512 {
                        self.next_batch += 1;
                        segment.batches.push(Batch {
                            id: self.next_batch,
                            route: lot.route,
                            material: id,
                            amount: lot.amount,
                            remaining_ms: duration,
                            duration_ms: duration,
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
        let mut used = BTreeSet::from([self.current_route]);
        used.extend(self.source.iter().map(|l| l.route));
        used.extend(
            self.stations
                .iter()
                .take(terminal)
                .flat_map(|s| s.routing.iter().map(|l| l.route)),
        );
        used.extend(
            self.segments
                .iter()
                .flat_map(|s| s.batches.iter().map(|b| b.route)),
        );
        used.extend(self.protected_routes.iter().copied());
        self.routes.retain(|id, _| used.contains(id));
        initial - space
    }

    /// Re-index duplicate v10 itineraries and discard route ownership that has
    /// already reached processing intake. Route ids are private save details.
    pub fn compact_routes(&mut self) -> Result<BTreeMap<u64, u64>, String> {
        let old_routes = std::mem::take(&mut self.routes);
        let mut by_itinerary = HashMap::<Vec<Vec<Leg>>, u64>::new();
        let mut routes = BTreeMap::new();
        let mut remap = BTreeMap::new();
        let mut next = 1u64;
        for (old, itinerary) in old_routes {
            let route = if let Some(route) = by_itinerary.get(&itinerary) {
                *route
            } else {
                let route = next;
                next = next.checked_add(1).ok_or("Route identity overflow")?;
                by_itinerary.insert(itinerary.clone(), route);
                routes.insert(route, itinerary);
                route
            };
            remap.insert(old, route);
        }
        let rewrite = |route: &mut u64| -> Result<(), String> {
            *route = *remap.get(route).ok_or("Missing transport route")?;
            Ok(())
        };
        if !routes.is_empty() {
            rewrite(&mut self.current_route)?;
        }
        for lot in &mut self.source {
            rewrite(&mut lot.route)?;
        }
        let terminal = self.stations.len().saturating_sub(1);
        for station in self.stations.iter_mut().take(terminal) {
            for lot in &mut station.routing {
                rewrite(&mut lot.route)?;
            }
            coalesce(&mut station.routing)?;
        }
        if let Some(station) = self.stations.last_mut() {
            station.routing.clear();
        }
        for segment in &mut self.segments {
            for batch in &mut segment.batches {
                rewrite(&mut batch.route)?;
            }
        }
        let old_protected = std::mem::take(&mut self.protected_routes);
        for mut route in old_protected {
            rewrite(&mut route)?;
            self.protected_routes.insert(route);
        }
        coalesce(&mut self.source)?;
        self.routes = routes;
        self.next_route = next;
        self.retain_live_routes();
        Ok(remap)
    }

    fn retain_live_routes(&mut self) {
        let terminal = self.stations.len().saturating_sub(1);
        let mut used = BTreeSet::from([self.current_route]);
        used.extend(self.source.iter().map(|lot| lot.route));
        used.extend(
            self.stations
                .iter()
                .take(terminal)
                .flat_map(|station| station.routing.iter().map(|lot| lot.route)),
        );
        used.extend(
            self.segments
                .iter()
                .flat_map(|segment| segment.batches.iter().map(|batch| batch.route)),
        );
        used.extend(self.protected_routes.iter().copied());
        self.routes.retain(|route, _| used.contains(route));
    }

    pub fn sources_match(&self, ore: &BTreeMap<usize, u64>) -> bool {
        routing_matches(&self.source, ore, &self.routes)
    }
    pub fn valid(&self, materials: usize) -> bool {
        self.current_route < 1_000_000_000_000
            && self.next_route > self.routes.keys().next_back().copied().unwrap_or(0)
            && self.next_route < 1_000_000_000_000
            && self.source.iter().all(|l| {
                l.material < materials
                    && l.amount > 0
                    && l.amount <= 1_000_000_000_000
                    && self.routes.contains_key(&l.route)
            })
            && self.routes.len() <= MAX_ROUTES
            && self
                .routes
                .values()
                .flat_map(|route| route.iter())
                .map(Vec::len)
                .sum::<usize>()
                <= MAX_ROUTE_LEGS
            && self.source.len() <= 32768
            && self
                .routes
                .values()
                .all(|r| r.len() == 4 && r.iter().all(|legs| valid_legs(legs)))
            && self.stations.iter().enumerate().all(|(index, s)| {
                (if index + 1 == self.stations.len() {
                    s.routing.is_empty()
                } else {
                    routing_matches(&s.routing, &s.cargo, &self.routes)
                }) && s.routing.len() <= 32768
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
            && self.segments.iter().enumerate().all(|(index, s)| {
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
                            && self.routes.get(&b.route).is_some_and(|route| {
                                route.get(index).is_some_and(|legs| {
                                    b.duration_ms
                                        == LOADING_MS
                                            + legs.iter().map(|leg| leg.milliseconds).sum::<u32>()
                                })
                            })
                    })
            })
    }
}

fn coalesce(lots: &mut Vec<RoutedCargo>) -> Result<(), String> {
    let mut compact = BTreeMap::<(usize, u64), u64>::new();
    for lot in lots.drain(..) {
        let amount = compact.entry((lot.material, lot.route)).or_default();
        *amount = amount
            .checked_add(lot.amount)
            .ok_or("Transport cargo overflow")?;
    }
    *lots = compact
        .into_iter()
        .filter(|(_, amount)| *amount > 0)
        .map(|((material, route), amount)| RoutedCargo {
            material,
            amount,
            route,
        })
        .collect();
    Ok(())
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
    fn configuring_known_itineraries_reuses_route_identity() {
        let mut network = Network::default();
        let first = Leg {
            from: [32, 200],
            to: [256, 200],
            mode: "conveyor".into(),
            milliseconds: 1200,
        };
        let second = Leg {
            from: [480, 200],
            to: [256, 200],
            mode: "conveyor".into(),
            milliseconds: 1300,
        };
        network.configure(std::slice::from_ref(&first), UNITS, 0);
        let first_id = network.current_route;
        network.configure(std::slice::from_ref(&second), UNITS, 0);
        let second_id = network.current_route;
        for _ in 0..1000 {
            network.configure(std::slice::from_ref(&first), UNITS, 0);
            assert_eq!(network.current_route, first_id);
            network.configure(std::slice::from_ref(&second), UNITS, 0);
            assert_eq!(network.current_route, second_id);
        }
        assert_eq!(network.routes.len(), 2);
    }

    #[test]
    fn migration_compacts_duplicate_routes_and_terminal_labels() {
        let leg = Leg {
            from: [256, 300],
            to: [256, 0],
            mode: "lift".into(),
            milliseconds: 3000,
        };
        let itinerary = vec![Vec::new(), Vec::new(), vec![leg], Vec::new()];
        let protected_itinerary = vec![
            vec![Leg {
                from: [320, 300],
                to: [256, 300],
                mode: "carrying".into(),
                milliseconds: 500,
            }],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ];
        let mut network = Network::default();
        network.routes.insert(17, itinerary.clone());
        network.routes.insert(29, itinerary);
        network.routes.insert(41, protected_itinerary);
        network.protect_routes([41]);
        network.current_route = 29;
        network.next_route = 30;
        network.stations[2].cargo.insert(3, 500);
        network.stations[2].routing = vec![
            RoutedCargo {
                material: 3,
                amount: 200,
                route: 17,
            },
            RoutedCargo {
                material: 3,
                amount: 300,
                route: 29,
            },
        ];
        network.stations[4].cargo.insert(4, 700);
        network.stations[4].routing.push(RoutedCargo {
            material: 4,
            amount: 700,
            route: 29,
        });
        network.segments[2].batches.push(Batch {
            id: 0,
            route: 29,
            material: 3,
            amount: 100,
            remaining_ms: 4000,
            duration_ms: 4000,
        });
        let mass = network.mass();
        let remap = network.compact_routes().unwrap();
        assert_eq!(network.routes.len(), 2);
        assert_eq!(remap[&17], remap[&29]);
        assert!(network.routes.contains_key(&remap[&41]));
        assert_eq!(network.stations[2].routing.len(), 1);
        assert_eq!(network.stations[2].routing[0].amount, 500);
        assert!(network.stations[4].routing.is_empty());
        assert_eq!(network.mass(), mass);
        assert!(network.valid(crate::materials().len()));
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
                id: 0,
                route: 0,
                material: 0,
                amount: UNITS,
                remaining_ms: 1000,
                duration_ms: 1000,
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

    #[test]
    fn configured_segments_keep_thirty_percent_cycle_headroom_at_depth() {
        for depth in [200, 1200, 2800, 6000] {
            let path = [[-256, depth], [256, depth], [256, 0], [16, -1]];
            let levels = BTreeMap::from([
                ("conveyor".into(), 1),
                ("minecart".into(), u32::from(depth >= 1200)),
                ("shaft".into(), u32::from(depth >= 1200)),
                ("train".into(), u32::from(depth >= 6000)),
            ]);
            let (legs, _) = crate::logistics::route(&path, &levels, 1.);
            let mut network = Network::default();
            network.configure(&legs, 4 * UNITS, 0);
            for segment in network.segments.iter().filter(|s| !s.legs.is_empty()) {
                let cycle = segment
                    .rate
                    .saturating_mul(segment.duration_ms as u64)
                    .div_ceil(1000);
                assert!(
                    segment.capacity >= cycle.saturating_mul(13).div_ceil(10),
                    "{} lacks headroom at {depth} cells",
                    segment.name
                );
            }
        }
    }
}

#[cfg(test)]
mod visual_payload_tests {
    use super::*;
    #[test]
    fn cargo_visual_keeps_current_leg_position_without_whole_route() {
        let mut network = Network::default();
        let legs = vec![
            Leg {
                from: [0, 0],
                to: [8, 0],
                mode: "carrying".into(),
                milliseconds: 1000,
            },
            Leg {
                from: [8, 0],
                to: [8, 40],
                mode: "lift".into(),
                milliseconds: 3000,
            },
        ];
        network
            .routes
            .insert(1, vec![legs.clone(), Vec::new(), Vec::new(), Vec::new()]);
        network.current_route = 1;
        network.segments[0].batches.push(Batch {
            id: 0,
            route: 1,
            material: 3,
            amount: 1234,
            remaining_ms: 5000,
            duration_ms: 5000,
        });
        for (remaining, index, elapsed) in [
            (5000, 0, 0.),
            (4500, 0, 0.),
            (3500, 0, 0.5),
            (3000, 0, 1.),
            (1500, 1, 1.5),
            (0, 1, 3.),
        ] {
            network.segments[0].batches[0].remaining_ms = remaining;
            let before = serde_json::to_value(&network).unwrap();
            let visual = network.visual();
            assert_eq!(visual.len(), 1);
            assert!(visual[0].legs == legs.iter().skip(index).take(2).cloned().collect::<Vec<_>>());
            assert!((visual[0].duration - visual[0].remaining - 1. - elapsed).abs() < 1e-9);
            assert_eq!(visual[0].amount, 1234);
            assert_eq!(serde_json::to_value(&network).unwrap(), before);
        }
    }
}

#[cfg(test)]
mod idle_flow_tests {
    use super::*;
    #[test]
    fn old_powered_route_remains_limited_after_current_route_changes() {
        let mut network = Network::default();
        network.configure(
            &[Leg {
                from: [256, 40],
                to: [256, 0],
                mode: "lift".into(),
                milliseconds: 2000,
            }],
            UNITS,
            0,
        );
        let powered = network.current_route;
        network.stations[2].cargo.insert(3, UNITS);
        put(&mut network.stations[2].routing, 3, UNITS, powered);
        network.configure(&[], UNITS, 0);
        network.stations[2].cargo.insert(4, UNITS);
        put(
            &mut network.stations[2].routing,
            4,
            UNITS,
            network.current_route,
        );
        let mut output = BTreeMap::new();
        network.advance_bulk(
            20,
            20,
            &mut BTreeMap::new(),
            &mut output,
            2 * UNITS,
            &[],
            false,
            0,
        );
        assert_eq!(network.stations[2].cargo[&3], UNITS);
        assert_eq!(output[&4], UNITS);
        network.advance_bulk(
            40,
            20,
            &mut BTreeMap::new(),
            &mut output,
            2 * UNITS,
            &[],
            false,
            250,
        );
        assert_eq!(output[&3], UNITS / 4);
        assert_eq!(network.mass() + output.values().sum::<u64>(), 2 * UNITS);
    }
    #[test]
    fn bulk_flow_obeys_slowest_stage_without_creating_batches_or_mass() {
        let mut network = Network::default();
        network.configure(&[], 4 * UNITS, 0);
        network.segments[2].rate = UNITS;
        let mut ore = BTreeMap::from([(3, 100 * UNITS)]);
        let mut output = BTreeMap::new();
        network.advance_bulk(20, 20, &mut ore, &mut output, 100 * UNITS, &[], false, 1000);
        assert_eq!(output[&3], UNITS);
        assert_eq!(network.next_batch, 0);
        assert!(network.segments.iter().all(|s| s.batches.is_empty()));
        assert_eq!(
            ore.values().sum::<u64>() + output.values().sum::<u64>() + network.mass(),
            100 * UNITS
        );
        assert!(network.sources_match(&ore));
    }
    #[test]
    fn power_loss_and_full_intake_keep_existing_cargo_identity_and_clocks() {
        let mut network = Network::default();
        network.configure(
            &[Leg {
                from: [256, 40],
                to: [256, 0],
                mode: "lift".into(),
                milliseconds: 2000,
            }],
            UNITS,
            0,
        );
        network.segments[2].batches.push(Batch {
            id: 7,
            route: network.current_route,
            material: 3,
            amount: UNITS,
            remaining_ms: 1500,
            duration_ms: 3000,
        });
        network.next_batch = 7;
        let mut ore = BTreeMap::new();
        let mut output = BTreeMap::new();
        network.advance_bulk(20, 20, &mut ore, &mut output, 0, &[], false, 0);
        let batch = &network.segments[2].batches[0];
        assert_eq!(
            (batch.id, batch.amount, batch.remaining_ms),
            (7, UNITS, 1500)
        );
        assert_eq!(network.mass(), UNITS);
        network.advance_bulk(40, 20, &mut ore, &mut output, UNITS, &[], false, 1000);
        assert_eq!(output[&3], UNITS);
        assert_eq!(network.mass(), 0);
        assert_eq!(network.next_batch, 7);
    }
    #[test]
    fn finite_buffers_stop_idle_intake_without_losing_route_ownership() {
        let mut network = Network::default();
        network.configure(&[], UNITS, 0);
        for station in &mut network.stations {
            station.capacity = UNITS;
        }
        let mut ore = BTreeMap::from([(3, 20 * UNITS)]);
        let mut output = BTreeMap::new();
        for second in 1..=10 {
            network.advance_bulk(second * 20, 20, &mut ore, &mut output, 0, &[], false, 1000);
        }
        assert!(ore[&3] > 0);
        assert!(network.stations.iter().all(|s| s.stored() <= s.capacity));
        assert!(network.segments.iter().any(|s| s.blocked));
        assert!(network.sources_match(&ore));
        assert_eq!(ore[&3] + network.mass(), 20 * UNITS);
    }
}
