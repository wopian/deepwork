//! Authored economy parameters shared by gameplay and deterministic benchmarks.
use serde::Deserialize;
use std::{collections::BTreeMap, sync::OnceLock};
#[derive(Deserialize)]
pub struct Pacing {
    pub costs: BTreeMap<String, u64>,
    pub worker_growth: f64,
    pub machine_growth: f64,
    pub capacity_growth: f64,
    pub worker_rate: f64,
    pub haul_rate: f64,
    pub sorting_rate: f64,
    pub refining_rate: f64,
    pub research_base: u64,
    pub starter_hold_units: u64,
    pub starter_iron_multiplier: u64,
    pub foundation_upgrades: Vec<String>,
    pub station_units: [u64; 5],
    pub transit_units: u64,
    pub station_cost: u64,
    pub windows: BTreeMap<String, [u64; 2]>,
}
pub fn get() -> &'static Pacing {
    static DATA: OnceLock<Pacing> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../../../content/pacing.json"))
            .expect("valid pacing content")
    })
}
pub fn validate() -> Result<(), String> {
    let p = get();
    if crate::requirements()
        .iter()
        .any(|u| !p.costs.contains_key(&u.id))
        || p.costs.values().any(|&v| v == 0 || v > 1_000_000_000)
        || [p.worker_growth, p.machine_growth, p.capacity_growth]
            .iter()
            .any(|v| !v.is_finite() || !(1. ..=2.).contains(v))
        || [p.worker_rate, p.haul_rate, p.sorting_rate, p.refining_rate]
            .iter()
            .any(|v| !v.is_finite() || !(0.01..=100.).contains(v))
        || p.research_base == 0
        || p.starter_hold_units > 8
        || !(1..=4).contains(&p.starter_iron_multiplier)
        || p.foundation_upgrades
            .iter()
            .any(|id| !crate::requirements().iter().any(|u| u.id == *id))
        || p.station_cost == 0
        || p.station_units.iter().any(|&v| v == 0 || v > 1000)
        || p.transit_units == 0
        || p.transit_units > 1000
        || p.windows.values().any(|w| w[0] > w[1])
    {
        return Err("Invalid campaign pacing parameters".into());
    }
    Ok(())
}
