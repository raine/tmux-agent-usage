use crate::binary;
use crate::model::{Credits, ProviderId, Snapshot, Window};
use anyhow::{anyhow, bail, Result};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::{Read, Write};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const PTY_TIMEOUT: Duration = Duration::from_secs(5);
const PTY_RETRY_TIMEOUT: Duration = Duration::from_secs(4);

pub fn probe_pty() -> Result<Snapshot> {
    match run_pty_probe(60, 200, PTY_TIMEOUT) {
        Ok(s) => Ok(s),
        Err(e) if is_parse_error(&e) => {
            // Retry with larger dimensions (mirrors CodexBar behavior)
            run_pty_probe(70, 220, PTY_RETRY_TIMEOUT)
        }
        Err(e) => Err(e),
    }
}

fn run_pty_probe(rows: u16, cols: u16, timeout: Duration) -> Result<Snapshot> {
    let bin =
        binary::resolve("codex", Some("CODEX_BINARY")).ok_or_else(|| anyhow!("codex not found"))?;
    let pty_system = NativePtySystem::default();
    let pair = pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let cmd = CommandBuilder::new(&bin);
    let _child = pair.slave.spawn_command(cmd)?;
    drop(pair.slave);

    let mut writer = pair.master.take_writer()?;
    let mut reader = pair.master.try_clone_reader()?;

    // Wait briefly for prompt, then send /status
    std::thread::sleep(Duration::from_millis(500));
    writeln!(writer, "/status")?;

    // Read with timeout
    let mut output = String::new();
    let mut buf = [0u8; 4096];
    let deadline = Instant::now() + timeout;

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                output.push_str(&String::from_utf8_lossy(&buf[..n]));
                if output.to_lowercase().contains("weekly limit") {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    drop(writer);
    drop(reader);

    let clean = strip_ansi(&output);
    parse_status_output(&clean)
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() || next == '~' {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn parse_status_output(text: &str) -> Result<Snapshot> {
    let lower = text.to_lowercase();

    // Detect known edge cases before parsing
    if lower.contains("data not available yet") {
        bail!("data not available yet");
    }
    if lower.contains("update required") || lower.contains("please update") {
        bail!("codex update required");
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let five_hour_left = find_percent_for(text, "5h limit");
    let weekly_left = find_percent_for(text, "weekly limit");
    let credits = parse_credits_line(text);

    if five_hour_left.is_none() && weekly_left.is_none() {
        bail!("parse failed: no limit percentages found in output");
    }

    Ok(Snapshot {
        provider: ProviderId::Codex,
        primary: Some(Window {
            // IMPORTANT: /status shows "% left" (remaining), invert to used
            used_percent: five_hour_left.map(|left| 100u8.saturating_sub(left.min(100))),
            window_minutes: Some(300),
            resets_at_unix: None,
        }),
        secondary: Some(Window {
            used_percent: weekly_left.map(|left| 100u8.saturating_sub(left.min(100))),
            window_minutes: Some(10080),
            resets_at_unix: None,
        }),
        credits,
        observed_at_unix: now,
    })
}

fn find_percent_for(text: &str, label: &str) -> Option<u8> {
    for line in text.lines() {
        if line.to_lowercase().contains(label) {
            return parse_percent(line);
        }
    }
    None
}

pub fn parse_percent(line: &str) -> Option<u8> {
    let idx = line.find('%')?;
    let start = line[..idx]
        .rfind(|c: char| !c.is_ascii_digit())
        .map(|i| i + 1)
        .unwrap_or(0);
    line[start..idx].parse::<u8>().ok()
}

fn parse_credits_line(text: &str) -> Option<Credits> {
    for line in text.lines() {
        let lower = line.to_lowercase();
        if lower.contains("credits:") || lower.contains("credit") {
            if lower.contains("unlimited") {
                return Some(Credits {
                    remaining: None,
                    is_unlimited: true,
                });
            }
            if let Some(idx) = line.find('$') {
                let rest = &line[idx + 1..];
                let end = rest
                    .find(|c: char| !c.is_ascii_digit() && c != '.')
                    .unwrap_or(rest.len());
                if let Ok(val) = rest[..end].parse::<f64>() {
                    return Some(Credits {
                        remaining: Some(val),
                        is_unlimited: false,
                    });
                }
            }
        }
    }
    None
}

fn is_parse_error(e: &anyhow::Error) -> bool {
    e.to_string().contains("parse failed")
}
