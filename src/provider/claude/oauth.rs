use crate::model::{Credits, ProviderId, ScopedWindow, Snapshot, Window};
use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const API_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const BETA_HEADER: &str = "oauth-2025-04-20";
const TIMEOUT: Duration = Duration::from_secs(3);

// --- Credentials ---

#[derive(Deserialize)]
struct CredentialsFile {
    #[serde(rename = "claudeAiOauth")]
    oauth: Option<OAuthEntry>,
    #[serde(rename = "claude.ai:oauth")]
    oauth_legacy: Option<OAuthEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OAuthEntry {
    access_token: String,
    expires_at: Option<f64>, // milliseconds since epoch
}

fn load_credentials() -> Result<String> {
    // Try macOS Keychain first (Claude Code keeps tokens fresh there)
    if let Ok(raw) = load_from_keychain() {
        if let Ok(token) = extract_token(&raw) {
            return Ok(token);
        }
    }

    // Fall back to credentials file
    let path = credentials_path()?;
    let raw =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    extract_token(&raw)
}

fn credentials_path() -> Result<std::path::PathBuf> {
    // Prefer CLAUDE_CONFIG_DIR env var if set and the file exists there
    if let Ok(dir) = std::env::var("CLAUDE_CONFIG_DIR") {
        let candidate = std::path::PathBuf::from(dir).join(".credentials.json");
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    // Default: ~/.claude/.credentials.json
    let home = directories::BaseDirs::new()
        .ok_or_else(|| anyhow!("no home directory"))?
        .home_dir()
        .to_path_buf();
    Ok(home.join(".claude/.credentials.json"))
}

fn load_from_keychain() -> Result<String> {
    let user = std::env::var("USER").context("no USER env var")?;
    let output = std::process::Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            "Claude Code-credentials",
            "-a",
            &user,
            "-w",
        ])
        .output()
        .context("running security command")?;
    if !output.status.success() {
        bail!("no keychain entry found");
    }
    String::from_utf8(output.stdout).context("keychain output not utf8")
}

fn extract_token(raw: &str) -> Result<String> {
    let creds: CredentialsFile = serde_json::from_str(raw).context("parsing credentials")?;
    let entry = creds
        .oauth
        .or(creds.oauth_legacy)
        .ok_or_else(|| anyhow!("no claudeAiOauth entry"))?;

    let token = entry.access_token.trim().to_string();
    if token.is_empty() {
        bail!("empty access token");
    }

    // Check expiry (milliseconds → seconds)
    if let Some(expires_ms) = entry.expires_at {
        let expires_secs = (expires_ms / 1000.0) as u64;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if now >= expires_secs {
            bail!("access token expired");
        }
    }

    Ok(token)
}

// --- API response ---

#[derive(Deserialize)]
struct UsageResponse {
    five_hour: Option<UsageWindow>,
    seven_day: Option<UsageWindow>,
    extra_usage: Option<ExtraUsage>,
    #[serde(default)]
    limits: Vec<LimitEntry>,
}

#[derive(Deserialize)]
struct UsageWindow {
    utilization: Option<f64>,
    resets_at: Option<String>,
}

/// One entry of the API's `limits` array. This is where per-model weekly
/// counters now live: the dedicated `seven_day_opus` / `seven_day_sonnet`
/// fields still exist in the payload but are null, so reading them is no
/// longer enough to see a model-scoped limit.
#[derive(Deserialize)]
struct LimitEntry {
    kind: Option<String>,
    percent: Option<f64>,
    resets_at: Option<String>,
    scope: Option<LimitScope>,
}

#[derive(Deserialize)]
struct LimitScope {
    model: Option<LimitModel>,
}

#[derive(Deserialize)]
struct LimitModel {
    display_name: Option<String>,
}

#[derive(Deserialize)]
struct ExtraUsage {
    is_enabled: Option<bool>,
    monthly_limit: Option<f64>,
    used_credits: Option<f64>,
}

// --- Probe ---

pub fn probe() -> Result<Snapshot> {
    let token = load_credentials()?;

    let agent = ureq::Agent::new_with_config(
        ureq::config::Config::builder()
            .timeout_global(Some(TIMEOUT))
            .build(),
    );
    let resp: UsageResponse = agent
        .get(API_URL)
        .header("Authorization", &format!("Bearer {token}"))
        .header("anthropic-beta", BETA_HEADER)
        .call()
        .context("usage API request failed")?
        .body_mut()
        .read_json()
        .context("parsing usage response")?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    Ok(Snapshot {
        provider: ProviderId::Claude,
        primary: resp.five_hour.map(|w| to_window(w, 300)),
        secondary: resp.seven_day.map(|w| to_window(w, 10080)),
        credits: resp.extra_usage.and_then(to_credits),
        scoped: to_scoped(resp.limits),
        observed_at_unix: now,
    })
}

/// Pull per-model weekly limits out of the `limits` array.
///
/// Only `weekly_scoped` entries are taken: `session` and `weekly_all` restate
/// the `five_hour` and `seven_day` windows already mapped above, so including
/// them would render each figure twice.
fn to_scoped(limits: Vec<LimitEntry>) -> Vec<ScopedWindow> {
    limits
        .into_iter()
        .filter(|l| l.kind.as_deref() == Some("weekly_scoped"))
        .filter_map(|l| {
            let label = l.scope?.model?.display_name?;
            Some(ScopedWindow {
                window: Window {
                    used_percent: l.percent.map(|p| p.round().clamp(0.0, 100.0) as u8),
                    window_minutes: Some(10080),
                    resets_at_unix: l.resets_at.as_deref().and_then(parse_iso8601),
                },
                label,
            })
        })
        .collect()
}

fn to_window(w: UsageWindow, window_minutes: u16) -> Window {
    Window {
        used_percent: w.utilization.map(|u| u.round().clamp(0.0, 100.0) as u8),
        window_minutes: Some(window_minutes),
        resets_at_unix: w.resets_at.as_deref().and_then(parse_iso8601),
    }
}

fn to_credits(eu: ExtraUsage) -> Option<Credits> {
    if eu.is_enabled != Some(true) {
        return None;
    }
    let remaining = match (eu.monthly_limit, eu.used_credits) {
        (Some(limit), Some(used)) => Some(limit - used),
        _ => None,
    };
    Some(Credits {
        remaining,
        is_unlimited: false,
    })
}

fn parse_iso8601(s: &str) -> Option<i64> {
    let s = s.trim();
    let s = s.trim_end_matches('Z').trim_end_matches("+00:00");
    let (date_part, time_part) = s.split_once('T')?;
    let mut date_iter = date_part.split('-');
    let year: i32 = date_iter.next()?.parse().ok()?;
    let month: u32 = date_iter.next()?.parse().ok()?;
    let day: u32 = date_iter.next()?.parse().ok()?;

    let time_clean = time_part.split('.').next()?;
    let mut time_iter = time_clean.split(':');
    let hour: u32 = time_iter.next()?.parse().ok()?;
    let min: u32 = time_iter.next()?.parse().ok()?;
    let sec: u32 = time_iter.next().and_then(|s| s.parse().ok()).unwrap_or(0);

    let days = days_from_civil(year, month, day);
    let ts = days * 86400 + (hour * 3600 + min * 60 + sec) as i64;
    Some(ts)
}

/// Howard Hinnant's civil_from_days algorithm (public domain)
fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = y as i64 - if m <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let m = m as u64;
    let d = d as u64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe as i64 - 719468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_credentials_camel_case() {
        let json = r#"{
            "claudeAiOauth": {
                "accessToken": "sk-ant-oat-test",
                "expiresAt": 4102444800000
            }
        }"#;
        let creds: CredentialsFile = serde_json::from_str(json).unwrap();
        let entry = creds.oauth.unwrap();
        assert_eq!(entry.access_token, "sk-ant-oat-test");
        assert_eq!(entry.expires_at, Some(4102444800000.0));
    }

    #[test]
    fn parse_credentials_legacy_key() {
        let json = r#"{
            "claude.ai:oauth": {
                "accessToken": "sk-ant-oat-legacy",
                "expiresAt": 4102444800000
            }
        }"#;
        let creds: CredentialsFile = serde_json::from_str(json).unwrap();
        let entry = creds.oauth_legacy.unwrap();
        assert_eq!(entry.access_token, "sk-ant-oat-legacy");
    }

    #[test]
    fn parse_credentials_missing_oauth() {
        let json = r#"{}"#;
        let creds: CredentialsFile = serde_json::from_str(json).unwrap();
        assert!(creds.oauth.is_none());
        assert!(creds.oauth_legacy.is_none());
    }

    #[test]
    fn usage_response_full() {
        let json = r#"{
            "five_hour": {"utilization": 28.0, "resets_at": "2026-04-04T15:00:00Z"},
            "seven_day": {"utilization": 63.0, "resets_at": "2026-04-10T00:00:00Z"},
            "extra_usage": {"is_enabled": true, "monthly_limit": 100.0, "used_credits": 42.10}
        }"#;
        let resp: UsageResponse = serde_json::from_str(json).unwrap();
        let w = to_window(resp.five_hour.unwrap(), 300);
        assert_eq!(w.used_percent, Some(28));
        assert_eq!(w.window_minutes, Some(300));
        assert!(w.resets_at_unix.is_some());

        let cr = to_credits(resp.extra_usage.unwrap()).unwrap();
        assert!((cr.remaining.unwrap() - 57.90).abs() < 0.01);
    }

    #[test]
    fn usage_response_partial() {
        let json = r#"{"five_hour": {"utilization": 50.0}}"#;
        let resp: UsageResponse = serde_json::from_str(json).unwrap();
        assert!(resp.seven_day.is_none());
        assert!(resp.extra_usage.is_none());
        let w = to_window(resp.five_hour.unwrap(), 300);
        assert_eq!(w.used_percent, Some(50));
        assert!(w.resets_at_unix.is_none());
    }

    #[test]
    fn extra_usage_disabled() {
        let eu = ExtraUsage {
            is_enabled: Some(false),
            monthly_limit: Some(100.0),
            used_credits: Some(50.0),
        };
        assert!(to_credits(eu).is_none());
    }

    #[test]
    fn parse_iso8601_basic() {
        assert_eq!(parse_iso8601("2026-01-01T00:00:00Z"), Some(1767225600));
    }

    #[test]
    fn parse_iso8601_fractional() {
        assert_eq!(parse_iso8601("2026-01-01T00:00:00.000Z"), Some(1767225600));
    }

    #[test]
    fn scoped_limits_extracted_from_limits_array() {
        // Trimmed shape of a real response: the plan-wide counters are
        // repeated in `limits` as session/weekly_all, and the per-model one
        // arrives as weekly_scoped.
        let json = r#"{
            "five_hour": {"utilization": 26.0, "resets_at": "2026-08-07T17:20:00.489883+00:00"},
            "seven_day": {"utilization": 25.0, "resets_at": "2026-08-13T02:00:00.489908+00:00"},
            "seven_day_opus": null,
            "limits": [
                {"kind": "session", "percent": 26, "resets_at": "2026-08-07T17:20:00.489883+00:00", "scope": null},
                {"kind": "weekly_all", "percent": 25, "resets_at": "2026-08-13T02:00:00.489908+00:00", "scope": null},
                {"kind": "weekly_scoped", "percent": 27, "resets_at": "2026-08-13T02:00:00.490165+00:00",
                 "scope": {"model": {"display_name": "Fable", "id": null}, "surface": null}}
            ]
        }"#;
        let resp: UsageResponse = serde_json::from_str(json).unwrap();
        let scoped = to_scoped(resp.limits);

        // session and weekly_all are dropped: they restate five_hour/seven_day.
        assert_eq!(scoped.len(), 1);
        assert_eq!(scoped[0].label, "Fable");
        assert_eq!(scoped[0].window.used_percent, Some(27));
        assert_eq!(scoped[0].window.window_minutes, Some(10080));
        assert!(scoped[0].window.resets_at_unix.is_some());
    }

    #[test]
    fn scoped_limits_absent_when_limits_missing() {
        let json = r#"{"five_hour": {"utilization": 50.0}}"#;
        let resp: UsageResponse = serde_json::from_str(json).unwrap();
        assert!(to_scoped(resp.limits).is_empty());
    }

    #[test]
    fn scoped_limit_without_model_name_is_skipped() {
        // A scoped limit that names no model has nothing to label a segment
        // with, so it is dropped rather than rendered as an anonymous figure.
        let json = r#"{
            "limits": [
                {"kind": "weekly_scoped", "percent": 40, "scope": {"surface": "code"}},
                {"kind": "weekly_scoped", "percent": 41, "scope": null}
            ]
        }"#;
        let resp: UsageResponse = serde_json::from_str(json).unwrap();
        assert!(to_scoped(resp.limits).is_empty());
    }

    #[test]
    fn scoped_limit_percent_is_rounded_and_clamped() {
        let json = r#"{
            "limits": [
                {"kind": "weekly_scoped", "percent": 26.6, "scope": {"model": {"display_name": "Fable"}}},
                {"kind": "weekly_scoped", "percent": 140.0, "scope": {"model": {"display_name": "Opus"}}}
            ]
        }"#;
        let resp: UsageResponse = serde_json::from_str(json).unwrap();
        let scoped = to_scoped(resp.limits);
        assert_eq!(scoped[0].window.used_percent, Some(27));
        assert_eq!(scoped[1].window.used_percent, Some(100));
    }
}
