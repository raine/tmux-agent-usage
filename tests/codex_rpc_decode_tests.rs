use serde_json::json;

// Test the RPC response structure parsing by constructing expected Snapshot values
use agent_usage::model::{Credits, ProviderId, Snapshot, Window};

fn snapshot_from_rpc_result(result: &serde_json::Value) -> Snapshot {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    Snapshot {
        provider: ProviderId::Codex,
        primary: result.get("primary").map(|v| Window {
            used_percent: v
                .get("usedPercent")
                .and_then(|v| v.as_f64())
                .map(|n| n.round().clamp(0.0, 100.0) as u8),
            window_minutes: v
                .get("windowDurationMins")
                .and_then(|v| v.as_u64())
                .map(|n| n as u16),
            resets_at_unix: v.get("resetsAt").and_then(|v| v.as_i64()),
        }),
        secondary: result.get("secondary").map(|v| Window {
            used_percent: v
                .get("usedPercent")
                .and_then(|v| v.as_f64())
                .map(|n| n.round().clamp(0.0, 100.0) as u8),
            window_minutes: v
                .get("windowDurationMins")
                .and_then(|v| v.as_u64())
                .map(|n| n as u16),
            resets_at_unix: v.get("resetsAt").and_then(|v| v.as_i64()),
        }),
        credits: result.get("credits").map(|v| Credits {
            remaining: v.get("balance").and_then(|v| v.as_f64()),
            is_unlimited: v
                .get("unlimited")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        }),
        observed_at_unix: now,
    }
}

#[test]
fn decode_complete_response() {
    let result = json!({
        "primary": {"usedPercent": 72.0, "windowDurationMins": 300, "resetsAt": 1700000000},
        "secondary": {"usedPercent": 41.0, "windowDurationMins": 10080, "resetsAt": 1700500000},
        "credits": {"balance": 18.20, "unlimited": false}
    });
    let s = snapshot_from_rpc_result(&result);
    assert_eq!(s.primary.as_ref().unwrap().used_percent, Some(72));
    assert_eq!(s.primary.as_ref().unwrap().window_minutes, Some(300));
    assert_eq!(s.secondary.as_ref().unwrap().used_percent, Some(41));
    assert_eq!(s.secondary.as_ref().unwrap().window_minutes, Some(10080));
    assert_eq!(s.credits.as_ref().unwrap().remaining, Some(18.20));
    assert!(!s.credits.as_ref().unwrap().is_unlimited);
}

#[test]
fn decode_missing_secondary() {
    let result = json!({
        "primary": {"usedPercent": 50.0, "windowDurationMins": 300}
    });
    let s = snapshot_from_rpc_result(&result);
    assert!(s.primary.is_some());
    assert!(s.secondary.is_none());
    assert!(s.credits.is_none());
}

#[test]
fn decode_unlimited_credits() {
    let result = json!({
        "primary": {"usedPercent": 10.0, "windowDurationMins": 300},
        "credits": {"unlimited": true}
    });
    let s = snapshot_from_rpc_result(&result);
    assert!(s.credits.as_ref().unwrap().is_unlimited);
    assert!(s.credits.as_ref().unwrap().remaining.is_none());
}

#[test]
fn decode_empty_result() {
    let result = json!({});
    let s = snapshot_from_rpc_result(&result);
    assert!(s.primary.is_none());
    assert!(s.secondary.is_none());
    assert!(s.credits.is_none());
}

#[test]
fn decode_percent_rounding() {
    let result = json!({
        "primary": {"usedPercent": 72.6}
    });
    let s = snapshot_from_rpc_result(&result);
    assert_eq!(s.primary.as_ref().unwrap().used_percent, Some(73));
}
