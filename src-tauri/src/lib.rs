use mine_core::{materials, Action, Game};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{ipc::Channel, Manager, State};
struct Runtime {
    game: Mutex<Game>,
    path: PathBuf,
    channel: Mutex<Option<Channel<Game>>>,
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn save(path: &Path, game: &Game) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    let backup = path.with_extension("bak");
    let data = serde_json::to_vec(game).map_err(|e| e.to_string())?;
    fs::write(&tmp, data).map_err(|e| e.to_string())?;
    if path.exists() {
        fs::copy(path, &backup).map_err(|e| e.to_string())?;
    }
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}
fn load(path: &Path) -> Result<Game, String> {
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut g: Game = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    g.migrate()?;
    Ok(g)
}
#[tauri::command]
fn connect(channel: Channel<Game>, state: State<Runtime>) -> Result<Game, String> {
    *state.channel.lock().map_err(|e| e.to_string())? = Some(channel);
    Ok(state.game.lock().map_err(|e| e.to_string())?.clone())
}
#[tauri::command]
fn command(action: Action, state: State<Runtime>) -> Result<Game, String> {
    let mut g = state.game.lock().map_err(|e| e.to_string())?;
    g.action(action)?;
    g.last_saved = now();
    save(&state.path, &g)?;
    Ok(g.clone())
}
#[tauri::command]
fn export_save(state: State<Runtime>) -> Result<String, String> {
    serde_json::to_string(&*state.game.lock().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn import_save(data: String, state: State<Runtime>) -> Result<Game, String> {
    if data.len() > 32_000_000 {
        return Err("Save exceeds 32 MB".into());
    }
    let mut candidate: Game = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    candidate.migrate()?;
    candidate.advance_offline(now(), &materials());
    save(&state.path, &candidate)?;
    *state.game.lock().map_err(|e| e.to_string())? = candidate.clone();
    Ok(candidate)
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            fs::create_dir_all(&dir)?;
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
                game: Mutex::new(game),
                path,
                channel: Mutex::new(None),
            });
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let cat = materials();
                let mut previous = now();
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    let state = handle.state::<Runtime>();
                    if let Ok(mut g) = state.game.lock() {
                        let current = now();
                        if current.saturating_sub(previous) > 2 {
                            g.last_saved = previous;
                            g.advance_offline(current, &cat);
                            if let Err(e) = save(&state.path, &g) {
                                eprintln!("Resume save failed: {e}")
                            }
                        } else {
                            g.second(&cat, false);
                        }
                        previous = current;
                        if g.ticks % 600 == 0 {
                            g.last_saved = now();
                            if let Err(e) = save(&state.path, &g) {
                                eprintln!("Save failed: {e}")
                            }
                        }
                        if let Ok(channel) = state.channel.lock() {
                            if let Some(c) = channel.as_ref() {
                                let _ = c.send(g.clone());
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
