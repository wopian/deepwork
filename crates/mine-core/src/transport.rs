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
}
impl Station {
    pub fn stored(&self) -> u64 {
        self.cargo.values().sum()
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Batch {
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
    pub demand: u32,
    pub utilisation: f64,
    pub time_fraction: u64,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Network {
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
    pub path: Vec<[u32; 2]>,
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
            capacity: [4, 8, 8, 12, 20][index] * UNITS,
            cargo: BTreeMap::new(),
            preferred: false,
            incoming: 0,
            outgoing: 0,
            quote: "60".into(),
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
            capacity: 20 * UNITS,
            duration_ms: 2000,
            legs: vec![],
            rate: UNITS,
            blocked: false,
            demand: 0,
            utilisation: 0.,
            time_fraction: 0,
        })
        .collect();
        Self {
            stations,
            segments,
            express: 0,
        }
    }
}
impl Network {
    pub fn configure(&mut self, legs: &[Leg], rate: u64, global_capacity: u32) {
        for (i, station) in self.stations.iter_mut().enumerate() {
            station.capacity = ([4, 8, 8, 12, 20][i] * UNITS + 5 * UNITS * global_capacity as u64)
                * (4 + station.level as u64)
                / 4;
            station.quote = (60. * 1.12f64.powi(station.level as i32))
                .ceil()
                .to_string();
        }
        for segment in &mut self.segments {
            segment.legs.clear();
            segment.rate = rate;
            segment.demand = 0;
        }
        for leg in legs {
            let index = if leg.from[1] < 0 && leg.to[1] < 0 {
                3
            } else if leg.from[0] == leg.to[0] {
                2
            } else if leg.from[1] >= 192 {
                1
            } else {
                0
            };
            let segment = &mut self.segments[index];
            if matches!(leg.mode.as_str(), "conveyor" | "train" | "lift") {
                segment.demand = 1;
            }
            segment.legs.push(leg.clone());
        }
        for segment in &mut self.segments {
            segment.duration_ms = 2000 + segment.legs.iter().map(|l| l.milliseconds).sum::<u32>();
            segment.capacity = (20 + global_capacity as u64 * 5) * UNITS;
        }
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
                    .map(|l| l.from[1].max(0) as u32 / 4)
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
        for (&id, q) in &mut self.stations[last].cargo {
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
            for batch in &mut segment.batches {
                batch.remaining_ms = batch.remaining_ms.saturating_sub(elapsed);
                if batch.remaining_ms == 0 {
                    let n = batch.amount.min(space);
                    batch.amount -= n;
                    space -= n;
                    *station.cargo.entry(batch.material).or_default() += n;
                    station.incoming += n;
                    segment.blocked |= batch.amount > 0;
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
                // Coalesce same-material departures within a 100ms service slot.
                if let Some(batch) = segment.batches.last_mut().filter(|b| {
                    b.material == id
                        && b.duration_ms == segment.duration_ms
                        && b.remaining_ms / 100 == segment.duration_ms / 100
                }) {
                    batch.amount += n;
                } else if segment.batches.len() < 512 {
                    segment.batches.push(Batch {
                        material: id,
                        amount: n,
                        remaining_ms: segment.duration_ms,
                        duration_ms: segment.duration_ms,
                        legs: segment.legs.clone(),
                    });
                } else {
                    break;
                }
                *station.cargo.get_mut(&id).unwrap() -= n;
                station.outgoing += n;
                budget -= n;
            }
            segment.utilisation = if initial_budget > 0 {
                (initial_budget - budget) as f64 / initial_budget as f64
            } else {
                0.
            };
            segment.blocked |= used >= segment.capacity || segment.batches.len() >= 512;
        }
        initial - space
    }
    pub fn valid(&self, materials: usize) -> bool {
        self.stations.len() == 5
            && self.segments.len() == 4
            && self.express < 4
            && self.stations.iter().enumerate().all(|(index, s)| {
                s.id == index.to_string()
                    && s.name.len() <= 64
                    && s.quote.len() <= 32
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
                            && b.amount <= 1_000_000_000_000
                            && b.remaining_ms <= b.duration_ms
                            && b.duration_ms > 0
                            && b.duration_ms <= 100_000_000
                            && valid_legs(&b.legs)
                    })
            })
    }
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
                    (0..=576).contains(&p[0])
                        && (-24..crate::geometry::MAX_ROWS as i32).contains(&p[1])
                })
        })
}
#[cfg(test)]
mod tests {
    use super::*;
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
    fn express_only_accelerates_selected_segment() {
        let mut n = Network::default();
        n.express = 2;
        for s in &mut n.segments {
            s.batches.push(Batch {
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
}
