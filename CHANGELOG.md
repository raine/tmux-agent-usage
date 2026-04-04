# Changelog

## v0.1.2 (2026-04-04)

- ANSI output now shows columns aligned with separator bars for easier reading
- ANSI output includes spark bar usage indicators alongside each tier
- Reset time now shows days and hours (e.g. `2d 3h`) for longer durations
  instead of only hours

## v0.1.1 (2026-04-04)

- Default to ANSI terminal output; add `--tmux` flag for tmux-formatted output
- Show all providers when no argument is given
- Add compact mode (`--compact`) with spark bar visualizations for
  space-efficient status bars
- Make tmux display style configurable via `@agent-usage-style` option
- Improve compact mode

## v0.1.0 (2026-04-04)

Initial release
