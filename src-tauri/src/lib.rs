mod persistence;
use base64::Engine;
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
}
impl From<Game> for Snapshot {
    fn from(mut game: Game) -> Self {
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
        let shipments = game.transport.visual();
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
        }
        Self {
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

fn snapshot_for(game: Game, state: &Runtime) -> Snapshot {
    let mut snapshot: Snapshot = game.into();
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
        let offset = if reset {
            0
        } else {
            self.passages.min(g.workings.passages.len())
        };
        self.passages = g.workings.passages.len();
        state.workings.passages.drain(..offset);
        let mut snapshot: Snapshot = state.into();
        snapshot.workings_offset = offset;
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
async fn connect(channel: Channel<Update>, handle: tauri::AppHandle) -> Result<Snapshot, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = handle.state::<Runtime>();
        *state.channel.lock().map_err(|e| e.to_string())? = Some(channel);
        let snapshot = snapshot_for(
            state.game.lock().map_err(|e| e.to_string())?.clone(),
            &state,
        );
        Ok(snapshot)
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
            candidate.advance_offline(timestamp, &materials());
        }
        persist(state, &candidate)?;
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
    let mut g = state.game.lock().map_err(|e| e.to_string())?;
    if state.suspended.load(Ordering::Relaxed) {
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
fn import_save(data: String, encoding: String, state: State<Runtime>) -> Result<Snapshot, String> {
    let mut game = state.game.lock().map_err(|e| e.to_string())?;
    if state.suspended.load(Ordering::Relaxed) {
        return Err("Resume the game before importing a save".into());
    }
    if game.legacy_pending {
        return Err("Archive the legacy campaign before importing".into());
    }
    let bytes = match encoding.as_str() {
        "base64" => {
            let encoded_limit = persistence::MAX_CONTAINER_BYTES.div_ceil(3) * 4;
            if data.len() > encoded_limit {
                return Err("Compressed save exceeds 16 MiB".into());
            }
            base64::engine::general_purpose::STANDARD
                .decode(data)
                .map_err(|_| "Invalid base64 save data")?
        }
        "json" => {
            if data.len() > persistence::MAX_LEGACY_BYTES {
                return Err("Legacy save exceeds 128 MiB".into());
            }
            data.into_bytes()
        }
        _ => return Err("Unsupported import encoding".into()),
    };
    let mut candidate = persistence::decode_bytes(&bytes)?;
    candidate.campaign_id = fresh_identity().to_string();
    candidate.advance_offline(now(), &materials());
    persist(&state, &candidate)?;
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
            let archive_id = format!("pre-v11-{}", fresh_identity());
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
                            match checkpoint_gap(&mut g, &state.path, gap_start, current) {
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
        assert_eq!(snapshot.shipments[0].legs.len(), 1);
        assert!(snapshot.game.transport.segments[0].legs.is_empty());
        assert!(snapshot.game.mining_fronts[0].route.is_empty());
        assert_eq!(snapshot.game.transport.mass(), game.transport.mass());
        assert_eq!(game.transport.routes[&1][0].len(), 4096);
        assert!(serde_json::to_vec(&snapshot).unwrap().len() < 100_000);
    }
}
