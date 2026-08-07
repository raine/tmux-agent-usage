use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ProviderId {
    Codex,
    Claude,
    Zai,
    Grok,
}

impl ProviderId {
    pub fn display_name(&self) -> &'static str {
        match self {
            ProviderId::Codex => "Codex",
            ProviderId::Claude => "Claude",
            ProviderId::Zai => "z.ai",
            ProviderId::Grok => "Grok",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Window {
    pub used_percent: Option<u8>,
    pub window_minutes: Option<u16>,
    pub resets_at_unix: Option<i64>,
}

/// A usage window that meters part of a plan rather than the whole of it —
/// currently Claude's per-model weekly limits, which are counted separately
/// from the plan-wide weekly window.
///
/// `label` is the provider's own name for the scope (a model display name, for
/// instance) and is rendered verbatim, so a provider can add or rename scopes
/// without a change here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScopedWindow {
    pub label: String,
    pub window: Window,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Credits {
    pub remaining: Option<f64>,
    pub is_unlimited: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Snapshot {
    pub provider: ProviderId,
    pub primary: Option<Window>,
    pub secondary: Option<Window>,
    pub credits: Option<Credits>,
    /// Extra windows beyond primary/secondary. `serde(default)` keeps cache
    /// files written by older versions readable — without it every user's
    /// cache would fail to deserialize once on upgrade.
    #[serde(default)]
    pub scoped: Vec<ScopedWindow>,
    pub observed_at_unix: i64,
}
