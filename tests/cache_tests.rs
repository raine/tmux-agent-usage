use agent_usage::cache;
use agent_usage::model::{ProviderId, Snapshot};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn minimal_snapshot(observed_at: i64) -> Snapshot {
    Snapshot {
        provider: ProviderId::Codex,
        primary: None,
        secondary: None,
        credits: None,
        observed_at_unix: observed_at,
    }
}

#[test]
fn fresh_snapshot_within_ttl() {
    let s = minimal_snapshot(now_unix());
    assert!(cache::is_fresh(&s, 55));
}

#[test]
fn stale_snapshot_beyond_ttl() {
    let s = minimal_snapshot(now_unix() - 60);
    assert!(!cache::is_fresh(&s, 55));
}

#[test]
fn future_timestamp_is_stale() {
    let s = minimal_snapshot(i64::MAX);
    assert!(!cache::is_fresh(&s, 55));
}

#[test]
fn exactly_at_ttl_boundary_is_fresh() {
    let s = minimal_snapshot(now_unix() - 55);
    assert!(cache::is_fresh(&s, 55));
}

#[test]
fn save_and_load_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.json");
    let s = minimal_snapshot(now_unix());
    cache::save(&path, &s).unwrap();
    let loaded = cache::load(&path).unwrap();
    assert_eq!(s, loaded);
}

#[test]
fn load_missing_file_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nonexistent.json");
    assert!(cache::load(&path).is_none());
}

#[test]
fn load_corrupt_file_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("corrupt.json");
    std::fs::write(&path, "not json").unwrap();
    assert!(cache::load(&path).is_none());
}
