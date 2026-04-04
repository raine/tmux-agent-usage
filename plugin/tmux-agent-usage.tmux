#!/usr/bin/env bash
CURRENT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Auto-install binary if not found
if ! command -v agent-usage >/dev/null 2>&1; then
    tmux display-message "agent-usage: installing binary..."
    if bash "$CURRENT_DIR/../scripts/install.sh" >/dev/null 2>&1; then
        tmux display-message "agent-usage: installed successfully"
    else
        tmux display-message "agent-usage: install failed, see https://github.com/raine/tmux-agent-usage"
    fi
fi

# Configurable providers: set -g @agent-usage-providers "codex claude"
# Defaults to "codex" if not set.
providers="$(tmux show-option -gqv @agent-usage-providers)"
providers="${providers:-codex}"

# Display style: set -g @agent-usage-style "compact" (or "default")
style="$(tmux show-option -gqv @agent-usage-style)"
case "$style" in
    compact) style_flag="--compact" ;;
    *)       style_flag="--tmux" ;;
esac

# Build status-right fragments for each provider
fragments=""
for provider in $providers; do
    fragments="${fragments}#($CURRENT_DIR/bin/status.sh $provider $style_flag)"
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
