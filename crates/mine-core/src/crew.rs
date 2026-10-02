//! Persistent people, shared itineraries, and arrival-gated work.
use crate::{geometry::WIDTH, logistics::Crew, terrain::Terrain, workings::Workings};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap};
pub type Point = [i64; 2];
const ENTRANCE: Point = [WIDTH / 2, 0];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TravelLeg {
    pub from: Point,
    pub to: Point,
    pub mode: String,
    pub milliseconds: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Worker {
    pub id: u32,
    pub role: String,
    pub job: String,
    pub position: Point,
    pub target: Point,
    pub route: u64,
    pub leg: usize,
    pub elapsed_ms: u32,
    #[serde(default)]
    pub time_fraction: u32,
    pub activity: String,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Movement {
    pub workers: Vec<Worker>,
    pub routes: BTreeMap<u64, Vec<TravelLeg>>,
    #[serde(default)]
    route_revisions: BTreeMap<u64, (u64, u32)>,
    pub next_route: u64,
    #[serde(default)]
    pub(crate) lift_level: u32,
    #[serde(skip)]
    failed: BTreeSet<(Point, Point, u64)>,
}
#[derive(Clone)]
pub struct Job {
    pub id: String,
    pub role: &'static str,
    pub target: Point,
    pub count: u32,
    pub activity: &'static str,
}
#[derive(Clone, Default, Serialize)]
pub struct Counts {
    pub assigned: u32,
    pub travelling: u32,
    pub working: u32,
    pub blocked: u32,
}
#[derive(Clone, Serialize)]
pub struct VisualLeg {
    #[serde(flatten)]
    pub leg: TravelLeg,
    pub speed: f64,
}
impl Counts {
    fn observe(&mut self, p: &Worker) {
        self.assigned += u32::from(!p.job.is_empty());
        self.blocked += u32::from(p.activity == "blocked");
        self.travelling +=
            u32::from(p.route != 0 && p.activity != "blocked" && p.activity != "waiting");
        self.working += u32::from(
            p.route == 0
                && p.position == p.target
                && p.activity != "waiting"
                && p.activity != "blocked",
        );
    }
}
#[derive(Clone, Serialize)]
pub struct VisualWorker {
    pub id: u32,
    pub role: String,
    pub job: String,
    pub position: Point,
    pub activity: String,
    pub route: u64,
    pub elapsed_ms: f64,
    pub speed: f64,
    pub legs: Vec<VisualLeg>,
}

pub fn clear(point: Point, t: &Terrain) -> bool {
    point == ENTRANCE || t.column_clear(point[0], point[1].saturating_sub(7).max(0), point[1])
}
/// Walk cleared space; use temporary ladders for vertical development.
fn local(from: Point, to: Point, t: &Terrain, _w: &Workings) -> Option<Vec<Point>> {
    if !clear(from, t) || !clear(to, t) {
        return None;
    }
    if from == to {
        return Some(vec![from]);
    }
    if from[0] == to[0]
        && t.column_clear(
            from[0],
            from[1].min(to[1]).saturating_sub(7).max(0),
            from[1].max(to[1]),
        )
    {
        return Some(vec![from, to]);
    }
    let steps = from[0].abs_diff(to[0]).max(from[1].abs_diff(to[1]));
    if (0..=steps).all(|step| {
        clear(
            [
                from[0] + ((to[0] - from[0]) * step as i64).div_euclid(steps as i64),
                from[1] + ((to[1] - from[1]) * step as i64).div_euclid(steps as i64),
            ],
            t,
        )
    }) {
        return Some(vec![from, to]);
    }
    if from[1] == to[1] && (from[0].min(to[0])..=from[0].max(to[0])).all(|x| clear([x, from[1]], t))
    {
        return Some(vec![from, to]);
    }
    let estimate = |p: Point| p[0].abs_diff(to[0]).max(p[1].abs_diff(to[1])) * 1000;
    let mut queue = BinaryHeap::from([Reverse((estimate(from), 0u64, from))]);
    let mut previous = HashMap::from([(from, from)]);
    let mut costs = HashMap::from([(from, 0u64)]);
    while let Some(Reverse((_, cost, p))) = queue.pop() {
        if costs.get(&p) != Some(&cost) {
            continue;
        }
        if previous.len() > 8192 {
            return None;
        }
        for dx in -1..=1 {
            for dy in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let next = [p[0] + dx, p[1] + dy];
                if next[0] < from[0].min(to[0]) - 64
                    || next[0] > from[0].max(to[0]) + 64
                    || next[1] < from[1].min(to[1]) - 64
                    || next[1] > from[1].max(to[1]) + 64
                    || !clear(next, t)
                {
                    continue;
                }
                let next_cost = cost + if dx != 0 && dy != 0 { 1414 } else { 1000 };
                if costs.get(&next).is_some_and(|old| *old <= next_cost) {
                    continue;
                }
                // Cleared chambers may lose old floors as neighbouring cuts advance.
                // Temporary rope ladders keep those occupied positions connected.
                previous.insert(next, p);
                costs.insert(next, next_cost);
                if next == to {
                    let mut result = vec![to];
                    let mut at = to;
                    while at != from {
                        at = previous[&at];
                        result.push(at);
                    }
                    result.reverse();
                    return Some(result);
                }
                queue.push(Reverse((next_cost + estimate(next), next_cost, next)));
            }
        }
    }
    None
}
pub fn stand_near(point: Point, t: &Terrain, w: &Workings) -> Point {
    if clear(point, t) {
        return point;
    }
    let mut candidates = Vec::new();
    for dx in -8..=8 {
        for dy in -8..=8 {
            let p = [point[0] + dx, point[1] + dy];
            if clear(p, t) {
                candidates.push(p);
            }
        }
    }
    candidates.sort_by_key(|p| {
        (
            !t.worker_reachable(*p),
            p[0].abs_diff(point[0]) + p[1].abs_diff(point[1]),
            p[1],
            p[0],
        )
    });
    candidates
        .into_iter()
        .find(|p| !clear(point, t) || local(point, *p, t, w).is_some())
        .unwrap_or(point)
}
pub fn access_position(t: &Terrain, w: &Workings) -> Point {
    let Some(s) = &w.section else {
        return w.passages.get(w.active).map(|p| p.feet).unwrap_or(ENTRANCE);
    };
    let from = w.passages[s.from].feet;
    let steps = from[0]
        .abs_diff(s.to[0])
        .max(from[1].abs_diff(s.to[1]))
        .max(1);
    let mut result = from;
    for step in 1..=steps {
        let p = [
            from[0] + ((s.to[0] - from[0]) * step as i64).div_euclid(steps as i64),
            from[1] + ((s.to[1] - from[1]) * step as i64).div_euclid(steps as i64),
        ];
        if !clear(p, t) {
            break;
        }
        result = p;
    }
    result
}

pub fn route(
    from: Point,
    to: Point,
    t: &Terrain,
    w: &Workings,
    shaft_level: u32,
) -> Option<Vec<TravelLeg>> {
    let portals = || -> Option<Vec<Point>> {
        let attach = |point: Point| -> Option<(usize, Vec<Point>)> {
            let mut nodes: Vec<_> = w
                .passages
                .iter()
                .enumerate()
                .filter(|(_, n)| clear(n.feet, t))
                .collect();
            nodes.sort_by_key(|(_, n)| n.feet[0].abs_diff(point[0]) + n.feet[1].abs_diff(point[1]));
            let nearby: Vec<_> = nodes.into_iter().take(8).collect();
            nearby
                .iter()
                .find_map(|&(i, n)| local(point, n.feet, t, w).map(|path| (i, path)))
                .or_else(|| {
                    nearby
                        .into_iter()
                        .find_map(|(i, n)| t.worker_path(point, n.feet).map(|path| (i, path)))
                })
        };
        let (mut a, mut left) = attach(from)?;
        let (mut b, mut right) = attach(to)?;
        let mut ancestors = BTreeSet::new();
        let mut index = a;
        loop {
            ancestors.insert(index);
            if index == 0 {
                break;
            }
            index = w.passages[index].parent;
        }
        let mut tail = Vec::new();
        while !ancestors.contains(&b) {
            tail.push(w.passages[b].feet);
            b = w.passages[b].parent;
        }
        while a != b {
            a = w.passages[a].parent;
            left.push(w.passages[a].feet);
        }
        tail.reverse();
        left.extend(tail);
        right.reverse();
        left.extend(right.into_iter().skip(1));
        Some(left)
    };
    let nearby = from[0].abs_diff(to[0]).max(from[1].abs_diff(to[1])) <= 64;
    // A continuous cleared corridor needs no detour through passage ancestry.
    // Steep lateral offsets still use connected access and ladder/lift legs.
    let steps = from[0].abs_diff(to[0]).max(from[1].abs_diff(to[1])).max(1);
    let straight = (from[0] == to[0] || from[0].abs_diff(to[0]) >= from[1].abs_diff(to[1]))
        && (0..=steps).all(|step| {
            clear(
                [
                    from[0] + ((to[0] - from[0]) * step as i64).div_euclid(steps as i64),
                    from[1] + ((to[1] - from[1]) * step as i64).div_euclid(steps as i64),
                ],
                t,
            )
        });
    let path = if straight {
        Some(vec![from, to])
    } else if nearby {
        local(from, to, t, w).or_else(portals)
    } else {
        portals().or_else(|| local(from, to, t, w))
    }
    .or_else(|| t.worker_path(from, to))?;
    let mut expanded = vec![from];
    for pair in path.windows(2) {
        if pair[0][0] == pair[1][0] {
            let mut stops: Vec<_> = w
                .passages
                .iter()
                .enumerate()
                .filter(|(i, n)| *i > 0 && n.lift && n.feet[0] == pair[0][0])
                .flat_map(|(_, n)| [n.feet[1], w.passages[n.parent].feet[1]])
                .filter(|y| *y > pair[0][1].min(pair[1][1]) && *y < pair[0][1].max(pair[1][1]))
                .collect();
            stops.sort_unstable();
            stops.dedup();
            if pair[0][1] > pair[1][1] {
                stops.reverse();
            }
            expanded.extend(stops.into_iter().map(|y| [pair[0][0], y]));
        }
        expanded.push(pair[1]);
    }
    let path = expanded;
    let mut legs: Vec<TravelLeg> = Vec::new();
    for pair in path.windows(2) {
        if pair[0] == pair[1] {
            continue;
        }
        let steps = pair[0][0]
            .abs_diff(pair[1][0])
            .max(pair[0][1].abs_diff(pair[1][1]))
            .max(1);
        if !(0..=steps).all(|i| {
            clear(
                [
                    pair[0][0] + ((pair[1][0] - pair[0][0]) * i as i64).div_euclid(steps as i64),
                    pair[0][1] + ((pair[1][1] - pair[0][1]) * i as i64).div_euclid(steps as i64),
                ],
                t,
            )
        }) {
            return None;
        }
        let vertical = pair[0][0] == pair[1][0];
        let lift = vertical
            && shaft_level > 0
            && w.passages.iter().enumerate().any(|(i, n)| {
                i > 0
                    && n.lift
                    && n.feet[0] == pair[0][0]
                    && [pair[0][1], pair[1][1]].iter().all(|y| {
                        (n.feet[1].min(w.passages[n.parent].feet[1])
                            ..=n.feet[1].max(w.passages[n.parent].feet[1]))
                            .contains(y)
                    })
            });
        let mode = if lift {
            "lift"
        } else if vertical {
            "climbing"
        } else {
            "walking"
        };
        let metres =
            ((pair[0][0] - pair[1][0]) as f64).hypot((pair[0][1] - pair[1][1]) as f64) / 4.;
        let speed = if lift {
            30. * (1. + 0.12 * shaft_level as f64)
        } else if vertical {
            0.5
        } else {
            1.5
        };
        let milliseconds = (metres / speed * 1000.).ceil().max(1.) as u32;
        if let Some(last) = legs.last_mut() {
            let ab = [last.to[0] - last.from[0], last.to[1] - last.from[1]];
            let bc = [pair[1][0] - pair[0][0], pair[1][1] - pair[0][1]];
            if last.mode == mode
                && last.to == pair[0]
                && ab[0] * bc[1] == ab[1] * bc[0]
                && ab[0] * bc[0] + ab[1] * bc[1] > 0
            {
                last.to = pair[1];
                last.milliseconds = ((((last.from[0] - last.to[0]) as f64)
                    .hypot((last.from[1] - last.to[1]) as f64)
                    / 4.)
                    / speed
                    * 1000.)
                    .ceil()
                    .max(1.) as u32;
                continue;
            }
        }
        legs.push(TravelLeg {
            from: pair[0],
            to: pair[1],
            mode: mode.into(),
            milliseconds,
        });
    }
    Some(legs)
}

impl Movement {
    pub fn new(count: u32) -> Self {
        let mut movement = Self::default();
        movement.initialise(count);
        movement
    }
    pub fn initialise(&mut self, count: u32) {
        while self.workers.len() < count as usize {
            self.workers.push(Worker {
                id: self.workers.len() as u32,
                role: String::new(),
                job: String::new(),
                position: ENTRANCE,
                target: ENTRANCE,
                route: 0,
                leg: 0,
                elapsed_ms: 0,
                time_fraction: 0,
                activity: "waiting".into(),
            });
        }
        self.workers.truncate(count as usize);
    }
    pub fn tick(&mut self, jobs: &[Job], t: &Terrain, w: &Workings, shaft_level: u32, power: u32) {
        if self.lift_level != shaft_level {
            for (&id, legs) in &mut self.routes {
                for (index, leg) in legs
                    .iter_mut()
                    .enumerate()
                    .filter(|(_, l)| l.mode == "lift")
                {
                    let old = leg.milliseconds;
                    let metres = ((leg.from[0] - leg.to[0]) as f64)
                        .hypot((leg.from[1] - leg.to[1]) as f64)
                        / 4.;
                    leg.milliseconds = (metres / (30. * (1. + 0.12 * shaft_level as f64)) * 1000.)
                        .ceil()
                        .max(1.) as u32;
                    for worker in self
                        .workers
                        .iter_mut()
                        .filter(|p| p.route == id && p.leg == index)
                    {
                        let progress = (worker.elapsed_ms as u64 * 1000
                            + worker.time_fraction as u64)
                            * leg.milliseconds as u64
                            / old as u64;
                        worker.elapsed_ms = (progress / 1000) as u32;
                        worker.time_fraction = (progress % 1000) as u32;
                    }
                }
            }
            self.lift_level = shaft_level;
        }
        let mut remaining: BTreeMap<_, _> = jobs.iter().map(|j| (j.id.as_str(), j.count)).collect();
        // Keep existing owners before assigning vacancies. Priority changes do not shuffle everybody.
        for worker in &mut self.workers {
            if let Some(n) = remaining.get_mut(worker.job.as_str()).filter(|n| **n > 0) {
                *n -= 1;
            } else {
                worker.job.clear();
            }
        }
        let mut cache = BTreeMap::<(Point, Point), Option<u64>>::new();
        for worker in &mut self.workers {
            let job = jobs
                .iter()
                .find(|j| j.id == worker.job)
                .or_else(|| jobs.iter().find(|j| remaining[j.id.as_str()] > 0));
            let Some(job) = job else {
                worker.activity = "waiting".into();
                continue;
            };
            if worker.job != job.id {
                *remaining.get_mut(job.id.as_str()).unwrap() -= 1;
                worker.job = job.id.clone();
            }
            worker.role = job.role.into();
            worker.target = job.target;
            if worker.route != 0 {
                let legs = &self.routes[&worker.route];
                let mut remaining = 50_000u64;
                let mut stopped = false;
                loop {
                    let leg = &legs[worker.leg];
                    let speed = if leg.mode == "lift" {
                        power as u64
                    } else {
                        1000
                    };
                    worker.activity = if speed == 0 {
                        "blocked".into()
                    } else {
                        leg.mode.clone()
                    };
                    if speed == 0 {
                        stopped = true;
                        break;
                    }
                    let needed = (leg.milliseconds - worker.elapsed_ms) as u64 * 1000
                        - worker.time_fraction as u64;
                    if remaining * speed / 1000 < needed {
                        worker.time_fraction += (remaining * speed / 1000) as u32;
                        worker.elapsed_ms += worker.time_fraction / 1000;
                        worker.time_fraction %= 1000;
                        stopped = true;
                        break;
                    }
                    remaining -= needed * 1000 / speed;
                    worker.position = leg.to;
                    worker.elapsed_ms = 0;
                    worker.time_fraction = 0;
                    worker.leg += 1;
                    if worker.leg == legs.len() || legs.last().unwrap().to != worker.target {
                        break;
                    }
                }
                if stopped || (worker.leg < legs.len() && legs.last().unwrap().to == worker.target)
                {
                    continue;
                }
                worker.route = 0;
                worker.leg = 0;
                worker.elapsed_ms = 0;
                worker.time_fraction = 0;
            }
            if worker.position == worker.target && clear(worker.position, t) {
                worker.activity = job.activity.into();
                continue;
            }
            let key = (worker.position, worker.target);
            let id = *cache.entry(key).or_insert_with(|| {
                if self.failed.contains(&(key.0, key.1, t.revision)) {
                    return None;
                }
                if let Some((&id, _)) = self.routes.iter().find(|(id, legs)| {
                    self.route_revisions.get(id) == Some(&(w.revision, shaft_level))
                        && legs.first().is_some_and(|l| l.from == key.0)
                        && legs.last().is_some_and(|l| l.to == key.1)
                }) {
                    return Some(id);
                }
                let Some(legs) = route(key.0, key.1, t, w, shaft_level) else {
                    self.failed.insert((key.0, key.1, t.revision));
                    return None;
                };
                self.next_route += 1;
                self.routes.insert(self.next_route, legs);
                self.route_revisions
                    .insert(self.next_route, (w.revision, shaft_level));
                Some(self.next_route)
            });
            if let Some(id) = id {
                worker.route = id;
                worker.activity = self.routes[&id][0].mode.clone();
            } else {
                worker.activity = "blocked".into();
            }
        }
        let used: BTreeSet<_> = self.workers.iter().map(|p| p.route).collect();
        self.routes.retain(|id, _| used.contains(id));
        self.route_revisions.retain(|id, _| used.contains(id));
        self.failed
            .retain(|(_, _, revision)| *revision == t.revision);
    }
    pub fn working(&self, job: &str) -> u32 {
        self.workers
            .iter()
            .filter(|p| {
                p.job == job
                    && p.route == 0
                    && p.position == p.target
                    && p.activity != "blocked"
                    && p.activity != "waiting"
            })
            .count() as u32
    }
    pub fn working_counts(&self) -> BTreeMap<String, u32> {
        let mut counts = BTreeMap::new();
        for p in &self.workers {
            if p.route == 0
                && p.position == p.target
                && p.activity != "blocked"
                && p.activity != "waiting"
            {
                *counts.entry(p.job.clone()).or_default() += 1;
            }
        }
        counts
    }
    pub fn counts(&self) -> Counts {
        let mut c = Counts::default();
        for p in &self.workers {
            c.observe(p);
        }
        c
    }
    pub fn role_counts(&self) -> BTreeMap<String, Counts> {
        let mut roles = BTreeMap::<String, Counts>::new();
        for p in &self.workers {
            roles.entry(p.role.clone()).or_default().observe(p);
        }
        roles
    }
    pub fn visual(&self, power: u32) -> Vec<VisualWorker> {
        self.workers
            .iter()
            .take(250)
            .map(|p| {
                let legs: Vec<_> = self
                    .routes
                    .get(&p.route)
                    .map(|r| {
                        r.iter()
                            .skip(p.leg)
                            .take(2)
                            .map(|leg| VisualLeg {
                                leg: leg.clone(),
                                speed: if p.activity == "waiting" {
                                    0.
                                } else if leg.mode == "lift" {
                                    power as f64 / 1000.
                                } else {
                                    1.
                                },
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let speed = legs.first().map(|l| l.speed).unwrap_or(0.);
                VisualWorker {
                    id: p.id,
                    role: p.role.clone(),
                    job: p.job.clone(),
                    position: p.position,
                    activity: p.activity.clone(),
                    route: p.route,
                    elapsed_ms: p.elapsed_ms as f64 + p.time_fraction as f64 / 1000.,
                    speed,
                    legs,
                }
            })
            .collect()
    }
    pub fn valid(&self, count: u32) -> bool {
        self.next_route < u64::MAX
            && self
                .routes
                .keys()
                .all(|id| *id > 0 && *id <= self.next_route)
            && self.workers.len() <= count as usize
            && self.routes.len() <= count as usize
            && self.route_revisions.len() <= self.routes.len()
            && self
                .route_revisions
                .keys()
                .all(|id| self.routes.contains_key(id))
            && self.workers.iter().enumerate().all(|(i, p)| {
                p.id == i as u32
                    && p.time_fraction < 1000
                    && crate::geometry::valid_cell(p.position[0], p.position[1])
                    && crate::geometry::valid_cell(p.target[0], p.target[1])
                    && (p.route == 0
                        || self.routes.get(&p.route).is_some_and(|legs| {
                            p.leg < legs.len() && p.elapsed_ms < legs[p.leg].milliseconds
                        }))
            })
            && self.routes.values().all(|legs| {
                !legs.is_empty()
                    && legs.len() <= 250_000
                    && legs.windows(2).all(|pair| pair[0].to == pair[1].from)
                    && legs.iter().all(|l| {
                        l.milliseconds > 0
                            && ["walking", "climbing", "lift"].contains(&l.mode.as_str())
                            && crate::geometry::valid_cell(l.from[0], l.from[1])
                            && crate::geometry::valid_cell(l.to[0], l.to[1])
                    })
            })
    }
}

pub fn surface_jobs(crew: &Crew) -> Vec<Job> {
    [
        ("operators", crew.operators),
        ("reclaimers", crew.reclaimers),
    ]
    .into_iter()
    .map(|(role, count)| Job {
        id: role.into(),
        role,
        target: ENTRANCE,
        count,
        activity: "working",
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cleared_corridor_avoids_distant_portal_detours() {
        let mut terrain = Terrain::default();
        for y in 0..=8 {
            for x in 0..=256 {
                assert!(terrain.excavate(x, y));
            }
        }
        for y in 9..=24 {
            assert!(terrain.excavate(192, y));
        }
        for y in 17..=24 {
            for x in (0..192).rev() {
                assert!(terrain.excavate(x, y));
            }
        }
        let mut workings = Workings::default();
        workings.initialise();
        for (feet, parent, lift) in [
            ([256, 8], 0, true),
            ([192, 8], 1, false),
            ([192, 24], 2, true),
            ([32, 24], 3, false),
            ([0, 8], 1, false),
        ] {
            workings.passages.push(crate::workings::Passage {
                feet,
                parent,
                lift,
                supported: true,
                column: false,
            });
        }
        let legs = route([0, 24], [80, 24], &terrain, &workings, 1).unwrap();
        assert_eq!(legs.len(), 1);
        assert_eq!(legs[0].from, [0, 24]);
        assert_eq!(legs[0].to, [80, 24]);
        assert_eq!(legs[0].mode, "walking");
        assert_eq!(legs[0].milliseconds, 13_334);
    }
    #[test]
    fn unassigned_worker_holds_occupied_leg_without_visual_prediction() {
        let (terrain, workings) = corridor();
        let job = Job {
            id: "face".into(),
            role: "diggers",
            target: [272, 7],
            count: 1,
            activity: "digging",
        };
        let mut movement = Movement::new(1);
        movement.tick(&[job.clone()], &terrain, &workings, 0, 1000);
        movement.tick(&[job], &terrain, &workings, 0, 1000);
        let elapsed = movement.workers[0].elapsed_ms;
        movement.tick(&[], &terrain, &workings, 0, 1000);
        assert_eq!(movement.workers[0].elapsed_ms, elapsed);
        assert_eq!(movement.workers[0].activity, "waiting");
        assert_eq!(movement.counts().travelling, 0);
        let visual = movement.visual(1000);
        assert!(!visual[0].legs.is_empty());
        assert!(visual[0].legs.iter().all(|leg| leg.speed == 0.));
    }
    fn corridor() -> (Terrain, Workings) {
        let mut t = Terrain::default();
        for x in 256..=272 {
            for y in 0..=7 {
                t.excavate(x, y);
            }
        }
        let mut w = Workings::default();
        w.initialise();
        (t, w)
    }
    #[test]
    fn assignment_waits_for_arrival_and_survives_reload() {
        let (t, w) = corridor();
        let job = Job {
            id: "dig".into(),
            role: "diggers",
            target: [272, 7],
            count: 1,
            activity: "digging",
        };
        let mut m = Movement::default();
        m.initialise(1);
        m.tick(&[job.clone()], &t, &w, 0, 1000);
        assert_eq!(m.working("dig"), 0);
        for _ in 0..8 {
            m.tick(&[job.clone()], &t, &w, 0, 1000);
        }
        let mut restored: Movement =
            serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        for _ in 0..100 {
            m.tick(&[job.clone()], &t, &w, 0, 1000);
            restored.tick(&[job.clone()], &t, &w, 0, 1000);
        }
        assert_eq!(
            serde_json::to_value(&m).unwrap(),
            serde_json::to_value(restored).unwrap()
        );
        assert_eq!(m.working("dig"), 1);
    }

    #[test]
    fn waiting_for_next_section_keeps_assignment_without_work_credit() {
        let (terrain, workings) = corridor();
        let mut movement = Movement::default();
        movement.initialise(1);
        let mut job = Job {
            id: "access".into(),
            role: "diggers",
            target: ENTRANCE,
            count: 1,
            activity: "waiting",
        };
        for _ in 0..20 {
            movement.tick(&[job.clone()], &terrain, &workings, 0, 1000);
            assert_eq!(movement.workers[0].job, "access");
            assert_eq!(movement.working("access"), 0);
        }
        job.activity = "digging";
        movement.tick(&[job], &terrain, &workings, 0, 1000);
        assert_eq!(movement.working("access"), 1);
    }
    #[test]
    fn rock_gap_and_occupied_rock_never_form_routes() {
        let (mut t, w) = corridor();
        for x in 280..=288 {
            for y in 0..=7 {
                t.excavate(x, y);
            }
        }
        assert!(route([256, 7], [288, 7], &t, &w, 0).is_none());
        assert!(route([260, 7], [260, 20], &t, &w, 0).is_none());
    }
    #[test]
    fn stable_jobs_do_not_shuffle_people_when_recruiting() {
        let (t, w) = corridor();
        let mut m = Movement::default();
        m.initialise(2);
        let mut jobs = vec![
            Job {
                id: "dig".into(),
                role: "diggers",
                target: [272, 7],
                count: 1,
                activity: "digging",
            },
            Job {
                id: "haul".into(),
                role: "haulers",
                target: [256, 0],
                count: 1,
                activity: "hauling",
            },
        ];
        m.tick(&jobs, &t, &w, 0, 1000);
        let route = m.workers[0].route;
        m.initialise(3);
        jobs[0].count = 2;
        m.tick(&jobs, &t, &w, 0, 1000);
        assert_eq!(m.workers[0].route, route);
        assert_eq!(m.workers[1].job, "haul");
        assert_eq!(m.workers[2].job, "dig");
    }
    #[test]
    fn powered_lift_stops_without_power() {
        let mut t = Terrain::default();
        for y in 0..=32 {
            for x in 253..=259 {
                t.excavate(x, y);
            }
        }
        let mut w = Workings::default();
        w.initialise();
        w.passages.push(crate::workings::Passage {
            feet: [256, 32],
            parent: 0,
            lift: true,
            supported: true,
            column: false,
        });
        let job = Job {
            id: "dig".into(),
            role: "diggers",
            target: [256, 32],
            count: 1,
            activity: "digging",
        };
        let mut m = Movement::default();
        m.initialise(1);
        m.tick(&[job.clone()], &t, &w, 1, 1000);
        let before = m.workers[0].elapsed_ms;
        m.tick(&[job], &t, &w, 1, 0);
        assert_eq!(m.workers[0].elapsed_ms, before);
        assert_eq!(m.workers[0].activity, "blocked");
    }
    #[test]
    fn signed_slopes_connect_across_chunks_in_both_directions() {
        let mut t = Terrain::default();
        let mut w = Workings::default();
        for i in 0..=5 {
            let feet = [-80 + i * 32, 32 + i * 5];
            w.passages.push(crate::workings::Passage {
                feet,
                parent: (i - 1).max(0) as usize,
                lift: false,
                supported: true,
                column: false,
            });
            if i > 0 {
                for p in crate::workings::cut_cells(w.passages[i as usize - 1].feet, feet, false) {
                    t.frontier.insert(crate::geometry::cell_key(p[0], p[1]));
                    t.excavate(p[0], p[1]);
                }
            }
        }
        let a = w.passages[0].feet;
        let b = w.passages[5].feet;
        assert!(route(a, b, &t, &w, 0).is_some());
        assert!(route(b, a, &t, &w, 0).is_some());
    }
    #[test]
    fn priority_change_finishes_occupied_leg_without_teleporting() {
        let (t, w) = corridor();
        let mut m = Movement::new(1);
        let mut job = Job {
            id: "dig".into(),
            role: "diggers",
            target: [272, 7],
            count: 1,
            activity: "digging",
        };
        for _ in 0..5 {
            m.tick(&[job.clone()], &t, &w, 0, 1000);
        }
        let before = m.workers[0].elapsed_ms;
        let occupied = m.workers[0].route;
        job.id = "haul".into();
        job.target = ENTRANCE;
        m.tick(&[job], &t, &w, 0, 1000);
        assert_eq!(m.workers[0].route, occupied);
        assert!(m.workers[0].elapsed_ms > before);
        assert_eq!(m.working("haul"), 0);
    }
    #[test]
    fn low_lift_power_retains_fractional_travel() {
        let (t, w) = corridor();
        let mut m = Movement::new(1);
        m.routes.insert(
            1,
            vec![TravelLeg {
                from: ENTRANCE,
                to: [272, 7],
                mode: "lift".into(),
                milliseconds: 1000,
            }],
        );
        m.next_route = 1;
        m.workers[0].route = 1;
        let job = Job {
            id: "dig".into(),
            role: "diggers",
            target: [272, 7],
            count: 1,
            activity: "digging",
        };
        for _ in 0..20 {
            m.tick(&[job.clone()], &t, &w, 1, 1);
        }
        assert_eq!(m.workers[0].elapsed_ms, 1);
        assert_eq!(m.working("dig"), 0);
    }
    #[test]
    fn long_cleared_chambers_connect_after_reload() {
        let t = Terrain::from_heights(&(220..=440).map(|x| (x, 80)).collect());
        let mut restored: Terrain =
            serde_json::from_str(&serde_json::to_string(&t).unwrap()).unwrap();
        restored.rebuild().unwrap();
        let mut w = Workings::default();
        w.initialise();
        for terrain in [&t, &restored] {
            assert!(terrain.worker_reachable([430, 79]));
            let legs = route(ENTRANCE, [430, 79], terrain, &w, 0).unwrap();
            assert_eq!(legs.first().unwrap().from, ENTRANCE);
            assert_eq!(legs.last().unwrap().to, [430, 79]);
        }
    }
    #[test]
    fn commissioned_lift_and_development_ladder_keep_distinct_timing() {
        let mut t = Terrain::default();
        for y in 0..=80 {
            for x in 253..=259 {
                t.excavate(x, y);
            }
        }
        let mut w = Workings::default();
        w.initialise();
        for y in [32, 64] {
            let parent = w.passages.len() - 1;
            w.passages.push(crate::workings::Passage {
                feet: [256, y],
                parent,
                lift: true,
                supported: true,
                column: false,
            });
        }
        w.section = Some(crate::workings::Section {
            from: 2,
            to: [256, 96],
            lift: true,
            cells: Vec::new(),
            support_work: 0,
        });
        let legs = route(ENTRANCE, [256, 80], &t, &w, 1).unwrap();
        assert!(legs.iter().any(|l| l.mode == "lift"));
        assert_eq!(legs.last().unwrap().mode, "climbing");
        assert_eq!(legs.last().unwrap().milliseconds, 8000);
        assert!(route(ENTRANCE, [256, 96], &t, &w, 1).is_none());
    }
    #[test]
    fn lift_upgrade_preserves_position_and_retimes_active_leg() {
        let (t, w) = corridor();
        let mut m = Movement::new(1);
        m.lift_level = 1;
        m.routes.insert(
            1,
            vec![TravelLeg {
                from: ENTRANCE,
                to: [272, 7],
                mode: "lift".into(),
                milliseconds: 130,
            }],
        );
        m.next_route = 1;
        m.workers[0].route = 1;
        m.workers[0].elapsed_ms = 65;
        let job = Job {
            id: "dig".into(),
            role: "diggers",
            target: [272, 7],
            count: 1,
            activity: "digging",
        };
        m.tick(&[job], &t, &w, 2, 0);
        let duration = m.routes[&1][0].milliseconds;
        assert!(duration < 130);
        assert_eq!(
            m.workers[0].elapsed_ms as u64 * 1000 + m.workers[0].time_fraction as u64,
            duration as u64 * 500
        );
        assert_eq!(m.workers[0].position, ENTRANCE);
        assert_eq!(m.workers[0].activity, "blocked");
    }
}
