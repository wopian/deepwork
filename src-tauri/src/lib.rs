mod persistence;
use mine_core::{materials, Action, Game};
use persistence::{recover, save};
use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{ipc::Channel, Manager, State};
struct Runtime {
    _save_lock: fs::File,
    game: Mutex<Game>,
    suspended: AtomicBool,
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
    shipments: Vec<mine_core::transport::VisualCargo>,
    upgrade_previews: std::collections::BTreeMap<String, mine_core::UpgradePreview>,
    purchase_blockers: std::collections::BTreeMap<String, String>,
}
impl From<Game> for Snapshot {
    fn from(game: Game) -> Self {
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
        let retirement_award = game.retirement_quote.unwrap_or_else(|| game.award());
        let purchase_blockers = mine_core::requirements()
            .iter()
            .filter_map(|u| {
                game.purchase_blocker(&u.id)
                    .map(|reason| (u.id.clone(), reason))
            })
            .collect();
        Self {
            requires_reset: game.legacy_pending,
            shipments: game.transport.visual(),
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
    identity: Option<(String, u32, u64)>,
    chunks: std::collections::BTreeMap<u32, Vec<u8>>,
    visible: std::collections::BTreeMap<u32, Vec<u8>>,
    revealed: std::collections::BTreeMap<u32, Vec<u8>>,
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
        Update {
            state: state.into(),
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
fn connect(channel: Channel<Update>, state: State<Runtime>) -> Result<Snapshot, String> {
    *state.channel.lock().map_err(|e| e.to_string())? = Some(channel);
    Ok(state.game.lock().map_err(|e| e.to_string())?.clone().into())
}
/// Persist before publishing either transition. Repeated platform/webview events
/// are idempotent; a crash after resume cannot grant the same interval again.
fn transition_background(
    state: &Runtime,
    background: bool,
    timestamp: u64,
) -> Result<Snapshot, String> {
    let mut game = state.game.lock().map_err(|e| e.to_string())?;
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
fn set_background(background: bool, state: State<Runtime>) -> Result<Snapshot, String> {
    transition_background(&state, background, now())
}
#[tauri::command]
fn command(action: Action, campaign_id: String, state: State<Runtime>) -> Result<Snapshot, String> {
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
    serde_json::to_string(&*state.game.lock().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
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
        None => serde_json::to_string(&*game).map_err(|e| e.to_string())?,
    };
    let seed = fresh_identity();
    let mut candidate = Game::new(seed, 1);
    candidate.last_saved = now();
    persistence::archive(&state.path, &raw, &seed.to_string())?;
    save(&state.path, &candidate)?;
    *game = candidate;
    *legacy = None;
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
            if !game.legacy_pending {
                game.advance_offline(now(), &materials());
                save(&path, &game).map_err(std::io::Error::other)?;
            }
            app.manage(Runtime {
                _save_lock: save_lock,
                game: Mutex::new(game),
                suspended: AtomicBool::new(false),
                legacy: Mutex::new(legacy),
                path,
                channel: Mutex::new(None),
            });
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
                        previous = current;
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
                    if let Err(e) =
                        transition_background(&handle.state::<Runtime>(), background, now())
                    {
                        eprintln!("Lifecycle checkpoint failed: {e}");
                    }
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
            legacy: Mutex::new(None),
            path: directory.join("mine.json"),
            channel: Mutex::new(None),
        };
        state.game.lock().unwrap().cooldowns = [100; 4];
        transition_background(&state, true, 100).unwrap();
        transition_background(&state, true, 110).unwrap();
        assert_eq!(state.game.lock().unwrap().last_saved, 100);
        let resumed = transition_background(&state, false, 120).unwrap();
        assert_eq!(resumed.game.offline.as_ref().unwrap().effective, 10);
        assert_eq!(resumed.game.ticks, 200);
        assert_eq!(resumed.game.cooldowns, [100; 4]);
        let repeated = transition_background(&state, false, 140).unwrap();
        assert_eq!(repeated.game.ticks, 200);
        let recovered = recover(&state.path).unwrap().unwrap();
        assert_eq!(recovered.last_saved, 120);
        assert_eq!(recovered.ticks, 200);
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
