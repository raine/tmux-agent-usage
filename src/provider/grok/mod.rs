use crate::model::{ProviderId, Snapshot, Window};
use crate::provider::Provider;
use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const API_URL: &str = "https://cli-chat-proxy.grok.com/v1/billing?format=credits";
const TIMEOUT: Duration = Duration::from_secs(3);

pub struct GrokProvider;

impl GrokProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GrokProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Provider for GrokProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Grok
    }

    fn display_name(&self) -> &str {
        "Grok"
    }

    fn refresh(&self) -> Result<Snapshot> {
        probe()
    }
}

#[derive(Deserialize)]
struct AuthEntry {
    key: String,
    expires_at: Option<String>,
}

#[derive(Deserialize)]
struct BillingResponse {
    config: BillingConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BillingConfig {
    credit_usage_percent: Option<f64>,
    billing_period_end: Option<String>,
    current_period: Option<CurrentPeriod>,
    #[serde(default)]
    product_usage: Vec<ProductUsage>,
}

#[derive(Deserialize)]
struct CurrentPeriod {
    end: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProductUsage {
    product: String,
    usage_percent: Option<f64>,
}

pub fn probe() -> Result<Snapshot> {
    let token = load_token()?;
    let agent = ureq::Agent::new_with_config(
        ureq::config::Config::builder()
            .timeout_global(Some(TIMEOUT))
            .build(),
    );
    let resp: BillingResponse = agent
        .get(API_URL)
        .header("Authorization", &format!("Bearer {token}"))
        .header("Accept", "application/json")
        .header("x-xai-token-auth", "xai-grok-cli")
        .call()
        .context("Grok billing API request failed")?
        .body_mut()
        .read_json()
        .context("parsing Grok billing response")?;

    parse_response(resp)
}

fn load_token() -> Result<String> {
    let path = auth_path()?;
    let raw =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    extract_token(&raw)
}

fn auth_path() -> Result<std::path::PathBuf> {
    if let Ok(dir) = std::env::var("GROK_CONFIG_DIR") {
        let candidate = std::path::PathBuf::from(dir).join("auth.json");
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    let home = directories::BaseDirs::new()
        .ok_or_else(|| anyhow!("no home directory"))?
        .home_dir()
        .to_path_buf();
    Ok(home.join(".grok/auth.json"))
}

fn extract_token(raw: &str) -> Result<String> {
    let entries: HashMap<String, AuthEntry> =
        serde_json::from_str(raw).context("parsing auth.json")?;
    let entry = entries
        .values()
        .find(|entry| entry.expires_at.as_deref().and_then(parse_iso8601) != Some(0))
        .ok_or_else(|| anyhow!("Grok auth.json does not include a token"))?;

    if let Some(expires_at) = &entry.expires_at {
        let expires =
            parse_iso8601(expires_at).ok_or_else(|| anyhow!("invalid Grok token expiry"))?;
        let now = now_unix();
        if now >= expires {
            bail!("Grok access token expired");
        }
    }

    let token = entry.key.trim().to_string();
    if token.is_empty() {
        bail!("Grok access token is empty");
    }
    Ok(token)
}

fn parse_response(resp: BillingResponse) -> Result<Snapshot> {
    let config = resp.config;
    let usage = config
        .product_usage
        .iter()
        .find(|usage| usage.product == "GrokBuild")
        .and_then(|usage| usage.usage_percent)
        .or(config.credit_usage_percent);
    let reset = config
        .current_period
        .and_then(|period| period.end)
        .or(config.billing_period_end)
        .as_deref()
        .and_then(parse_iso8601);

    if usage.is_none() && reset.is_none() {
        bail!("Grok billing response did not include usage data");
    }

    Ok(Snapshot {
        provider: ProviderId::Grok,
        primary: None,
        secondary: Some(Window {
            used_percent: usage.map(|u| u.round().clamp(0.0, 100.0) as u8),
            window_minutes: Some(10080),
            resets_at_unix: reset,
        }),
        credits: None,
        scoped: Vec::new(),
        observed_at_unix: now_unix(),
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
    Some(days * 86400 + (hour * 3600 + min * 60 + sec) as i64)
}

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

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_grok_build_usage() {
        let json = r#"{
            "config": {
                "billingPeriodEnd": "2026-07-17T21:10:25.887554+00:00",
                "creditUsagePercent": 3.0,
                "currentPeriod": {
                    "end": "2026-07-17T21:10:25.887554+00:00",
                    "type": "USAGE_PERIOD_TYPE_WEEKLY"
                },
                "productUsage": [
                    {"product": "GrokBuild", "usagePercent": 1.0}
                ]
            }
        }"#;
        let resp: BillingResponse = serde_json::from_str(json).unwrap();
        let snapshot = parse_response(resp).unwrap();

        assert_eq!(snapshot.provider, ProviderId::Grok);
        assert!(snapshot.primary.is_none());
        let secondary = snapshot.secondary.unwrap();
        assert_eq!(secondary.used_percent, Some(1));
        assert_eq!(secondary.window_minutes, Some(10080));
        assert_eq!(secondary.resets_at_unix, Some(1784322625));
    }

    #[test]
    fn falls_back_to_credit_usage_percent() {
        let json = r#"{
            "config": {
                "billingPeriodEnd": "2026-07-17T21:10:25Z",
                "creditUsagePercent": 7.0,
                "productUsage": []
            }
        }"#;
        let resp: BillingResponse = serde_json::from_str(json).unwrap();
        let snapshot = parse_response(resp).unwrap();

        assert_eq!(snapshot.secondary.unwrap().used_percent, Some(7));
    }

    #[test]
    fn extracts_token_from_auth_file() {
        let json = r#"{
            "https://auth.x.ai::client": {
                "key": "test-token",
                "expires_at": "2999-01-01T00:00:00Z"
            }
        }"#;

        assert_eq!(extract_token(json).unwrap(), "test-token");
    }
}
