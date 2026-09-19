mod persistence;
use mine_core::{materials, Action, Game};
use persistence::{recover, save};
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
    legacy: Mutex<Option<String>>,
    path: PathBuf,
    channel: Mutex<Option<Channel<Update>>>,
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
    research_invested: u64,
    upgrade_previews: std::collections::BTreeMap<String, mine_core::UpgradePreview>,
    purchase_blockers: std::collections::BTreeMap<String, String>,
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
            for batch in &mut segment.batches {
                batch.legs.clear();
            }
        }
        // Persistence retains search internals; rendering receives only public knowledge.
        game.workings.search = None;
        game.workings.blocked_at = None;
        game.workings.surveyed.clear();
        game.workings.deferred.clear();
        // Private anchors may lie behind the revealed facing edge.
        game.workings.veins.clear();
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
        Self {
            selected_vein,
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
        }
    }
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
        let snapshot = state.game.lock().map_err(|e| e.to_string())?.clone().into();
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
            candidate.last_saved = timestamp;
        } else {
            candidate.advance_offline(timestamp, &materials());
        }
        save(&state.path, &candidate)?;
        *game = candidate;
        state.suspended.store(background, Ordering::Relaxed);
    }
    Ok(game.clone().into())
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
    candidate.last_saved = now();
    save(&state.path, &candidate)?;
    *g = candidate;
    Ok(g.clone().into())
}
#[tauri::command]
fn export_save(state: State<Runtime>) -> Result<String, String> {
    if let Some(raw) = state.legacy.lock().map_err(|e| e.to_string())?.as_ref() {
        return Ok(raw.clone());
    }
    persistence::encode(&*state.game.lock().map_err(|e| e.to_string())?)
}
#[tauri::command]
fn import_save(data: String, state: State<Runtime>) -> Result<Snapshot, String> {
    let mut game = state.game.lock().map_err(|e| e.to_string())?;
    if state.suspended.load(Ordering::Relaxed) {
        return Err("Resume the game before importing a save".into());
    }
    if game.legacy_pending {
        return Err("Archive the legacy campaign before importing".into());
    }
    let mut candidate = persistence::decode(&data)?;
    candidate.campaign_id = fresh_identity().to_string();
    candidate.advance_offline(now(), &materials());
    save(&state.path, &candidate)?;
    *game = candidate.clone();
    Ok(candidate.into())
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
        None => persistence::encode(&game)?,
    };
    let seed = fresh_identity();
    let mut candidate = Game::new(seed, 1);
    candidate.last_saved = now();
    persistence::archive(&state.path, &raw, &seed.to_string())?;
    save(&state.path, &candidate)?;
    *game = candidate;
    *legacy = None;
    state.epoch.fetch_add(1, Ordering::SeqCst);
    state.suspended.store(false, Ordering::Relaxed);
    Ok(game.clone().into())
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
            let path = dir.join("mine.json");
            let legacy = persistence::legacy_source(&path).map_err(std::io::Error::other)?;
            let mut game = if legacy.is_some() {
                Game::default()
            } else {
                recover(&path)
                    .map_err(std::io::Error::other)?
                    .unwrap_or_else(|| Game::new(fresh_identity(), 1))
            };
            game.legacy_pending = legacy.is_some();
            let (lifecycle, requests) = mpsc::channel::<LifecycleRequest>();
            app.manage(Runtime {
                _save_lock: save_lock,
                game: Mutex::new(game),
                suspended: AtomicBool::new(true),
                lifecycle: lifecycle.clone(),
                epoch: AtomicU64::new(0),
                legacy: Mutex::new(legacy),
                path,
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
                        state.game.lock().map(|game| game.clone().into()).map_err(|e| e.to_string())
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
                            g.last_saved = gap_start;
                            g.advance_offline(current, &cat);
                            next_tick = std::time::Instant::now();
                            if let Err(e) = save(&state.path, &g) {
                                eprintln!("Resume save failed: {e}")
                            }
                        } else {
                            g.tick(&cat, false);
                        }
                        if g.ticks % 600 == 0 {
                            g.last_saved = now();
                            if let Err(e) = save(&state.path, &g) {
                                eprintln!("Save failed: {e}")
                            }
                        }
                        if g.ticks % 4 == 0 {
                            if let Ok(channel) = state.channel.lock() {
                                if let Some(c) = channel.as_ref() {
                                    if c.send(stream.update(&g)).is_err() {
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
                    if !state.suspended.load(Ordering::Relaxed) {
                        g.last_saved = now();
                    }
                    if !g.legacy_pending {
                        let _ = save(&state.path, &g);
                    }
                };
            }
        });
}

#[cfg(test)]
mod stream_tests {
    use super::*;
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
        let recovered = recover(&state.path).unwrap().unwrap();
        assert_eq!(recovered.last_saved, 120);
        assert_eq!(recovered.ticks, 200);
        transition_background(&state, true, 160).unwrap();
        let backwards = transition_background(&state, false, 150).unwrap();
        assert_eq!(backwards.game.offline.as_ref().unwrap().effective, 0);
        assert!(!state.suspended.load(Ordering::Relaxed));
        drop(state);
        fs::remove_dir_all(directory).unwrap();
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
            recover(&state.path).unwrap().unwrap().campaign_id,
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
        game.workings.deferred_at = (100, 200);
        game.workings.signals.push(mine_core::workings::Signal {
            centre: [240, 240],
            radius: 23,
            confidence: 1,
        });
        let snapshot: Snapshot = game.into();
        assert!(snapshot.game.workings.veins.is_empty());
        let json = serde_json::to_value(snapshot).unwrap();
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
        game.transport.segments[0]
            .batches
            .push(mine_core::transport::Batch {
                route: 0,
                material: 3,
                amount: 1234,
                remaining_ms: 4_098_000,
                duration_ms: 4_098_000,
                legs: vec![leg; 4096],
            });
        let snapshot = Snapshot::from(game.clone());
        assert_eq!(snapshot.shipments[0].legs.len(), 1);
        assert!(snapshot.game.transport.segments[0].legs.is_empty());
        assert!(snapshot.game.transport.segments[0].batches[0]
            .legs
            .is_empty());
        assert_eq!(snapshot.game.transport.mass(), game.transport.mass());
        assert_eq!(game.transport.segments[0].batches[0].legs.len(), 4096);
        assert!(serde_json::to_vec(&snapshot).unwrap().len() < 100_000);
    }
}
