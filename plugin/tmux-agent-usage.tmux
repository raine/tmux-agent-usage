#!/usr/bin/env bash
CURRENT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tmux set-option -ga status-right " #($CURRENT_DIR/bin/status.sh)"
