use crate::binary;
use crate::model::{Credits, ProviderId, Snapshot, Window};
use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const RPC_TIMEOUT: Duration = Duration::from_secs(3);

pub fn probe_rpc() -> Result<Snapshot> {
    let bin =
        binary::resolve("codex", Some("CODEX_BINARY")).ok_or_else(|| anyhow!("codex not found"))?;

    let mut child = Command::new(&bin)
        .args(["-s", "read-only", "-a", "untrusted", "app-server"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let mut stdin = child.stdin.take().ok_or_else(|| anyhow!("no stdin"))?;
    let stdout = child.stdout.take().ok_or_else(|| anyhow!("no stdout"))?;

    // Reader thread: parse JSON lines, forward responses (have "id" field)
    let (tx, rx) = mpsc::channel::<Value>();
    let reader_handle = thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if let Ok(msg) = serde_json::from_str::<Value>(&line) {
                if msg.get("id").is_some() && tx.send(msg).is_err() {
                    break;
                }
            }
        }
    });

    // initialize (id: 1)
    let init_req = json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "initialize",
        "params": {"clientInfo": {"name": "agent-usage", "version": "0.1.0"}}
    });
    writeln!(stdin, "{}", init_req)?;
    let _init_resp = recv_response(&rx, 1, RPC_TIMEOUT)?;

    // initialized notification (no id)
    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc": "2.0", "method": "initialized", "params": {}})
    )?;

    // rateLimits (id: 2)
    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc": "2.0", "id": 2, "method": "account/rateLimits/read", "params": {}})
    )?;
    let limits_resp = recv_response(&rx, 2, RPC_TIMEOUT)?;

    // Clean up
    drop(stdin);
    let _ = child.kill();
    let _ = reader_handle.join();

    parse_limits_response(&limits_resp)
}

fn recv_response(rx: &mpsc::Receiver<Value>, expected_id: u64, timeout: Duration) -> Result<Value> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            bail!("RPC timeout waiting for response id={expected_id}");
        }
        match rx.recv_timeout(remaining) {
            Ok(msg) => {
                if msg.get("id").and_then(|v| v.as_u64()) == Some(expected_id) {
                    return Ok(msg);
                }
            }
            Err(_) => bail!("RPC timeout waiting for response id={expected_id}"),
        }
    }
}

fn parse_limits_response(resp: &Value) -> Result<Snapshot> {
    let result = resp.get("result").ok_or_else(|| {
        let err = resp.get("error");
        anyhow!("RPC error: {}", err.unwrap_or(&Value::Null))
    })?;

    // Data is nested under result.rateLimits
    let limits = result.get("rateLimits").unwrap_or(result);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    Ok(Snapshot {
        provider: ProviderId::Codex,
        primary: limits.get("primary").map(parse_window),
        secondary: limits.get("secondary").map(parse_window),
        credits: limits.get("credits").map(parse_credits),
        observed_at_unix: now,
    })
}

fn parse_window(v: &Value) -> Window {
    Window {
        used_percent: v.get("usedPercent").and_then(percent_from_value),
        window_minutes: v
            .get("windowDurationMins")
            .and_then(|v| v.as_u64().map(|n| n as u16)),
        resets_at_unix: v.get("resetsAt").and_then(|v| v.as_i64()),
    }
}

fn parse_credits(v: &Value) -> Credits {
    let balance = v.get("balance").and_then(|b| {
        // balance can be a number or a string
        b.as_f64()
            .or_else(|| b.as_str().and_then(|s| s.parse::<f64>().ok()))
    });
    Credits {
        remaining: balance,
        is_unlimited: v
            .get("unlimited")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    }
}

fn percent_from_value(v: &Value) -> Option<u8> {
    v.as_f64().map(|n| n.round().clamp(0.0, 100.0) as u8)
}
