pub mod pty;
pub mod rpc;

use crate::model::{ProviderId, Snapshot};
use crate::provider::Provider;
use anyhow::{Context, Result};

pub struct CodexProvider;

impl Default for CodexProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Provider for CodexProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Codex
    }

    fn display_name(&self) -> &str {
        "Codex"
    }

    fn refresh(&self) -> Result<Snapshot> {
        match rpc::probe_rpc() {
            Ok(s) => Ok(s),
            Err(rpc_err) => pty::probe_pty().with_context(|| format!("rpc failed: {rpc_err}")),
        }
    }
}
