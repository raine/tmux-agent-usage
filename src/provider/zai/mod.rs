use crate::model::{ProviderId, Snapshot, Window};
use crate::provider::Provider;
use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const API_URL: &str = "https://api.z.ai/api/monitor/usage/quota/limit";
const TIMEOUT: Duration = Duration::from_secs(3);

pub struct ZaiProvider;

impl ZaiProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ZaiProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Provider for ZaiProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Zai
    }

    fn display_name(&self) -> &str {
        "z.ai"
    }

    fn refresh(&self) -> Result<Snapshot> {
        probe()
    }
}

#[derive(Deserialize)]
struct QuotaResponse {
    success: Option<bool>,
    #[serde(default)]
    msg: Option<String>,
    data: Option<QuotaData>,
}

#[derive(Deserialize)]
struct QuotaData {
    #[serde(default)]
    limits: Vec<Limit>,
}

#[derive(Debug, Clone, Deserialize)]
struct Limit {
    #[serde(rename = "type")]
    kind: String,
    percentage: Option<f64>,
    unit: Option<u8>,
    #[serde(rename = "nextResetTime")]
    next_reset_time: Option<f64>,
}

pub fn probe() -> Result<Snapshot> {
    let token = std::env::var("ZAI_API_TOKEN")
        .or_else(|_| std::env::var("Z_AI_API_KEY"))
        .or_else(|_| std::env::var("ZHIPUAI_API_KEY"))
        .or_else(|_| std::env::var("GLM_API_KEY"))
        .context("ZAI_API_TOKEN not set")?;
    let token = token.trim();
    if token.is_empty() {
        bail!("ZAI_API_TOKEN is empty");
    }

    let agent = ureq::Agent::new_with_config(
        ureq::config::Config::builder()
            .timeout_global(Some(TIMEOUT))
            .build(),
    );
    let resp: QuotaResponse = agent
        .get(API_URL)
        .header("Authorization", &format!("Bearer {token}"))
        .header("Accept", "application/json")
        .call()
        .context("z.ai usage API request failed")?
        .body_mut()
        .read_json()
        .context("parsing z.ai usage response")?;

    parse_response(resp)
}

fn parse_response(resp: QuotaResponse) -> Result<Snapshot> {
    if resp.success == Some(false) {
        bail!("z.ai API error: {}", resp.msg.unwrap_or_default());
    }

    let data = resp
        .data
        .ok_or_else(|| anyhow!("z.ai response missing data"))?;
    let primary = find_limit(&data.limits, Some(3))
        .or_else(|| find_time_limit(&data.limits))
        .map(|limit| to_window(limit, 300));
    let secondary = find_limit(&data.limits, Some(6))
        .or_else(|| longest_token_limit(&data.limits))
        .map(|limit| to_window(limit, 10080));

    if primary.is_none() && secondary.is_none() {
        bail!("z.ai response did not include quota limits");
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    Ok(Snapshot {
        provider: ProviderId::Zai,
        primary,
        secondary,
        credits: None,
        scoped: Vec::new(),
        observed_at_unix: now,
    })
}

fn find_limit(limits: &[Limit], unit: Option<u8>) -> Option<Limit> {
    limits
        .iter()
        .find(|limit| limit.kind == "TOKENS_LIMIT" && limit.unit == unit)
        .cloned()
}

fn find_time_limit(limits: &[Limit]) -> Option<Limit> {
    limits
        .iter()
        .find(|limit| limit.kind == "TIME_LIMIT")
        .cloned()
}

fn longest_token_limit(limits: &[Limit]) -> Option<Limit> {
    limits
        .iter()
        .filter(|limit| limit.kind == "TOKENS_LIMIT")
        .max_by_key(|limit| limit.unit.unwrap_or(0))
        .cloned()
}

fn to_window(limit: Limit, window_minutes: u16) -> Window {
    Window {
        used_percent: limit.percentage.map(|p| p.round().clamp(0.0, 100.0) as u8),
        window_minutes: Some(window_minutes),
        resets_at_unix: limit
            .next_reset_time
            .map(|millis| (millis / 1000.0).round() as i64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_five_hour_and_weekly_token_limits() {
        let json = r#"{
            "success": true,
            "data": {
                "limits": [
                    {"type": "TOKENS_LIMIT", "unit": 3, "percentage": 20, "nextResetTime": 1783028760000},
                    {"type": "TOKENS_LIMIT", "unit": 6, "percentage": 21, "nextResetTime": 1783460760000}
                ]
            }
        }"#;
        let resp: QuotaResponse = serde_json::from_str(json).unwrap();
        let snapshot = parse_response(resp).unwrap();

        assert_eq!(snapshot.provider, ProviderId::Zai);
        assert_eq!(snapshot.primary.unwrap().used_percent, Some(20));
        let secondary = snapshot.secondary.unwrap();
        assert_eq!(secondary.used_percent, Some(21));
        assert_eq!(secondary.window_minutes, Some(10080));
        assert_eq!(secondary.resets_at_unix, Some(1783460760));
    }

    #[test]
    fn uses_time_limit_as_five_hour_fallback() {
        let json = r#"{
            "success": true,
            "data": {
                "limits": [
                    {"type": "TIME_LIMIT", "percentage": 34, "nextResetTime": 1783028760000},
                    {"type": "TOKENS_LIMIT", "percentage": 55, "nextResetTime": 1783460760000}
                ]
            }
        }"#;
        let resp: QuotaResponse = serde_json::from_str(json).unwrap();
        let snapshot = parse_response(resp).unwrap();

        assert_eq!(snapshot.primary.unwrap().used_percent, Some(34));
        assert_eq!(snapshot.secondary.unwrap().used_percent, Some(55));
    }
}
