# agent-usage

Display AI agent rate limit usage in your tmux status bar. Shows session and
weekly utilization for Codex and Claude with color-coded percentages and a
braille reset indicator.

<img src="meta/screenshot.png" alt="agent-usage tmux status bar" width="500" />

- **Green** < 50%, **yellow** 50–79%, **red** 80%+
- Braille character after weekly % shows time until reset (fuller = more time)
- Cache-first with 55s TTL — tmux refreshes stay instant

## Install

```bash
cargo install --path .
```

### tmux setup (manual)

Add to `~/.tmux.conf`:

```tmux
set -g status-right-length 120
set -g status-right '#(agent-usage codex)#(agent-usage claude)#[fg=green]%d.%m. %H:%M'
```

### tmux setup (TPM plugin)

```tmux
# Optional: configure which providers to show (default: codex)
set -g @agent-usage-providers "codex claude"

# Local install:
run-shell /path/to/tmux-agent-usage/plugin/tmux-agent-usage.tmux

# Or via TPM (once published to GitHub):
# set -g @plugin 'raine/tmux-agent-usage'
```

Then reload: `tmux source ~/.tmux.conf`

## Providers

### Codex

Probes rate limits via JSON-RPC by spawning `codex app-server`, with a PTY
fallback that sends `/status` and parses the output. Requires `codex` in PATH
(or set `CODEX_BINARY` env var).

### Claude

Reads OAuth credentials from macOS Keychain (falling back to
`~/.claude/.credentials.json`) and queries the
`api.anthropic.com/api/oauth/usage` endpoint. Requires being logged into Claude
Code.

## Usage

```bash
# Normal usage (prints tmux-formatted status line)
agent-usage codex
agent-usage claude

# Debug mode (prints raw JSON snapshot, bypasses cache)
agent-usage --debug-probe
agent-usage claude --debug-probe

# Provider selection via env var
AGENT_USAGE_PROVIDER=claude agent-usage
```

## How it works

1. **Cache-first**: reads per-provider cache from
   `~/.cache/agent-usage/<provider>.json`. If fresh (< 55s), prints immediately
   and exits.
2. **Stale-serve**: if cache exists but is stale, prints it immediately (no tmux
   lag), then tries to refresh behind a file lock.
3. **Probe**: if no cache exists (first run), probes synchronously.
4. **Locking**: file locks prevent thundering herd from multiple tmux panes.
   Atomic writes via PID-suffixed temp files prevent cache corruption.

## Adding a provider

The architecture is provider-agnostic. To add a new provider:

1. Create `src/provider/<name>/mod.rs` implementing the `Provider` trait
2. Add probe modules as needed
3. Register in `provider::registry()`
4. Cache, formatting, and CLI are automatically per-provider

## Development

```bash
# Run all checks (fmt, clippy, build, test)
just check

# Install debug binary via symlink
just install-dev

# Run directly
cargo run -- codex
cargo run -- claude --debug-probe
```

## Related projects

- [workmux](https://github.com/raine/workmux) — Git worktrees + tmux windows for
  parallel AI agent workflows
- [CodexBar](https://github.com/steipete/CodexBar) — macOS menu bar app for
  agent rate limits (inspiration for this project)
