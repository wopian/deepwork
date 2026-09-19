//! Durable local checkpoints. Keep the last valid primary as backup.
use mine_core::Game;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::Path,
};
pub fn lock(path: &Path) -> Result<File, String> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.try_lock()
        .map_err(|_| "This mine is already open in another game instance".to_string())?;
    Ok(file)
}
pub fn load(path: &Path) -> Result<Game, String> {
    if fs::metadata(path).map_err(|e| e.to_string())?.len() > 32_000_000 {
        return Err("Save exceeds 32 MB".into());
    }
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    decode(&raw)
}
pub fn recover(path: &Path) -> Result<Option<Game>, String> {
    let backup = path.with_extension("bak");
    if !path.exists() && !backup.exists() {
        return Ok(None);
    }
    load(path).or_else(|_| load(&backup)).map(Some)
}
pub fn decode(raw: &str) -> Result<Game, String> {
    if raw.len() > 32_000_000 {
        return Err("Save exceeds 32 MB".into());
    }
    let mut header: serde_json::Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    if header.get("version").and_then(|v| v.as_u64()) != Some(mine_core::VERSION as u64) {
        return Err("Unsupported save version; archive and start a fresh campaign".into());
    }
    let compact = match header.get("terrain_encoding").and_then(|v| v.as_str()) {
        None => false,
        Some("rle-v1") => true,
        _ => return Err("Unsupported terrain encoding".into()),
    };
    let terrain = if compact {
        let source = header.get_mut("terrain").ok_or("Missing terrain")?;
        let mut t = mine_core::terrain::Terrain::default();
        t.revision = source
            .get("revision")
            .and_then(|v| v.as_u64())
            .ok_or("Invalid terrain revision")?;
        t.chunks = decode_chunks(&source["chunks"], 512)?;
        t.revealed = decode_chunks(&source["revealed"], 512)?;
        t.visible = decode_chunks(&source["visible"], 4096)?;
        *source = serde_json::to_value(mine_core::terrain::Terrain::default())
            .map_err(|e| e.to_string())?;
        Some(t)
    } else {
        None
    };
    let mut g: Game = serde_json::from_value(header).map_err(|e| e.to_string())?;
    if let Some(t) = terrain {
        g.terrain = t;
    }
    g.migrate()?;
    Ok(g)
}
/// Disk/export compression does not alter the full versioned IPC snapshot.
pub fn encode(game: &Game) -> Result<String, String> {
    // Avoid expanding full pixel arrays into allocation-heavy serde_json Values.
    let mut metadata = game.clone();
    metadata.terrain = mine_core::terrain::Terrain::default();
    let mut value = serde_json::to_value(&metadata).map_err(|e| e.to_string())?;
    value["terrain"]["revision"] = serde_json::json!(game.terrain.revision);
    value["terrain_encoding"] = serde_json::json!("rle-v1");
    for (name, chunks) in [
        ("chunks", &game.terrain.chunks),
        ("revealed", &game.terrain.revealed),
        ("visible", &game.terrain.visible),
    ] {
        let encoded: std::collections::BTreeMap<_, _> = chunks
            .iter()
            .map(|(id, bytes)| {
                let mut runs: Vec<u64> = Vec::new();
                for &byte in bytes {
                    if runs.last() == Some(&(byte as u64)) {
                        let n = runs.len();
                        runs[n - 2] += 1;
                    } else {
                        runs.extend([1, byte as u64]);
                    }
                }
                (id.to_string(), runs)
            })
            .collect();
        value["terrain"][name] = serde_json::to_value(encoded).map_err(|e| e.to_string())?;
    }
    serde_json::to_string(&value).map_err(|e| e.to_string())
}
fn decode_chunks(
    value: &serde_json::Value,
    length: usize,
) -> Result<std::collections::BTreeMap<i64, Vec<u8>>, String> {
    let map = value.as_object().ok_or("Invalid compact terrain map")?;
    if map.len() > 16384 {
        return Err("Terrain chunk budget exceeded".into());
    }
    map.iter()
        .map(|(key, value)| {
            let id = key.parse::<i64>().map_err(|_| "Invalid terrain key")?;
            if !mine_core::geometry::valid_chunk(id) {
                return Err("Invalid terrain key".into());
            }
            let runs = value.as_array().ok_or("Invalid terrain runs")?;
            if runs.len() % 2 != 0 || runs.len() > length * 2 {
                return Err("Invalid terrain runs".into());
            }
            let mut bytes = Vec::with_capacity(length);
            for pair in runs.chunks_exact(2) {
                let count = pair[0].as_u64().ok_or("Invalid terrain run length")?;
                let byte = pair[1].as_u64().ok_or("Invalid terrain byte")?;
                if count == 0
                    || count > length as u64
                    || byte > 255
                    || bytes.len() + count as usize > length
                {
                    return Err("Invalid terrain run".into());
                }
                bytes.resize(bytes.len() + count as usize, byte as u8);
            }
            if bytes.len() != length {
                return Err("Invalid terrain length".into());
            }
            Ok((id, bytes))
        })
        .collect()
}
pub fn save(path: &Path, game: &Game) -> Result<(), String> {
    game.validate()?;
    let temp = path.with_extension("tmp");
    let backup = path.with_extension("bak");
    let data = encode(game)?.into_bytes();
    if data.len() > 32_000_000 {
        return Err("Save exceeds 32 MB".into());
    }
    {
        let mut f = File::create(&temp).map_err(|e| e.to_string())?;
        f.write_all(&data).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
    }
    // A corrupt primary must never overwrite the only good backup during recovery.
    if path.exists() && load(path).is_ok() {
        fs::copy(path, &backup).map_err(|e| e.to_string())?;
        OpenOptions::new()
            .write(true)
            .open(&backup)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
    }
    fs::rename(&temp, path).map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!(
                "deepwork-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn missing_primary_recovers_previous_good_checkpoint() {
        let t = Temp::new();
        let path = t.0.join("mine.json");
        let mut game = Game::default();
        game.credits = 12;
        save(&path, &game).unwrap();
        game.credits = 34;
        save(&path, &game).unwrap();
        fs::remove_file(&path).unwrap();
        assert_eq!(recover(&path).unwrap().unwrap().credits, 12);
    }
    #[test]
    fn exclusive_lock_releases_on_drop() {
        let t = Temp::new();
        let p = t.0.join("lock");
        let a = lock(&p).unwrap();
        assert!(lock(&p).is_err());
        drop(a);
        assert!(lock(&p).is_ok());
    }
    #[test]
    fn invalid_primary_does_not_replace_good_backup() {
        let t = Temp::new();
        let p = t.0.join("mine.json");
        let g = Game::default();
        save(&p, &g).unwrap();
        save(&p, &g).unwrap();
        fs::write(&p, b"broken").unwrap();
        let recovered = load(&p.with_extension("bak")).unwrap();
        save(&p, &recovered).unwrap();
        assert!(load(&p).is_ok());
        assert!(load(&p.with_extension("bak")).is_ok());
    }
    #[test]
    fn compact_terrain_roundtrip_preserves_masks_and_rejects_expansion() {
        let mut game = Game::default();
        for y in 0..128 {
            game.terrain.excavate(256, y);
            game.heights.insert((256) as i64, y + 1);
        }
        game.excavated = 128;
        game.terrain.reveal(
            game.seed,
            game.profile,
            256,
            120,
            16,
            &mine_core::materials(),
        );
        let encoded = encode(&game).unwrap();
        assert!(encoded.len() < serde_json::to_string(&game).unwrap().len() / 2);
        let restored = decode(&encoded).unwrap();
        assert_eq!(restored.terrain.chunks, game.terrain.chunks);
        assert_eq!(restored.terrain.visible, game.terrain.visible);
        assert_eq!(restored.terrain.revealed, game.terrain.revealed);
        let mut broken: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        broken["terrain"]["chunks"]["4"] = serde_json::json!([999999999, 0]);
        assert!(decode(&broken.to_string()).is_err());
    }
    #[test]
    fn unknown_versions_rejected() {
        let mut g = Game::default();
        g.version = 999;
        assert!(decode(&serde_json::to_string(&g).unwrap()).is_err());
    }
}

/// Inspect the header before attempting to decode an incompatible world.
pub fn legacy_source(path: &Path) -> Result<Option<String>, String> {
    for candidate in [path.to_path_buf(), path.with_extension("bak")] {
        if !candidate.exists() {
            continue;
        }
        if fs::metadata(&candidate).map_err(|e| e.to_string())?.len() > 32_000_000 {
            return Err("Save exceeds 32 MB".into());
        }
        let raw = fs::read_to_string(&candidate).map_err(|e| e.to_string())?;
        if let Ok(header) = serde_json::from_str::<serde_json::Value>(&raw) {
            if let Some(version) = header.get("version").and_then(|v| v.as_u64()) {
                if version > mine_core::VERSION as u64 {
                    return Err("Unsupported newer save version".into());
                }
                if version < mine_core::VERSION as u64 {
                    return Ok(Some(raw));
                }
                if version == mine_core::VERSION as u64 {
                    return Ok(None);
                }
            }
        }
    }
    Ok(None)
}
pub fn archive(path: &Path, raw: &str, id: &str) -> Result<(), String> {
    let directory = path
        .parent()
        .ok_or("Missing save directory")?
        .join("archives");
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join(format!("campaign-{id}.json")))
        .map_err(|e| e.to_string())?;
    file.write_all(raw.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())
}
