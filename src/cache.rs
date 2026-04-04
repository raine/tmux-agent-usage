use crate::model::{ProviderId, Snapshot};
use anyhow::Result;
use directories::BaseDirs;
use fs2::FileExt;
use std::{
    fs,
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn cache_dir() -> PathBuf {
    let base = BaseDirs::new().expect("home directory required");
    base.cache_dir().join("agent-usage")
}

pub fn path_for(provider: ProviderId) -> PathBuf {
    let name = match provider {
        ProviderId::Codex => "codex",
        ProviderId::Claude => "claude",
    };
    cache_dir().join(format!("{name}.json"))
}

pub fn lock_path(cache_path: &Path) -> PathBuf {
    cache_path.with_extension("lock")
}

pub fn try_lock(lock_path: &Path) -> Option<fs::File> {
    if let Some(parent) = lock_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(lock_path)
        .ok()?;
    file.try_lock_exclusive().ok()?;
    Some(file)
}

pub fn load(path: &Path) -> Option<Snapshot> {
    let raw = fs::read_to_string(path).ok()?;
    match serde_json::from_str::<Snapshot>(&raw) {
        Ok(snapshot) => Some(snapshot),
        Err(e) => {
            eprintln!("agent-usage: cache decode error: {e}");
            None
        }
    }
}

pub fn save(path: &Path, snapshot: &Snapshot) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!("tmp.{}", process::id()));
    fs::write(&tmp, serde_json::to_vec(snapshot)?)?;
    fs::rename(tmp, path)?;
    Ok(())
}

pub fn is_fresh(snapshot: &Snapshot, ttl_secs: u64) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let observed = snapshot.observed_at_unix as u64;
    let age = now.saturating_sub(observed);
    // Reject future timestamps (observed_at should never exceed now).
    age <= ttl_secs && observed <= now
}
