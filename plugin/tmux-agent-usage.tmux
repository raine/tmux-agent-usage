#!/usr/bin/env bash
CURRENT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Ensure status-right-length is wide enough to show agent usage
current_len="$(tmux show-option -gqv status-right-length)"
min_len=80
if [ -z "$current_len" ] || [ "$current_len" -lt "$min_len" ] 2>/dev/null; then
    tmux set-option -g status-right-length "$min_len"
fi

tmux set-option -ga status-right " #($CURRENT_DIR/bin/status.sh)"
