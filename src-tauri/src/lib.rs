mod persistence;
use mine_core::{materials, Action, Game};
use persistence::save;
use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc, Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{ipc::Channel, Emitter, Manager, State};
struct LifecycleRequest {
    issued: std::time::Instant,
    background: bool,
    timestamp: u64,
    epoch: u64,
    source: &'static str,
    reply: Option<mpsc::Sender<Result<Snapshot, String>>>,
}
/// Native mobile lifecycle owns suspension once available. WebView events
/// remain a desktop/startup fallback and cannot undo a newer native transition.
#[derive(Default)]
struct LifecycleGate {
    last: Option<std::time::Instant>,
    native_seen: bool,
}
impl LifecycleGate {
    fn accepts(&mut self, source: &str, issued: std::time::Instant) -> bool {
        if self.last.is_some_and(|last| issued <= last) || (self.native_seen && source == "webview")
        {
            return false;
        }
        self.last = Some(issued);
        self.native_seen |= source == "native";
        true
    }
}
struct Runtime {
    _save_lock: fs::File,
    game: Mutex<Game>,
    suspended: AtomicBool,
    lifecycle: mpsc::Sender<LifecycleRequest>,
    epoch: AtomicU64,
    legacy: Mutex<Option<Vec<u8>>>,
    path: PathBuf,
    save_status: Mutex<SaveStatus>,
    channel: Mutex<Option<Channel<Update>>>,
    replay: Mutex<Replay>,
}
#[derive(Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum VisualEvent {
    Start {
        session: String,
        state: Snapshot,
        total: u64,
    },
    Frame {
        session: String,
        update: Update,
        done: u64,
        total: u64,
    },
    Saved {
        session: String,
        state: Snapshot,
    },
    Failed {
        session: String,
        message: String,
        state: Snapshot,
    },
}
/// Transient compressed replay. Gameplay state remains in Runtime.game.
#[derive(Default)]
struct Replay {
    channel: Option<Channel<serde_json::Value>>,
    session: Option<String>,
    start: Option<ReplayPayload>,
    frames: std::collections::VecDeque<ReplayPayload>,
    finish: Option<ReplayPayload>,
    bytes: usize,
    pending: bool,
}
/// Oversized single snapshots spill into temporary files, removed with their session.
/// Adjacent frames still coalesce first; no geometry is discarded to meet the RAM budget.
struct ReplayPayload {
    bytes: Vec<u8>,
    path: Option<PathBuf>,
    /// Number of original buckets. Balance adjacent merges so a growing oldest
    /// snapshot is not decoded and encoded again for every arriving frame.
    buckets: u64,
}
impl ReplayPayload {
    fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            path: None,
            buckets: 1,
        }
    }
    fn resident(&self) -> usize {
        self.bytes.capacity() + self.path.as_ref().map_or(0, PathBuf::capacity)
    }
    fn decode(&self) -> Result<serde_json::Value, String> {
        match &self.path {
            Some(path) => serde_json::from_reader(flate2::read::GzDecoder::new(
                fs::File::open(path).map_err(|e| e.to_string())?,
            ))
            .map_err(|e| e.to_string()),
            None => Ok(decode_visual(&self.bytes)),
        }
    }
    fn spill(&mut self) -> Result<(), String> {
        use std::io::Write;
        if self.path.is_some() {
            return Ok(());
        }
        let path = std::env::temp_dir().join(format!(
            "deepwork-replay-{}-{}.tmp",
            std::process::id(),
            fresh_identity()
        ));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        if let Err(error) = file.write_all(&self.bytes) {
            drop(file);
            let _ = fs::remove_file(&path);
            return Err(error.to_string());
        }
        self.path = Some(path);
        self.bytes = Vec::new();
        Ok(())
    }
}
impl Drop for ReplayPayload {
    fn drop(&mut self) {
        if let Some(path) = &self.path {
            let _ = fs::remove_file(path);
        }
    }
}
fn encode_visual(value: &serde_json::Value) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    serde_json::to_writer(&mut encoder, value).expect("Serializable visual event");
    let mut encoded = encoder.finish().expect("Memory encoding");
    encoded.shrink_to_fit();
    encoded
}
fn decode_visual(bytes: &[u8]) -> serde_json::Value {
    serde_json::from_reader(flate2::read::GzDecoder::new(bytes))
        .expect("Internally encoded visual event")
}
fn coalesce_visual(left: serde_json::Value, mut right: serde_json::Value) -> serde_json::Value {
    if right["update"]["reset"] != true {
        let a = &left["update"]["state"];
        let b = &mut right["update"]["state"];
        for key in ["chunks", "visible", "revealed"] {
            if let (Some(source), Some(destination)) = (
                a["terrain"][key].as_object(),
                b["terrain"][key].as_object_mut(),
            ) {
                for (id, bytes) in source {
                    destination
                        .entry(id.clone())
                        .or_insert_with(|| bytes.clone());
                }
            }
        }
        let mut passages = a["workings"]["passages"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        passages.extend(
            b["workings"]["passages"]
                .as_array()
                .cloned()
                .unwrap_or_default(),
        );
        b["workings"]["passages"] = passages.into();
        b["workings_offset"] = a["workings_offset"].clone();
        right["update"]["reset"] = left["update"]["reset"].clone();
    }
    right
}
impl Replay {
    fn clear(&mut self) {
        self.start = None;
        self.frames.clear();
        self.frames.shrink_to_fit();
        self.finish = None;
        self.session = None;
        self.bytes = 0;
        self.pending = false;
    }
    fn publish(&mut self, event: VisualEvent) -> Result<(), String> {
        let value = serde_json::to_value(&event).expect("Serializable visual event");
        if let Some(channel) = &self.channel {
            let _ = channel.send(value.clone());
        }
        let bytes = ReplayPayload::new(encode_visual(&value));
        match event {
            VisualEvent::Start { session, .. } => {
                self.session = Some(session);
                self.bytes = bytes.resident();
                self.start = Some(bytes);
                self.frames.clear();
                self.finish = None;
                self.pending = true;
            }
            VisualEvent::Frame { .. } => {
                self.bytes += bytes.resident();
                self.frames.push_back(bytes);
            }
            _ => {
                self.bytes -= self.finish.as_ref().map_or(0, ReplayPayload::resident);
                self.bytes += bytes.resident();
                self.finish = Some(bytes);
                if self.channel.is_none() {
                    self.pending = false;
                }
            }
        }
        self.bound(16 * 1024 * 1024)
    }
    fn bound(&mut self, limit: usize) -> Result<(), String> {
        while self.bytes + self.frames.capacity() * std::mem::size_of::<ReplayPayload>() > limit
            && self.frames.len() > 1
        {
            let index = self
                .frames
                .iter()
                .zip(self.frames.iter().skip(1))
                .enumerate()
                .min_by_key(|(_, (left, right))| left.buckets.saturating_add(right.buckets))
                .map(|(index, _)| index)
                .unwrap();
            let left = self.frames.remove(index).unwrap();
            let right = self.frames.remove(index).unwrap();
            let buckets = left.buckets.saturating_add(right.buckets);
            let mut combined = ReplayPayload::new(encode_visual(&coalesce_visual(
                left.decode()?,
                right.decode()?,
            )));
            combined.buckets = buckets;
            self.bytes = self.bytes - left.resident() - right.resident() + combined.resident();
            self.frames.insert(index, combined);
        }
        self.frames.shrink_to_fit();
        let metadata = self.frames.capacity() * std::mem::size_of::<ReplayPayload>();
        for payload in self
            .start
            .iter_mut()
            .chain(self.finish.iter_mut())
            .chain(self.frames.iter_mut())
        {
            if self.bytes + metadata <= limit {
                break;
            }
            let old = payload.resident();
            payload.spill()?;
            self.bytes = self.bytes - old + payload.resident();
        }
        Ok(())
    }
}
#[derive(Clone, serde::Serialize)]
struct Construction {
    from: [i64; 2],
    to: [i64; 2],
    cleared_to: [i64; 2],
    lift: bool,
    support_progress: f64,
}
fn construction(game: &Game) -> Option<Construction> {
    let section = game.workings.section.as_ref()?;
    Some(Construction {
        from: game.workings.passages.get(section.from)?.feet,
        to: section.to,
        cleared_to: mine_core::crew::access_position(&game.terrain, &game.workings),
        lift: section.lift,
        support_progress: (section.support_work as f64
            / mine_core::workings::settings().support_work as f64)
            .min(1.),
    })
}
#[derive(Clone, Default, serde::Serialize)]
struct SaveStatus {
    last_success: u64,
    bytes: u64,
    format: u32,
    error: String,
}
#[derive(Clone, serde::Serialize)]
struct Update {
    state: Snapshot,
    reset: bool,
}
/// UI prices and retirement rewards are derived by the authoritative core.
#[derive(Clone, serde::Serialize)]
struct Snapshot {
    #[serde(flatten)]
    game: Game,
    quotes: std::collections::BTreeMap<String, String>,
    retirement_award: u64,
    requires_reset: bool,
    workings_offset: usize,
    shipments: Vec<mine_core::transport::VisualCargo>,
    work_route: Vec<[i64; 2]>,
    raw_stock_capacity: u64,
    processing: Vec<mine_core::ProcessingFeed>,
    pinned_inputs: std::collections::BTreeMap<String, u64>,
    selected_vein: Option<mine_core::workings::VeinOrderView>,
    access_depth_limit: u32,
    access_upgrades: Vec<&'static str>,
    research_invested: u64,
    upgrade_previews: std::collections::BTreeMap<String, mine_core::UpgradePreview>,
    purchase_blockers: std::collections::BTreeMap<String, String>,
    save_status: SaveStatus,
    visual_workers: Vec<mine_core::crew::VisualWorker>,
    crew_state: mine_core::crew::Counts,
    crew_roles: std::collections::BTreeMap<String, mine_core::crew::Counts>,
    construction: Option<Construction>,
}
impl From<Game> for Snapshot {
    fn from(game: Game) -> Self {
        Self::new(game, true)
    }
}
impl Snapshot {
    fn new(mut game: Game, actors: bool) -> Self {
        let construction = construction(&game);
        let mut visual_workers = if actors {
            game.movement.visual(game.travel_power())
        } else {
            Vec::new()
        };
        for worker in &mut visual_workers {
            if let Some((role, id)) = worker.job.split_once(':') {
                worker.job = format!("{role}:{}", public_front_id(id));
            }
        }
        let crew_state = game.movement.counts();
        let crew_roles = game.movement.role_counts();
        game.movement = mine_core::crew::Movement::default();
        let quotes = mine_core::requirements()
            .iter()
            .map(|u| (u.id.clone(), game.cost(&u.id).to_string()))
            .chain(
                [
                    "worker", "housing", "capacity", "recovery", "drill", "sorter",
                ]
                .into_iter()
                .map(|id| (id.into(), game.cost(id).to_string())),
            )
            .collect();
        let upgrade_previews = game.upgrade_previews();
        let selected_vein = game.workings.order_view(&game.terrain);
        let access_depth_limit = game.equipment_depth_limit();
        let mut access_upgrades = Vec::new();
        if game.lift_depth_limit() == access_depth_limit && game.level("shaft") < 50 {
            access_upgrades.push("shaft");
        }
        for (id, depth) in [("supports", 300), ("pump", 700), ("ventilation", 1500)] {
            if game.level(id) == 0 && depth == access_depth_limit {
                access_upgrades.push(id);
            }
        }
        let retirement_award = game.retirement_quote.unwrap_or_else(|| game.award());
        let purchase_blockers = mine_core::requirements()
            .iter()
            .filter_map(|u| {
                game.purchase_blocker(&u.id)
                    .map(|reason| (u.id.clone(), reason))
            })
            .collect();
        let mut shipments = if actors {
            game.transport.visual()
        } else {
            Vec::new()
        };
        for shipment in &mut shipments {
            if let Some((_, segment)) = game
                .transport
                .segments
                .iter()
                .enumerate()
                .find(|(_, s)| s.batches.iter().any(|b| b.id.to_string() == shipment.id))
            {
                shipment.speed = if segment.demand > 0 {
                    game.travel_power() as f64 / 1000.
                } else {
                    1.
                };
                if shipment.remaining == 0. {
                    shipment.speed = 0.;
                }
            }
        }
        for segment in &mut game.transport.segments {
            segment.legs.clear();
        }
        // Persistence retains search internals; rendering receives only public knowledge.
        game.workings.search = None;
        game.workings.blocked_at = None;
        game.workings.surveyed.clear();
        game.workings.deferred.clear();
        // Private anchors may lie behind the revealed facing edge.
        game.workings.veins.clear();
        // Persistence identities encode the private descriptor centre. The
        // client selects public cells and needs only the filtered order view.
        game.workings.target_deposit = None;
        game.workings.deferred_at = (0, 0);
        game.workings.exhausted.clear();
        if let Some(section) = &mut game.workings.section {
            section.cells.clear();
        }
        game.transport.routes.clear();
        game.transport.source.clear();
        for station in &mut game.transport.stations {
            station.routing.clear();
        }
        for front in &mut game.mining_fronts {
            front.route.clear();
            front.id = public_front_id(&front.id);
            front.deposit.clear();
        }
        Self {
            construction,
            visual_workers,
            crew_state,
            crew_roles,
            selected_vein,
            access_depth_limit,
            access_upgrades,
            requires_reset: game.legacy_pending,
            workings_offset: 0,
            shipments,
            work_route: game.work_route().to_vec(),
            raw_stock_capacity: game.raw_stock_capacity(),
            processing: game.processing(),
            pinned_inputs: game
                .pinned
                .as_ref()
                .and_then(|id| mine_core::requirements().iter().find(|u| &u.id == id))
                .map(|u| u.inputs.clone())
                .unwrap_or_default(),
            research_invested: game.research_invested(),
            game,
            quotes,
            retirement_award,
            upgrade_previews,
            purchase_blockers,
            save_status: SaveStatus::default(),
        }
    }
}
fn public_front_id(id: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut hash);
    format!("{:016x}", hash.finish())
}

#[cfg(test)]
mod visual_replay_tests {
    use super::*;
    #[test]
    fn idle_frames_preserve_public_geometry_without_actor_movement() {
        let mut game = Game::new(42, 1);
        game.second(materials(), false);
        let live = Snapshot::new(game.clone(), true);
        let replay = Snapshot::new(game.clone(), false);
        assert!(!live.visual_workers.is_empty());
        assert!(replay.visual_workers.is_empty());
        assert!(replay.shipments.is_empty());
        assert_eq!(replay.game.terrain.chunks, live.game.terrain.chunks);
        assert_eq!(replay.game.terrain.visible, live.game.terrain.visible);
        assert_eq!(replay.game.terrain.revealed, live.game.terrain.revealed);
        assert_eq!(replay.crew_state.assigned, game.workers);
        assert!(replay.game.workings.search.is_none());
        assert!(replay.game.workings.veins.is_empty());
    }
    #[test]
    fn replay_merges_small_adjacent_buckets_before_reencoding_old_history() {
        let mut replay = Replay::default();
        for index in 0..3 {
            let mut payload = ReplayPayload::new(encode_visual(
                &serde_json::json!({"kind":"frame","done":index,"update":{"reset":false,"state":{"workings_offset":index,"workings":{"passages":[index]},"terrain":{"chunks":{},"revealed":{},"visible":{index.to_string():[index]}}}}}),
            ));
            if index == 0 {
                payload.buckets = 1024;
            }
            replay.bytes += payload.resident();
            replay.frames.push_back(payload);
        }
        let baseline = replay.frames[0].decode().unwrap();
        let limit =
            replay.bytes - 1 + replay.frames.capacity() * std::mem::size_of::<ReplayPayload>();
        replay.bound(limit).unwrap();
        assert_eq!(replay.frames.len(), 2);
        assert_eq!(replay.frames[0].buckets, 1024);
        assert_eq!(replay.frames[0].decode().unwrap(), baseline);
        let combined = replay.frames[1].decode().unwrap();
        assert_eq!(replay.frames[1].buckets, 2);
        assert_eq!(combined["done"], 2);
        assert_eq!(
            combined["update"]["state"]["workings"]["passages"],
            serde_json::json!([1, 2])
        );
        assert!(combined["update"]["state"]["terrain"]["visible"]
            .get("1")
            .is_some());
        assert!(combined["update"]["state"]["terrain"]["visible"]
            .get("2")
            .is_some());
    }
    #[test]
    fn coalescing_keeps_order_and_only_latest_public_knowledge() {
        let frame = |offset, passages, visible| {
            serde_json::json!({"kind":"frame", "update":{"reset":false,"state":{
                "workings_offset":offset,"workings":{"passages":passages},"terrain":{"chunks":{},"revealed":{},"visible":visible}
            }}})
        };
        let left = frame(2, vec![3], serde_json::json!({"1":[255]}));
        let right = frame(3, vec![4], serde_json::json!({"1":[5],"2":[255]}));
        let combined = coalesce_visual(left.clone(), right);
        assert_eq!(
            left["update"]["state"]["terrain"]["visible"]["1"],
            serde_json::json!([255])
        );
        assert_eq!(
            combined["update"]["state"]["workings"]["passages"],
            serde_json::json!([3, 4])
        );
        assert_eq!(combined["update"]["state"]["workings_offset"], 2);
        assert_eq!(decode_visual(&encode_visual(&combined)), combined);
        let mut replay = Replay::default();
        for _ in 0..10 {
            let bytes = ReplayPayload::new(encode_visual(&combined));
            replay.bytes += bytes.resident();
            replay.frames.push_back(bytes);
        }
        replay.bound(1000).unwrap();
        assert!(replay.bytes < 1000);
    }
    #[test]
    fn oversized_baseline_spills_without_losing_geometry_and_is_removed() {
        let value = serde_json::json!({"geometry": (0..20_000).map(|i| i * 7919 % 65521).collect::<Vec<_>>()});
        let payload = ReplayPayload::new(encode_visual(&value));
        let mut replay = Replay::default();
        replay.bytes = payload.resident();
        replay.start = Some(payload);
        replay.bound(1024).unwrap();
        assert!(replay.bytes < 1024);
        assert_eq!(replay.start.as_ref().unwrap().decode().unwrap(), value);
        let path = replay.start.as_ref().unwrap().path.clone().unwrap();
        assert!(path.exists());
        drop(replay);
        assert!(!path.exists());
    }
    #[test]
    fn renderer_worker_budget_does_not_limit_crew_simulation() {
        let mut game = Game::new(42, 1);
        game.workers = 1000;
        game.housing = 1000;
        game.tick(materials(), false);
        let snapshot = Snapshot::from(game.clone());
        assert_eq!(game.movement.workers.len(), 1000);
        assert_eq!(snapshot.visual_workers.len(), 250);
        assert_eq!(snapshot.crew_state.assigned, 1000);
        assert!(snapshot.game.movement.workers.is_empty());
        for front in &snapshot.game.mining_fronts {
            assert!(front.deposit.is_empty());
        }
    }
}

fn snapshot_for(game: Game, state: &Runtime) -> Snapshot {
    snapshot_with_actors(game, state, true)
}
fn snapshot_with_actors(game: Game, state: &Runtime, actors: bool) -> Snapshot {
    let mut snapshot = Snapshot::new(game, actors);
    snapshot.save_status = state
        .save_status
        .lock()
        .map(|s| s.clone())
        .unwrap_or_default();
    snapshot
}

fn persist(state: &Runtime, game: &Game) -> Result<persistence::SaveStats, String> {
    let stats = match save(&state.path, game) {
        Ok(stats) => stats,
        Err(error) => {
            if let Ok(mut status) = state.save_status.lock() {
                status.error = format!("Save failed: {error}");
            }
            return Err(error);
        }
    };
    *state.save_status.lock().map_err(|e| e.to_string())? = SaveStatus {
        last_success: game.last_saved,
        bytes: stats.bytes,
        format: stats.format,
        error: String::new(),
    };
    Ok(stats)
}
#[derive(Default)]
struct Stream {
    passages: usize,
    identity: Option<(String, u32, u64)>,
    chunks: std::collections::BTreeMap<i64, Vec<u8>>,
    visible: std::collections::BTreeMap<i64, Vec<u8>>,
    revealed: std::collections::BTreeMap<i64, Vec<u8>>,
}
impl Stream {
    fn update(&mut self, g: &Game) -> Update {
        self.update_with_actors(g, true)
    }
    fn update_with_actors(&mut self, g: &Game, actors: bool) -> Update {
        let reset = self.identity != Some((g.campaign_id.clone(), g.site, g.seed))
            || self
                .chunks
                .keys()
                .any(|id| !g.terrain.chunks.contains_key(id));
        let mut state = g.clone();
        if !reset {
            state
                .terrain
                .chunks
                .retain(|id, bytes| self.chunks.get(id) != Some(bytes));
            state
                .terrain
                .visible
                .retain(|id, bytes| self.visible.get(id) != Some(bytes));
            state
                .terrain
                .revealed
                .retain(|id, bytes| self.revealed.get(id) != Some(bytes));
        }
        self.identity = Some((g.campaign_id.clone(), g.site, g.seed));
        self.chunks = g.terrain.chunks.clone();
        self.visible = g.terrain.visible.clone();
        self.revealed = g.terrain.revealed.clone();
        let construction = construction(g);
        let selected_vein = g.workings.order_view(&g.terrain);
        let offset = if reset {
            0
        } else {
            self.passages.min(g.workings.passages.len())
        };
        self.passages = g.workings.passages.len();
        state.workings.passages.drain(..offset);
        let mut snapshot = Snapshot::new(state, actors);
        snapshot.workings_offset = offset;
        snapshot.construction = construction;
        snapshot.selected_vein = selected_vein;
        Update {
            state: snapshot,
            reset,
        }
    }
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
#[tauri::command]
async fn connect(
    channel: Channel<Update>,
    visual_channel: Channel<serde_json::Value>,
    handle: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = handle.state::<Runtime>();
        *state.channel.lock().map_err(|e| e.to_string())? = Some(channel);
        {
            let channel = visual_channel;
            let mut replay = state.replay.lock().map_err(|e| e.to_string())?;
            replay.channel = Some(channel.clone());
            if replay.session.is_some()
                && replay.finish.as_ref().is_none_or(|finish| {
                    finish.decode().is_ok_and(|value| value["kind"] != "failed")
                })
            {
                replay.pending = true;
            }
            if let Some(start) = &replay.start {
                let baseline = start.decode()?;
                let _ = channel.send(baseline.clone());
                for frame in &replay.frames {
                    let _ = channel.send(frame.decode()?);
                }
                if let Some(finish) = &replay.finish {
                    let finish = finish.decode()?;
                    let _ = channel.send(finish.clone());
                    if finish["kind"] == "failed" {
                        return Ok(finish["state"].clone());
                    }
                }
                return Ok(baseline["state"].clone());
            }
        }
        let snapshot = snapshot_for(
            state.game.lock().map_err(|e| e.to_string())?.clone(),
            &state,
        );
        serde_json::to_value(snapshot).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
/// Persist before publishing either transition. Repeated platform/webview events
/// are idempotent; a crash after resume cannot grant the same interval again.
#[cfg(test)]
fn transition_background(
    state: &Runtime,
    background: bool,
    timestamp: u64,
) -> Result<Snapshot, String> {
    transition_epoch(
        state,
        background,
        timestamp,
        state.epoch.load(Ordering::SeqCst),
    )
}
#[cfg(test)]
fn checkpoint_gap(
    game: &mut Game,
    path: &std::path::Path,
    from: u64,
    timestamp: u64,
) -> Result<persistence::SaveStats, String> {
    let mut candidate = game.clone();
    candidate.last_saved = candidate.last_saved.max(from);
    candidate.advance_offline(timestamp, &materials());
    let stats = save(path, &candidate)?;
    *game = candidate;
    Ok(stats)
}
fn catch_up(game: &mut Game, timestamp: u64, state: &Runtime) -> Result<(), String> {
    let total = if game.last_saved == 0 {
        0
    } else {
        timestamp.saturating_sub(game.last_saved).min(28800) / 2 * 20
    };
    if total == 0 {
        game.advance_offline(timestamp, &materials());
        return Ok(());
    }
    let session = format!("{}:{}:{}", game.campaign_id, game.last_saved, timestamp);
    let mut stream = Stream::default();
    stream.update_with_actors(game, false);
    if let Ok(mut replay) = state.replay.lock() {
        replay.publish(VisualEvent::Start {
            session: session.clone(),
            state: snapshot_with_actors(game.clone(), state, false),
            total,
        })?;
    }
    let mut sent = std::time::Instant::now();
    let mut capture_error = None;
    game.advance_offline_observed(timestamp, &materials(), |game, done, total| {
        if capture_error.is_none()
            && done > 0
            && (sent.elapsed().as_millis() >= 33 || done == total)
        {
            if let Ok(mut replay) = state.replay.lock() {
                if let Err(error) = replay.publish(VisualEvent::Frame {
                    session: session.clone(),
                    update: stream.update_with_actors(game, false),
                    done,
                    total,
                }) {
                    capture_error = Some(error);
                }
            }
            sent = std::time::Instant::now();
        }
    });
    capture_error.map_or(Ok(()), Err)
}
fn finish_replay(state: &Runtime, game: &Game) {
    if let Ok(mut replay) = state.replay.lock() {
        if let Some(session) = &replay.session {
            let session = session.clone();
            if replay
                .publish(VisualEvent::Saved {
                    session,
                    state: snapshot_for(game.clone(), state),
                })
                .is_err()
            {
                // The terminal event is already sent. Reconnecting clients can read
                // committed gameplay state if transient storage becomes unavailable.
                replay.start = None;
                replay.frames.clear();
                replay.finish = None;
                replay.session = None;
                replay.bytes = 0;
                replay.pending = false;
            }
        } else {
            replay.pending = false;
        }
    }
}
fn fail_replay(state: &Runtime, error: &str, committed: &Game) {
    if let Ok(mut replay) = state.replay.lock() {
        if let Some(session) = &replay.session {
            let session = session.clone();
            let _ = replay.publish(VisualEvent::Failed {
                session,
                message: error.into(),
                state: snapshot_for(committed.clone(), state),
            });
        }
        replay.pending = false;
    }
}
#[tauri::command]
fn acknowledge_replay(session: String, state: State<Runtime>) -> Result<(), String> {
    let mut replay = state.replay.lock().map_err(|e| e.to_string())?;
    if replay.session.as_ref() == Some(&session) && replay.finish.is_some() {
        replay.clear();
    }
    Ok(())
}

fn transition_epoch(
    state: &Runtime,
    background: bool,
    timestamp: u64,
    epoch: u64,
) -> Result<Snapshot, String> {
    let mut game = state.game.lock().map_err(|e| e.to_string())?;
    if epoch != state.epoch.load(Ordering::SeqCst) {
        return Err("Campaign changed during lifecycle reconciliation".into());
    }
    if !game.legacy_pending && state.suspended.load(Ordering::Relaxed) != background {
        let mut candidate = game.clone();
        if background {
            candidate.last_saved = candidate.last_saved.max(timestamp);
        } else {
            if let Err(error) = catch_up(&mut candidate, timestamp, state) {
                fail_replay(state, &error, &game);
                return Err(error);
            }
        }
        if let Err(error) = persist(state, &candidate) {
            fail_replay(state, &error, &game);
            return Err(error);
        }
        finish_replay(state, &candidate);
        *game = candidate;
        state.suspended.store(background, Ordering::Relaxed);
    }
    Ok(snapshot_for(game.clone(), state))
}
#[tauri::command]
async fn set_background(background: bool, handle: tauri::AppHandle) -> Result<Snapshot, String> {
    let (tx, rx) = mpsc::channel();
    let state = handle.state::<Runtime>();
    state
        .lifecycle
        .send(LifecycleRequest {
            issued: std::time::Instant::now(),
            background,
            timestamp: now(),
            epoch: state.epoch.load(Ordering::SeqCst),
            source: "webview",
            reply: Some(tx),
        })
        .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || rx.recv().map_err(|e| e.to_string())?)
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn command(
    action: Action,
    campaign_id: String,
    handle: tauri::AppHandle,
) -> Result<Snapshot, String> {
    tauri::async_runtime::spawn_blocking(move || {
        command_current(action, campaign_id, &handle.state::<Runtime>())
    })
    .await
    .map_err(|e| e.to_string())?
}
fn command_current(
    action: Action,
    campaign_id: String,
    state: &Runtime,
) -> Result<Snapshot, String> {
    if state.suspended.load(Ordering::Relaxed)
        || state.replay.lock().map_err(|e| e.to_string())?.pending
    {
        return Err("Resume the game before issuing commands".into());
    }
    let mut g = state.game.lock().map_err(|e| e.to_string())?;
    if state.suspended.load(Ordering::Relaxed)
        || state.replay.lock().map_err(|e| e.to_string())?.pending
    {
        return Err("Resume the game before issuing commands".into());
    }
    if g.legacy_pending || g.campaign_id != campaign_id {
        return Err("Campaign changed; refresh before issuing commands".into());
    }
    let mut candidate = g.clone();
    candidate.action(action)?;
    candidate.last_saved = candidate.last_saved.max(now());
    persist(state, &candidate)?;
    *g = candidate;
    Ok(snapshot_for(g.clone(), state))
}
#[tauri::command]
fn export_save(state: State<Runtime>) -> Result<tauri::ipc::Response, String> {
    if let Some(raw) = state.legacy.lock().map_err(|e| e.to_string())?.as_ref() {
        return Ok(tauri::ipc::Response::new(raw.clone()));
    }
    let data = persistence::encode_container(&*state.game.lock().map_err(|e| e.to_string())?)?;
    Ok(tauri::ipc::Response::new(data))
}
#[tauri::command]
async fn import_save(
    data: String,
    encoding: String,
    handle: tauri::AppHandle,
) -> Result<Snapshot, String> {
    tauri::async_runtime::spawn_blocking(move || {
        import_current(data, encoding, &handle.state::<Runtime>())
    })
    .await
    .map_err(|e| e.to_string())?
}
fn import_current(data: String, encoding: String, state: &Runtime) -> Result<Snapshot, String> {
    if state.suspended.load(Ordering::Relaxed)
        || state.replay.lock().map_err(|e| e.to_string())?.pending
    {
        return Err("Resume the game before importing a save".into());
    }
    let mut game = state.game.lock().map_err(|e| e.to_string())?;
    if state.suspended.load(Ordering::Relaxed)
        || state.replay.lock().map_err(|e| e.to_string())?.pending
    {
        return Err("Resume the game before importing a save".into());
    }
    if game.legacy_pending {
        return Err("Archive the legacy campaign before importing".into());
    }
    {
        let mut replay = state.replay.lock().map_err(|e| e.to_string())?;
        replay.clear();
        replay.pending = true;
    }
    let mut candidate = match persistence::decode_import(&data, &encoding) {
        Ok(candidate) => candidate,
        Err(error) => {
            state.replay.lock().map_err(|e| e.to_string())?.pending = false;
            return Err(error);
        }
    };
    candidate.campaign_id = fresh_identity().to_string();
    if let Err(error) = catch_up(&mut candidate, now(), &state) {
        fail_replay(state, &error, &game);
        return Err(error);
    }
    if let Err(error) = persist(&state, &candidate) {
        fail_replay(&state, &error, &game);
        return Err(error);
    }
    finish_replay(&state, &candidate);
    *game = candidate.clone();
    Ok(snapshot_for(candidate, &state))
}
fn fresh_identity() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    timestamp.wrapping_add(NEXT.fetch_add(1, Ordering::Relaxed))
}
#[tauri::command]
fn reset_campaign(
    confirmation: String,
    campaign_id: String,
    state: State<Runtime>,
) -> Result<Snapshot, String> {
    reset_current(&state, &confirmation, &campaign_id)
}
fn reset_current(
    state: &Runtime,
    confirmation: &str,
    campaign_id: &str,
) -> Result<Snapshot, String> {
    if state.replay.lock().map_err(|e| e.to_string())?.pending {
        return Err("Finish catch-up before resetting".into());
    }
    if confirmation != "RESET" {
        return Err("Type RESET to confirm".into());
    }
    let mut game = state.game.lock().map_err(|e| e.to_string())?;
    if game.campaign_id != campaign_id {
        return Err("Campaign changed; refresh first".into());
    }
    let mut legacy = state.legacy.lock().map_err(|e| e.to_string())?;
    let raw = match legacy.as_ref() {
        Some(raw) => raw.clone(),
        None => persistence::encode_container(&game)?,
    };
    let seed = fresh_identity();
    let mut candidate = Game::new(seed, 1);
    candidate.last_saved = candidate.last_saved.max(now());
    persistence::archive(&state.path, &raw, &seed.to_string())?;
    persist(state, &candidate)?;
    *game = candidate;
    *legacy = None;
    state.epoch.fetch_add(1, Ordering::SeqCst);
    state.suspended.store(false, Ordering::Relaxed);
    Ok(snapshot_for(game.clone(), state))
}

fn open_save(dir: &std::path::Path) -> Result<(Game, Option<Vec<u8>>, SaveStatus), String> {
    let path = dir.join("mine.deepwork");
    let old_path = dir.join("mine.json");
    if let Some(raw) = persistence::incompatible_source(&path)? {
        let mut game = Game::default();
        game.legacy_pending = true;
        return Ok((game, Some(raw), SaveStatus::default()));
    }
    let active = persistence::recover_source(&path);
    if let Ok(Some((saved, raw))) = &active {
        let mut game = saved.clone();
        game.legacy_pending = false;
        return Ok((
            game.clone(),
            None,
            SaveStatus {
                last_success: game.last_saved,
                bytes: raw.len() as u64,
                format: game.version,
                error: String::new(),
            },
        ));
    }
    let incompatible = persistence::incompatible_source(&old_path)?;
    if let Some(raw) = incompatible {
        let mut game = Game::default();
        game.legacy_pending = true;
        return Ok((game, Some(raw), SaveStatus::default()));
    }
    match persistence::recover_source(&old_path) {
        Ok(Some((mut game, raw))) => {
            let archive_id = format!("pre-v12-{}", fresh_identity());
            persistence::archive(&path, &raw, &archive_id)?;
            game.legacy_pending = false;
            let stats = save(&path, &game)?;
            Ok((
                game.clone(),
                None,
                SaveStatus {
                    last_success: game.last_saved,
                    bytes: stats.bytes,
                    format: stats.format,
                    error: String::new(),
                },
            ))
        }
        Ok(None) => match active {
            Err(error) => Err(error),
            Ok(None) => Ok((
                Game::new(fresh_identity(), 1),
                None,
                SaveStatus::default(),
            )),
            Ok(Some(_)) => unreachable!(),
        },
        Err(legacy_error) => match active {
            Err(active_error) => Err(format!(
                "Active save recovery failed: {active_error}; legacy recovery failed: {legacy_error}"
            )),
            _ => Err(legacy_error),
        },
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            mine_core::content::validate().map_err(std::io::Error::other)?;
            let dir = app.path().app_data_dir()?;
            #[cfg(not(mobile))]
            let dir = if std::env::args().any(|arg| arg == "--portable") {
                std::env::current_exe()?
                    .parent()
                    .ok_or_else(|| std::io::Error::other("Executable directory unavailable"))?
                    .join("deepwork-data")
            } else {
                dir
            };
            // Integration tests use isolated saves; release builds ignore this override.
            #[cfg(debug_assertions)]
            let dir = std::env::var_os("DEEPWORK_TEST_DATA_DIR")
                .map(PathBuf::from)
                .unwrap_or(dir);
            fs::create_dir_all(&dir)?;
            let save_lock =
                persistence::lock(&dir.join("mine.lock")).map_err(std::io::Error::other)?;
            let path = dir.join("mine.deepwork");
            let (game, legacy, save_status) =
                open_save(&dir).map_err(std::io::Error::other)?;
            let (lifecycle, requests) = mpsc::channel::<LifecycleRequest>();
            app.manage(Runtime {
                _save_lock: save_lock,
                game: Mutex::new(game),
                suspended: AtomicBool::new(true),
                lifecycle: lifecycle.clone(),
                epoch: AtomicU64::new(0),
                legacy: Mutex::new(legacy),
                path,
                save_status: Mutex::new(save_status),
                channel: Mutex::new(None),
        replay: Mutex::new(Replay::default()),
            });
            let lifecycle_handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut gate = LifecycleGate::default();
                while let Ok(request) = requests.recv() {
                    let started = std::time::Instant::now();
                    if !request.background {
                        let _ = lifecycle_handle.emit("mine-reconciling", true);
                    }
                    let state = lifecycle_handle.state::<Runtime>();
                    let accepted = request.epoch == state.epoch.load(Ordering::SeqCst)
                        && gate.accepts(request.source, request.issued);
                    let result = if accepted {
                        transition_epoch(&state, request.background, request.timestamp, request.epoch)
                    } else {
                        state
                            .game
                            .lock()
                            .map(|game| snapshot_for(game.clone(), &state))
                            .map_err(|e| e.to_string())
                    };
                    if let Err(error) = &result {
                        let _ = lifecycle_handle.emit("mine-lifecycle-error", error.clone());
                    }
                    eprintln!(
                        "Lifecycle source={} background={} timestamp={} duration_ms={} success={} accepted={} report={}",
                        request.source,
                        request.background,
                        request.timestamp,
                        started.elapsed().as_millis(),
                        result.is_ok(),
                        accepted,
                        result.as_ref().ok().and_then(|s| s.game.offline.as_ref()).map(|r| r.id.as_str()).unwrap_or("none")
                    );
                    if !request.background {
                        let _ = lifecycle_handle.emit("mine-reconciling", false);
                    }
                    if let Some(reply) = request.reply {
                        let _ = reply.send(result);
                    }
                }
            });
            lifecycle
                .send(LifecycleRequest {
            issued: std::time::Instant::now(),
                    background: false,
                    timestamp: now(),
                    epoch: 0,
                    source: "startup",
                    reply: None,
                })
                .map_err(std::io::Error::other)?;
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let cat = materials();
                let mut previous = now();
                let mut stream = Stream::default();
                let mut next_tick = std::time::Instant::now();
                loop {
                    next_tick += std::time::Duration::from_millis(50);
                    std::thread::sleep(
                        next_tick.saturating_duration_since(std::time::Instant::now()),
                    );
                    let state = handle.state::<Runtime>();
                    if let Ok(mut g) = state.game.lock() {
                        let current = now();
                        if g.legacy_pending || state.suspended.load(Ordering::Relaxed) {
                            previous = current;
                            next_tick = std::time::Instant::now();
                            continue;
                        }
                        // A lifecycle resume may already have consumed this clock gap.
                        let gap_start = previous.max(g.last_saved);
                        if current.saturating_sub(gap_start) > 2 {
                            match (|| {
                                let mut candidate = g.clone();
                                candidate.last_saved = candidate.last_saved.max(gap_start);
                                catch_up(&mut candidate, current, &state).inspect_err(|e| fail_replay(&state, e, &g))?;
                                let stats = persist(&state, &candidate).inspect_err(|e| fail_replay(&state, e, &g))?;
                                finish_replay(&state, &candidate);
                                *g = candidate;
                                Ok::<_, String>(stats)
                            })() {
                                Ok(stats) => {
                                    if let Ok(mut status) = state.save_status.lock() {
                                        *status = SaveStatus {
                                            last_success: g.last_saved,
                                            bytes: stats.bytes,
                                            format: stats.format,
                                            error: String::new(),
                                        };
                                    }
                                }
                                Err(e) => {
                                    if let Ok(mut status) = state.save_status.lock() {
                                        status.error = format!("Save failed: {e}");
                                    }
                                    eprintln!("Resume save failed: {e}");
                                    // Retry from the last committed state. Do not publish
                                    // catch-up rewards or replace the interval on failure.
                                    next_tick = std::time::Instant::now()
                                        + std::time::Duration::from_secs(1);
                                    continue;
                                }
                            }
                            next_tick = std::time::Instant::now();
                        } else {
                            g.tick(&cat, false);
                        }
                        if g.ticks % 600 == 0 {
                            let mut candidate = g.clone();
                            candidate.last_saved = candidate.last_saved.max(now());
                            match persist(&state, &candidate) {
                                Ok(_) => *g = candidate,
                                Err(e) => eprintln!("Save failed: {e}"),
                            }
                        }
                        if g.ticks % 4 == 0 {
                            if let Ok(channel) = state.channel.lock() {
                                if let Some(c) = channel.as_ref() {
                                    let mut update = stream.update(&g);
                                    update.state.save_status = state
                                        .save_status
                                        .lock()
                                        .map(|status| status.clone())
                                        .unwrap_or_default();
                                    if c.send(update).is_err() {
                                        stream = Stream::default();
                                    }
                                }
                            }
                        }
                        // Checkpoint/IPC work is foreground time, not an OS suspension.
                        // The monotonic deadline catches up fixed ticks after this work.
                        previous = now();
                    };
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            connect,
            acknowledge_replay,
            command,
            export_save,
            import_save,
            set_background,
            reset_campaign
        ])
        .build(tauri::generate_context!())
        .expect("Tauri startup failed")
        .run(|handle, event| {
            #[cfg(mobile)]
            if let tauri::RunEvent::WindowEvent { event, .. } = &event {
                let background = match event {
                    tauri::WindowEvent::Suspended => Some(true),
                    tauri::WindowEvent::Resumed => Some(false),
                    _ => None,
                };
                if let Some(background) = background {
                    let state = handle.state::<Runtime>();
                    let _ = state.lifecycle.send(LifecycleRequest {
            issued: std::time::Instant::now(),
                        background,
                        timestamp: now(),
                        epoch: state.epoch.load(Ordering::SeqCst),
                        source: "native",
                        reply: None,
                    });
                }
            }
            if let tauri::RunEvent::Exit = event {
                let state = handle.state::<Runtime>();
                if let Ok(mut g) = state.game.lock() {
                    let mut candidate = g.clone();
                    if !state.suspended.load(Ordering::Relaxed) {
                        candidate.last_saved = candidate.last_saved.max(now());
                    }
                    if !candidate.legacy_pending && persist(&state, &candidate).is_ok() {
                        *g = candidate;
                    }
                };
            }
        });
}

#[cfg(test)]
mod stream_tests {
    use super::*;
    #[test]
    fn access_inspection_names_each_equipment_gate_at_the_current_limit() {
        let mut game = Game::default();
        let snapshot = Snapshot::from(game.clone());
        assert_eq!(snapshot.access_depth_limit, 300);
        assert_eq!(snapshot.access_upgrades, vec!["shaft", "supports"]);
        game.levels.insert("shaft".into(), 3);
        game.levels.insert("supports".into(), 1);
        let snapshot = Snapshot::from(game.clone());
        assert_eq!(snapshot.access_depth_limit, 700);
        assert_eq!(snapshot.access_upgrades, vec!["pump"]);
        game.levels.insert("pump".into(), 1);
        let snapshot = Snapshot::from(game.clone());
        assert_eq!(snapshot.access_depth_limit, 1500);
        assert_eq!(snapshot.access_upgrades, vec!["shaft", "ventilation"]);
        game.levels.insert("shaft".into(), 50);
        game.levels.insert("ventilation".into(), 1);
        let snapshot = Snapshot::from(game);
        assert_eq!(snapshot.access_depth_limit, 20_300);
        assert!(snapshot.access_upgrades.is_empty());
    }
    #[test]
    fn importing_earlier_terrain_resets_removed_chunks() {
        let mut stream = Stream::default();
        let mut g = Game::default();
        for y in 0..65 {
            g.terrain.excavate(32, y);
        }
        stream.update(&g);
        g.terrain = mine_core::terrain::Terrain::default();
        let update = stream.update(&g);
        assert!(update.reset);
        assert!(update.state.game.terrain.chunks.is_empty());
    }
    #[test]
    fn only_changed_chunks_are_sent() {
        let mut stream = Stream::default();
        let mut game = Game::default();
        let first = stream.update(&game);
        assert!(first.reset);
        game.terrain.excavate(32, 0);
        assert_eq!(stream.update(&game).state.game.terrain.chunks.len(), 1);
        assert!(stream.update(&game).state.game.terrain.chunks.is_empty());
        game.site += 1;
        assert!(stream.update(&game).reset);
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    #[test]
    fn native_lifecycle_rejects_stale_and_contradictory_webview_events() {
        let mut gate = LifecycleGate::default();
        let start = std::time::Instant::now();
        let at = |n| start + std::time::Duration::from_millis(n);
        assert!(gate.accepts("startup", at(0)));
        assert!(gate.accepts("webview", at(1)));
        assert!(gate.accepts("native", at(3)));
        assert!(
            !gate.accepts("native", at(2)),
            "late queued suspend is stale"
        );
        assert!(!gate.accepts("native", at(3)), "same event is idempotent");
        assert!(
            !gate.accepts("webview", at(8)),
            "visibility cannot undo native resume"
        );
        assert!(
            gate.accepts("native", at(4)),
            "ignored web event cannot stale native events"
        );
    }
    #[test]
    fn fallback_gap_publishes_only_after_a_successful_checkpoint() {
        let directory = std::env::temp_dir().join(format!("deepwork-gap-{}", fresh_identity()));
        fs::create_dir_all(&directory).unwrap();
        let blocked = directory.join("parent-file");
        fs::write(&blocked, b"not a directory").unwrap();
        let mut game = Game::default();
        game.last_saved = 100;
        let before = serde_json::to_value(&game).unwrap();
        assert!(checkpoint_gap(&mut game, &blocked.join("mine.json"), 100, 120).is_err());
        assert_eq!(serde_json::to_value(&game).unwrap(), before);
        let path = directory.join("mine.json");
        checkpoint_gap(&mut game, &path, 100, 120).unwrap();
        assert_eq!(game.ticks, 200);
        assert_eq!(game.last_saved, 120);
        assert_eq!(
            serde_json::to_value(persistence::recover(&path).unwrap().unwrap()).unwrap(),
            serde_json::to_value(&game).unwrap()
        );
        fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn replay_checkpoint_failure_restores_committed_view_and_allows_retry() {
        let directory =
            std::env::temp_dir().join(format!("deepwork-replay-failure-{}", fresh_identity()));
        fs::create_dir_all(&directory).unwrap();
        let blocked = directory.join("parent-file");
        fs::write(&blocked, b"not a directory").unwrap();
        let mut game = Game::new(42, 1);
        game.last_saved = 100;
        let before = serde_json::to_value(&game).unwrap();
        let mut state = Runtime {
            _save_lock: persistence::lock(&directory.join("mine.lock")).unwrap(),
            game: Mutex::new(game),
            suspended: AtomicBool::new(true),
            lifecycle: mpsc::channel().0,
            epoch: AtomicU64::new(0),
            legacy: Mutex::new(None),
            path: blocked.join("mine.json"),
            save_status: Mutex::new(SaveStatus::default()),
            channel: Mutex::new(None),
            replay: Mutex::new(Replay::default()),
        };
        assert!(transition_background(&state, false, 120).is_err());
        assert_eq!(
            serde_json::to_value(&*state.game.lock().unwrap()).unwrap(),
            before
        );
        {
            let replay = state.replay.lock().unwrap();
            assert!(!replay.pending);
            let terminal = replay.finish.as_ref().unwrap().decode().unwrap();
            assert_eq!(terminal["kind"], "failed");
            assert_eq!(terminal["state"]["ticks"], before["ticks"]);
            assert_eq!(terminal["state"]["credits"], before["credits"]);
        }
        state.path = directory.join("mine.deepwork");
        let restored = transition_background(&state, false, 120).unwrap();
        assert_eq!(restored.game.ticks, 200);
        assert_eq!(restored.game.last_saved, 120);
        assert_eq!(restored.game.offline.unwrap().effective, 10);
        drop(state);
        fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn repeated_lifecycle_events_apply_offline_interval_once() {
        let directory = std::env::temp_dir().join(format!(
            "deepwork-lifecycle-{}-{}",
            std::process::id(),
            now()
        ));
        fs::create_dir_all(&directory).unwrap();
        let state = Runtime {
            _save_lock: persistence::lock(&directory.join("mine.lock")).unwrap(),
            game: Mutex::new(Game::default()),
            suspended: AtomicBool::new(false),
            lifecycle: mpsc::channel().0,
            epoch: AtomicU64::new(0),
            legacy: Mutex::new(None),
            path: directory.join("mine.json"),
            save_status: Mutex::new(SaveStatus::default()),
            channel: Mutex::new(None),
            replay: Mutex::new(Replay::default()),
        };
        transition_background(&state, true, 100).unwrap();
        transition_background(&state, true, 110).unwrap();
        assert_eq!(state.game.lock().unwrap().last_saved, 100);
        let resumed = transition_background(&state, false, 120).unwrap();
        assert_eq!(resumed.game.offline.as_ref().unwrap().effective, 10);
        assert_eq!(resumed.game.ticks, 200);
        let repeated = transition_background(&state, false, 140).unwrap();
        assert_eq!(repeated.game.ticks, 200);
        assert!(transition_epoch(&state, true, 150, 999).is_err());
        let recovered = persistence::recover(&state.path).unwrap().unwrap();
        assert_eq!(recovered.last_saved, 120);
        assert_eq!(recovered.ticks, 200);
        transition_background(&state, true, 160).unwrap();
        let backwards = transition_background(&state, false, 150).unwrap();
        assert_eq!(backwards.game.offline.as_ref().unwrap().effective, 0);
        assert!(!state.suspended.load(Ordering::Relaxed));
        assert_eq!(backwards.game.last_saved, 160);
        transition_background(&state, true, 155).unwrap();
        let restored_clock = transition_background(&state, false, 160).unwrap();
        assert_eq!(restored_clock.game.ticks, 200);
        assert_eq!(restored_clock.game.offline.as_ref().unwrap().effective, 0);
        drop(state);
        fs::remove_dir_all(directory).unwrap();
    }
}

#[cfg(test)]
mod save_migration_tests {
    use super::*;

    fn directory(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "deepwork-{label}-{}-{}",
            std::process::id(),
            fresh_identity()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn legacy(game: &Game) -> Vec<u8> {
        let mut old = game.clone();
        old.version = 10;
        serde_json::to_vec(&old).unwrap()
    }

    #[test]
    fn version_ten_is_archived_exactly_before_atomic_conversion() {
        let directory = directory("v10-conversion");
        let old_path = directory.join("mine.json");
        let mut original = Game::default();
        original.credits = 42;
        let raw = legacy(&original);
        fs::write(&old_path, &raw).unwrap();

        let (migrated, pending, status) = open_save(&directory).unwrap();
        assert_eq!(migrated.campaign_id, original.campaign_id);
        assert_eq!(migrated.credits, 42);
        assert_eq!(migrated.version, mine_core::VERSION);
        assert!(pending.is_none());
        assert_eq!(status.format, mine_core::VERSION);
        assert_eq!(fs::read(&old_path).unwrap(), raw);
        assert_eq!(
            persistence::load(&directory.join("mine.deepwork"))
                .unwrap()
                .campaign_id,
            original.campaign_id
        );
        let archived = fs::read_dir(directory.join("archives"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert_eq!(fs::read(archived).unwrap(), raw);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn failed_archive_leaves_legacy_campaign_and_active_slot_untouched() {
        let directory = directory("v10-rollback");
        let old_path = directory.join("mine.json");
        let raw = legacy(&Game::default());
        fs::write(&old_path, &raw).unwrap();
        fs::write(directory.join("archives"), b"blocked").unwrap();

        assert!(open_save(&directory).is_err());
        assert_eq!(fs::read(&old_path).unwrap(), raw);
        assert!(!directory.join("mine.deepwork").exists());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn recovery_prefers_new_backup_before_legacy_primary() {
        let directory = directory("recovery-order");
        let path = directory.join("mine.deepwork");
        let mut first = Game::default();
        first.credits = 12;
        persistence::save(&path, &first).unwrap();
        let backup_bytes = fs::read(&path).unwrap();
        let mut second = first.clone();
        second.credits = 34;
        persistence::save(&path, &second).unwrap();
        fs::write(&path, b"broken primary").unwrap();
        let mut old = first.clone();
        old.credits = 56;
        fs::write(directory.join("mine.json"), legacy(&old)).unwrap();
        let mut old_backup = first.clone();
        old_backup.credits = 78;
        fs::write(directory.join("mine.bak"), legacy(&old_backup)).unwrap();

        let (recovered, _, status) = open_save(&directory).unwrap();
        assert_eq!(recovered.credits, 12);
        assert_eq!(status.bytes, backup_bytes.len() as u64);

        let mut appended = path.as_os_str().to_os_string();
        appended.push(".bak");
        fs::write(PathBuf::from(appended), b"broken backup").unwrap();
        let (recovered, _, _) = open_save(&directory).unwrap();
        assert_eq!(recovered.credits, 56);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn unsupported_json_and_container_remain_exportable_for_reset() {
        let json_directory = directory("unsupported-json");
        let mut old = Game::default();
        old.version = 9;
        let raw = serde_json::to_vec(&old).unwrap();
        fs::write(json_directory.join("mine.json"), &raw).unwrap();
        let (placeholder, pending, _) = open_save(&json_directory).unwrap();
        assert!(placeholder.legacy_pending);
        assert_eq!(pending.unwrap(), raw);
        assert!(!json_directory.join("mine.deepwork").exists());
        fs::remove_dir_all(json_directory).unwrap();

        let container_directory = directory("unsupported-container");
        let mut raw = persistence::encode_container(&Game::default()).unwrap();
        raw[8..10].copy_from_slice(&2u16.to_le_bytes());
        fs::write(container_directory.join("mine.deepwork"), &raw).unwrap();
        let (placeholder, pending, _) = open_save(&container_directory).unwrap();
        assert!(placeholder.legacy_pending);
        assert_eq!(pending.unwrap(), raw);
        fs::remove_dir_all(container_directory).unwrap();
    }
}

#[cfg(test)]
mod reset_tests {
    use super::*;
    #[test]
    fn reset_archives_and_failures_preserve_campaign() {
        let directory = std::env::temp_dir().join(format!("deepwork-reset-{}", fresh_identity()));
        fs::create_dir_all(&directory).unwrap();
        let state = Runtime {
            _save_lock: persistence::lock(&directory.join("mine.lock")).unwrap(),
            game: Mutex::new(Game::default()),
            suspended: AtomicBool::new(false),
            lifecycle: mpsc::channel().0,
            epoch: AtomicU64::new(0),
            legacy: Mutex::new(None),
            path: directory.join("mine.json"),
            save_status: Mutex::new(SaveStatus::default()),
            channel: Mutex::new(None),
            replay: Mutex::new(Replay::default()),
        };
        let id = state.game.lock().unwrap().campaign_id.clone();
        state.game.lock().unwrap().research = 100;
        assert!(reset_current(&state, "cancel", &id).is_err());
        fs::write(directory.join("archives"), "blocked").unwrap();
        assert!(reset_current(&state, "RESET", &id).is_err());
        assert_eq!(state.game.lock().unwrap().research, 100);
        fs::remove_file(directory.join("archives")).unwrap();
        let fresh = reset_current(&state, "RESET", &id).unwrap();
        assert_ne!(fresh.game.campaign_id, id);
        assert_eq!(fresh.game.research, 0);
        assert_eq!(fresh.game.workers, 3);
        assert!(fresh.game.offline.is_none());
        assert!(reset_current(&state, "RESET", &id).is_err());
        let archive = fs::read_dir(directory.join("archives"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert_eq!(persistence::load(&archive).unwrap().research, 100);
        assert_eq!(
            persistence::recover(&state.path)
                .unwrap()
                .unwrap()
                .campaign_id,
            fresh.game.campaign_id
        );
        drop(state);
        fs::remove_dir_all(directory).unwrap();
    }
}

#[cfg(test)]
mod workings_stream_tests {
    use super::*;
    #[test]
    fn passage_deltas_preserve_global_parent_indices() {
        let mut game = Game::default();
        game.workings.initialise();
        let mut stream = Stream::default();
        let first = stream.update(&game);
        assert!(first.reset);
        assert_eq!(first.state.game.workings.passages.len(), 1);
        let idle = stream.update(&game);
        assert_eq!(idle.state.workings_offset, 1);
        assert!(idle.state.game.workings.passages.is_empty());
        game.workings.passages.push(mine_core::workings::Passage {
            feet: [260, 192],
            parent: 0,
            lift: false,
            supported: true,
            column: true,
        });
        let next = stream.update(&game);
        assert_eq!(next.state.workings_offset, 1);
        assert_eq!(next.state.game.workings.passages[0].parent, 0);
    }
    #[test]
    fn renderer_has_signals_but_no_search_or_private_survey_history() {
        let mut game = Game::default();
        game.workings.initialise();
        game.workings.surveyed.insert(42);
        game.workings.veins.insert(
            "private".into(),
            mine_core::workings::VeinSurvey {
                anchor: [300, 40],
                stage: 0,
            },
        );
        game.workings.deferred.insert(43);
        game.workings.target_deposit = Some("private-centre-identity".into());
        game.workings.deferred_at = (100, 200);
        game.workings.signals.push(mine_core::workings::Signal {
            centre: [240, 240],
            radius: 23,
            confidence: 1,
        });
        let snapshot: Snapshot = game.clone().into();
        assert!(snapshot.game.workings.veins.is_empty());
        assert_eq!(
            game.workings.target_deposit.as_deref(),
            Some("private-centre-identity")
        );
        let json = serde_json::to_value(snapshot).unwrap();
        assert!(json["workings"]["target_deposit"].is_null());
        assert_eq!(json["workings"]["surveyed"], serde_json::json!([]));
        assert_eq!(json["workings"]["deferred"], serde_json::json!([]));
        assert_eq!(json["workings"]["deferred_at"], serde_json::json!([0, 0]));
        assert_eq!(json["workings"]["signals"][0].as_object().unwrap().len(), 3);
    }
}

#[cfg(test)]
mod bounded_cargo_snapshot_tests {
    use super::*;
    #[test]
    fn visual_snapshot_does_not_repeat_deep_routes_per_batch() {
        let mut game = Game::new(42, 1);
        let leg = mine_core::logistics::Leg {
            from: [0, 0],
            to: [0, 16],
            mode: "lift".into(),
            milliseconds: 1000,
        };
        game.transport.segments[0].legs = vec![leg.clone(); 4096];
        game.transport.routes.insert(
            1,
            vec![vec![leg.clone(); 4096], Vec::new(), Vec::new(), Vec::new()],
        );
        game.transport.current_route = 1;
        game.transport.segments[0]
            .batches
            .push(mine_core::transport::Batch {
                id: 0,
                route: 1,
                material: 3,
                amount: 1234,
                remaining_ms: 4_097_000,
                duration_ms: 4_097_000,
            });
        game.mining_fronts.push(mine_core::fronts::MiningFront {
            id: "front".into(),
            deposit: "deposit".into(),
            material: 3,
            face: [0, 16],
            position: [0, 16],
            crew: 1,
            haulers: 1,
            progress: 0,
            cut_work: 0,
            work_remainder: 0,
            transfer_remainder: 0,
            stockpile: 0,
            stockpiles: std::collections::BTreeMap::new(),
            capacity: mine_core::geometry::UNITS,
            route: vec![[0, 16]; 4096],
            route_id: 0,
            selected: false,
            status: "Excavating".into(),
            blocker: String::new(),
        });
        let snapshot = Snapshot::from(game.clone());
        assert!(snapshot.shipments[0].legs.len() <= 2);
        assert!(snapshot.game.transport.segments[0].legs.is_empty());
        assert!(snapshot.game.mining_fronts[0].route.is_empty());
        assert_eq!(snapshot.game.transport.mass(), game.transport.mass());
        assert_eq!(game.transport.routes[&1][0].len(), 4096);
        assert!(serde_json::to_vec(&snapshot).unwrap().len() < 100_000);
    }
}
