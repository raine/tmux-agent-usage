pub mod oauth;

use crate::model::{ProviderId, Snapshot};
use crate::provider::Provider;
use anyhow::Result;

pub struct ClaudeProvider;

impl ClaudeProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClaudeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Provider for ClaudeProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Claude
    }

    fn display_name(&self) -> &str {
        "Claude"
    }

    fn refresh(&self) -> Result<Snapshot> {
        oauth::probe()
    }
}
