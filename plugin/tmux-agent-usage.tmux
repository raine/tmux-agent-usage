#!/usr/bin/env bash
CURRENT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Prepend agent usage before existing status-right content (e.g. clock)
current="$(tmux show-option -gqv status-right)"
tmux set-option -g status-right "#($CURRENT_DIR/bin/status.sh) ${current}"

# Ensure status-right-length is wide enough
current_len="$(tmux show-option -gqv status-right-length)"
min_len=80
if [ -z "$current_len" ] || [ "$current_len" -lt "$min_len" ] 2>/dev/null; then
    tmux set-option -g status-right-length "$min_len"
fi
