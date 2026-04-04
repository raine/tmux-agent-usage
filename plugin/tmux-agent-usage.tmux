#!/usr/bin/env bash
CURRENT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Configurable providers: set -g @agent-usage-providers "codex claude"
# Defaults to "codex" if not set.
providers="$(tmux show-option -gqv @agent-usage-providers)"
providers="${providers:-codex}"

# Build status-right fragments for each provider
fragments=""
for provider in $providers; do
    fragments="${fragments}#($CURRENT_DIR/bin/status.sh $provider)"
done

# Prepend agent usage before existing status-right content (e.g. clock)
current="$(tmux show-option -gqv status-right)"
tmux set-option -g status-right "${fragments}${current}"

# Ensure status-right-length is wide enough
current_len="$(tmux show-option -gqv status-right-length)"
min_len=$(( 40 * $(echo $providers | wc -w) + 40 ))
if [ -z "$current_len" ] || [ "$current_len" -lt "$min_len" ] 2>/dev/null; then
    tmux set-option -g status-right-length "$min_len"
fi
