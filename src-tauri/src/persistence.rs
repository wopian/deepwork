//! Durable local checkpoints. Keep the last valid primary as backup.
use base64::Engine;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use mine_core::Game;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
const MAGIC: &[u8; 8] = b"DWRKSAVE";
const CONTAINER_VERSION: u16 = 1;
const GZIP_CODEC: u8 = 1;
const HEADER_BYTES: usize = 20;
pub const MAX_CONTAINER_BYTES: usize = 16 * 1024 * 1024;
const MAX_PAYLOAD_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_LEGACY_BYTES: usize = 128 * 1024 * 1024;

#[derive(Clone, Copy, Debug)]
pub struct SaveStats {
    pub bytes: u64,
    pub format: u32,
}

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}
fn recovery_candidates(path: &Path) -> Vec<PathBuf> {
    let mut candidates = vec![path.to_path_buf()];
    if path
        .extension()
        .is_some_and(|extension| extension == "json")
    {
        candidates.push(path.with_extension("bak"));
    }
    candidates.push(sidecar(path, ".bak"));
    candidates
}
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
    let data = fs::read(path).map_err(|e| e.to_string())?;
    decode_bytes(&data)
}
#[cfg(test)]
pub fn recover(path: &Path) -> Result<Option<Game>, String> {
    let backup = sidecar(path, ".bak");
    if !path.exists() && !backup.exists() {
        return Ok(None);
    }
    load(path).or_else(|_| load(&backup)).map(Some)
}
pub fn recover_source(path: &Path) -> Result<Option<(Game, Vec<u8>)>, String> {
    let candidates = recovery_candidates(path);
    if candidates.iter().all(|candidate| !candidate.exists()) {
        return Ok(None);
    }
    for candidate in candidates {
        let Ok(data) = fs::read(&candidate) else {
            continue;
        };
        if let Ok(game) = decode_bytes(&data) {
            return Ok(Some((game, data)));
        }
    }
    Err("Primary and previous-good saves are invalid".into())
}

pub fn decode_bytes(data: &[u8]) -> Result<Game, String> {
    if data.starts_with(MAGIC) {
        return decode_container(data);
    }
    if data.len() > MAX_LEGACY_BYTES {
        return Err("Legacy save exceeds 128 MiB".into());
    }
    let raw = std::str::from_utf8(data).map_err(|_| "Legacy save is not UTF-8")?;
    decode(raw)
}

pub fn decode_import(data: &str, encoding: &str) -> Result<Game, String> {
    match encoding {
        "base64" => {
            let encoded_limit = MAX_CONTAINER_BYTES.div_ceil(3) * 4;
            if data.len() > encoded_limit {
                return Err("Compressed save exceeds 16 MiB".into());
            }
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(data)
                .map_err(|_| "Invalid base64 save data")?;
            decode_bytes(&bytes)
        }
        "json" => {
            if data.len() > MAX_LEGACY_BYTES {
                return Err("Legacy save exceeds 128 MiB".into());
            }
            decode_bytes(data.as_bytes())
        }
        _ => Err("Unsupported import encoding".into()),
    }
}

pub fn encode_container(game: &Game) -> Result<Vec<u8>, String> {
    let payload = encode(game)?.into_bytes();
    if payload.len() > MAX_PAYLOAD_BYTES {
        return Err("Save payload exceeds 64 MiB".into());
    }
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&payload).map_err(|e| e.to_string())?;
    let compressed = encoder.finish().map_err(|e| e.to_string())?;
    let mut container = Vec::with_capacity(HEADER_BYTES + compressed.len());
    container.extend_from_slice(MAGIC);
    container.extend_from_slice(&CONTAINER_VERSION.to_le_bytes());
    container.push(GZIP_CODEC);
    container.push(0);
    container.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    container.extend_from_slice(&compressed);
    if container.len() > MAX_CONTAINER_BYTES {
        return Err("Compressed save exceeds 16 MiB".into());
    }
    Ok(container)
}

fn decode_container(data: &[u8]) -> Result<Game, String> {
    let payload = container_payload(data)?;
    let raw = std::str::from_utf8(&payload).map_err(|_| "Save payload is not UTF-8")?;
    decode(raw)
}

fn container_payload(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() > MAX_CONTAINER_BYTES {
        return Err("Compressed save exceeds 16 MiB".into());
    }
    if data.len() < HEADER_BYTES {
        return Err("Truncated save container".into());
    }
    let version = u16::from_le_bytes([data[8], data[9]]);
    if version != CONTAINER_VERSION {
        return Err("Unsupported save container version".into());
    }
    if data[10] != GZIP_CODEC || data[11] != 0 {
        return Err("Unsupported save compression".into());
    }
    let expected = u64::from_le_bytes(data[12..20].try_into().unwrap());
    if expected > MAX_PAYLOAD_BYTES as u64 {
        return Err("Save payload exceeds 64 MiB".into());
    }
    let mut payload = Vec::with_capacity(expected as usize);
    GzDecoder::new(&data[HEADER_BYTES..])
        .take(expected + 1)
        .read_to_end(&mut payload)
        .map_err(|e| format!("Invalid compressed save: {e}"))?;
    if payload.len() as u64 != expected {
        return Err("Save payload length mismatch".into());
    }
    Ok(payload)
}

pub fn decode(raw: &str) -> Result<Game, String> {
    if raw.len() > MAX_LEGACY_BYTES {
        return Err("Legacy save exceeds 128 MiB".into());
    }
    let mut header: serde_json::Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let version = header.get("version").and_then(|v| v.as_u64());
    if version != Some(10) && version != Some(11) {
        return Err(format!(
            "Unsupported save version {version:?}; archive and start a fresh campaign"
        ));
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
    let encoded = serde_json::to_string(&value).map_err(|e| e.to_string())?;
    if encoded.len() > MAX_PAYLOAD_BYTES {
        return Err("Save payload exceeds 64 MiB".into());
    }
    Ok(encoded)
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
pub fn save(path: &Path, game: &Game) -> Result<SaveStats, String> {
    game.validate()?;
    let temp = sidecar(path, ".tmp");
    let backup = sidecar(path, ".bak");
    let data = encode_container(game)?;
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
    fs::rename(&temp, path).map_err(|e| e.to_string())?;
    Ok(SaveStats {
        bytes: data.len() as u64,
        format: mine_core::VERSION,
    })
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
        let recovered = load(&sidecar(&p, ".bak")).unwrap();
        save(&p, &recovered).unwrap();
        assert!(load(&p).is_ok());
        assert!(load(&sidecar(&p, ".bak")).is_ok());
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
    fn compressed_container_roundtrip_rejects_corruption_and_bombs() {
        let game = Game::default();
        let encoded = encode_container(&game).unwrap();
        assert!(encoded.starts_with(MAGIC));
        assert!(encoded.len() < encode(&game).unwrap().len());
        let restored = decode_bytes(&encoded).unwrap();
        assert_eq!(
            serde_json::to_value(restored).unwrap(),
            serde_json::to_value(game).unwrap()
        );

        let mut corrupt = encoded.clone();
        *corrupt.last_mut().unwrap() ^= 0xff;
        assert!(decode_bytes(&corrupt).is_err());
        assert!(decode_bytes(&encoded[..encoded.len() - 4]).is_err());

        let mut bomb = encoded;
        bomb[12..20].copy_from_slice(&((MAX_PAYLOAD_BYTES as u64) + 1).to_le_bytes());
        assert_eq!(
            decode_bytes(&bomb).err().unwrap(),
            "Save payload exceeds 64 MiB"
        );
    }
    #[test]
    fn version_ten_json_migrates_transport_without_reset() {
        let mut game = Game::default();
        game.version = 10;
        let raw = serde_json::to_vec(&game).unwrap();
        let restored = decode_bytes(&raw).unwrap();
        assert_eq!(restored.version, mine_core::VERSION);
        assert_eq!(restored.campaign_id, game.campaign_id);
        assert!(restored.transport.valid(mine_core::materials().len()));
    }
    #[test]
    fn json_ipc_import_accepts_binary_container_and_legacy_json() {
        let game = Game::default();
        let container = encode_container(&game).unwrap();
        let base64 = base64::engine::general_purpose::STANDARD.encode(container);
        assert_eq!(
            decode_import(&base64, "base64").unwrap().campaign_id,
            game.campaign_id
        );

        let mut old = game.clone();
        old.version = 10;
        let json = serde_json::to_string(&old).unwrap();
        assert_eq!(
            decode_import(&json, "json").unwrap().version,
            mine_core::VERSION
        );
        assert!(decode_import("not base64", "base64").is_err());
        assert!(decode_import(&json, "unknown").is_err());
    }
    #[test]
    fn migrated_save_load_keeps_deterministic_tick_parity() {
        let mut old = Game::default();
        old.version = 10;
        let mut uninterrupted = decode_bytes(&serde_json::to_vec(&old).unwrap()).unwrap();
        let checkpoint = encode_container(&uninterrupted).unwrap();
        let mut restored = decode_bytes(&checkpoint).unwrap();
        for _ in 0..1_000 {
            uninterrupted.tick(&mine_core::materials(), false);
            restored.tick(&mine_core::materials(), false);
        }
        assert_eq!(
            serde_json::to_value(restored).unwrap(),
            serde_json::to_value(uninterrupted).unwrap()
        );
    }
    #[test]
    fn unknown_versions_rejected() {
        let mut g = Game::default();
        g.version = 999;
        assert!(decode(&serde_json::to_string(&g).unwrap()).is_err());
    }
}

/// Find first valid but unsupported checkpoint in normal recovery order. It
/// remains exportable/resettable instead of being mistaken for corruption.
pub fn incompatible_source(path: &Path) -> Result<Option<Vec<u8>>, String> {
    for candidate in recovery_candidates(path) {
        if !candidate.exists() {
            continue;
        }
        let raw = fs::read(&candidate).map_err(|e| e.to_string())?;
        if raw.starts_with(MAGIC) {
            if raw.len() > MAX_CONTAINER_BYTES {
                continue;
            }
            if raw.len() >= 10 && u16::from_le_bytes([raw[8], raw[9]]) != CONTAINER_VERSION {
                return Ok(Some(raw));
            }
        } else if raw.len() > MAX_LEGACY_BYTES {
            continue;
        }
        let payload = if raw.starts_with(MAGIC) {
            let Ok(payload) = container_payload(&raw) else {
                continue;
            };
            payload
        } else {
            raw.clone()
        };
        let Ok(header) = serde_json::from_slice::<serde_json::Value>(&payload) else {
            continue;
        };
        let Some(version) = header.get("version").and_then(|v| v.as_u64()) else {
            continue;
        };
        if !matches!(version, 10 | 11) {
            return Ok(Some(raw));
        }
        if decode_bytes(&raw).is_ok() {
            return Ok(None);
        }
    }
    Ok(None)
}
pub fn archive(path: &Path, raw: &[u8], id: &str) -> Result<(), String> {
    let directory = path
        .parent()
        .ok_or("Missing save directory")?
        .join("archives");
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let extension = if raw.starts_with(MAGIC) {
        "deepwork"
    } else {
        "json"
    };
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join(format!("campaign-{id}.{extension}")))
        .map_err(|e| e.to_string())?;
    file.write_all(raw)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())
}
