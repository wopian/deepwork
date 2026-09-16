mod persistence;
use mine_core::{materials, Action, Game};
use persistence::{load, save};
use std::{
    fs,
    path::PathBuf,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{ipc::Channel, Manager, State};
struct Runtime {
    _save_lock: fs::File,
    game: Mutex<Game>,
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
        let retirement_award = game.award();
        let purchase_blockers = mine_core::requirements()
            .iter()
            .filter_map(|u| {
                game.purchase_blocker(&u.id)
                    .map(|reason| (u.id.clone(), reason))
            })
            .collect();
        Self {
            game,
            quotes,
            retirement_award,
            purchase_blockers,
        }
    }
}
#[derive(Default)]
struct Stream {
    identity: Option<(u32, u64)>,
    chunks: std::collections::BTreeMap<u32, Vec<u8>>,
}
impl Stream {
    fn update(&mut self, g: &Game) -> Update {
        let reset = self.identity != Some((g.site, g.seed));
        let mut state = g.clone();
        if !reset {
            state
                .terrain
                .chunks
                .retain(|id, bytes| self.chunks.get(id) != Some(bytes));
        }
        self.identity = Some((g.site, g.seed));
        self.chunks = g.terrain.chunks.clone();
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
#[tauri::command]
fn command(action: Action, state: State<Runtime>) -> Result<Snapshot, String> {
    let mut g = state.game.lock().map_err(|e| e.to_string())?;
    let mut candidate = g.clone();
    candidate.action(action)?;
    candidate.last_saved = now();
    save(&state.path, &candidate)?;
    *g = candidate;
    Ok(g.clone().into())
}
#[tauri::command]
fn export_save(state: State<Runtime>) -> Result<String, String> {
    serde_json::to_string(&*state.game.lock().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn import_save(data: String, state: State<Runtime>) -> Result<Snapshot, String> {
    let mut game = state.game.lock().map_err(|e| e.to_string())?;
    let mut candidate = persistence::decode(&data)?;
    candidate.advance_offline(now(), &materials());
    save(&state.path, &candidate)?;
    *game = candidate.clone();
    Ok(candidate.into())
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            mine_core::content::validate().map_err(std::io::Error::other)?;
            let dir = app.path().app_data_dir()?;
            // Integration tests use isolated saves; release builds ignore this override.
            #[cfg(debug_assertions)]
            let dir = std::env::var_os("DEEPWORK_TEST_DATA_DIR")
                .map(PathBuf::from)
                .unwrap_or(dir);
            fs::create_dir_all(&dir)?;
            let save_lock =
                persistence::lock(&dir.join("mine.lock")).map_err(std::io::Error::other)?;
            let path = dir.join("mine.json");
            let mut game = if path.exists() {
                load(&path)
                    .or_else(|_| load(&path.with_extension("bak")))
                    .map_err(std::io::Error::other)?
            } else {
                Game::default()
            };
            game.advance_offline(now(), &materials());
            save(&path, &game).map_err(std::io::Error::other)?;
            app.manage(Runtime {
                _save_lock: save_lock,
                game: Mutex::new(game),
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
                        if current.saturating_sub(previous) > 2 {
                            g.last_saved = previous;
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
            import_save
        ])
        .build(tauri::generate_context!())
        .expect("Tauri startup failed")
        .run(|handle, event| {
            if let tauri::RunEvent::Exit = event {
                let state = handle.state::<Runtime>();
                if let Ok(mut g) = state.game.lock() {
                    g.last_saved = now();
                    let _ = save(&state.path, &g);
                };
            }
        });
}

#[cfg(test)]
mod stream_tests {
    use super::*;
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
