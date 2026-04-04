#!/usr/bin/env bash
# No set -e: nonzero exit from agent-usage should fall through to fallback,
# not produce blank tmux output.

PROVIDER="${1:-codex}"

if command -v agent-usage >/dev/null 2>&1; then
  agent-usage "$PROVIDER" --tmux || printf '%s n/a\n' "$PROVIDER"
else
  printf '%s n/a\n' "$PROVIDER"
fi
