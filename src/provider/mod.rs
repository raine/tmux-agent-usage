pub mod claude;
pub mod codex;
pub mod grok;
pub mod zai;

use crate::model::{ProviderId, Snapshot};
use anyhow::Result;

pub trait Provider {
    fn id(&self) -> ProviderId;
    fn display_name(&self) -> &str;
    fn refresh(&self) -> Result<Snapshot>;
    fn ttl_secs(&self) -> u64 {
        300
    }
}

pub fn registry() -> Vec<Box<dyn Provider>> {
    vec![
        Box::new(codex::CodexProvider::new()),
        Box::new(claude::ClaudeProvider::new()),
        Box::new(zai::ZaiProvider::new()),
        Box::new(grok::GrokProvider::new()),
    ]
}

pub fn by_id(id: ProviderId) -> Option<Box<dyn Provider>> {
    registry().into_iter().find(|p| p.id() == id)
}

pub fn parse_provider_arg(arg: &str) -> Option<ProviderId> {
    match arg.to_lowercase().as_str() {
        "codex" => Some(ProviderId::Codex),
        "claude" => Some(ProviderId::Claude),
        "zai" | "z.ai" => Some(ProviderId::Zai),
        "grok" | "grog" => Some(ProviderId::Grok),
        _ => None,
    }
}
