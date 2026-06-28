# Changelog

## v0.1.7 (2026-06-28)

- Keep stale usage data visible when a provider refresh fails, so status output continues to show the last known values.
- Keep Codex status refreshes responsive when probe output times out.

## v0.1.6 (2026-06-27)

- Show reset progress indicators for both 5h and weekly windows in tmux output.

## v0.1.5 (2026-06-27)

- Add z.ai usage support, including 5h and weekly quota windows.
- Respect `CLAUDE_CONFIG_DIR` when reading Claude credentials from a credentials file. ([#1](https://github.com/raine/tmux-agent-usage/pull/1))
- Keep tmux provider separators between providers, with spacing before existing status content. ([#2](https://github.com/raine/tmux-agent-usage/pull/2))
- Reduce repeated refresh attempts when a stale cached provider fails to update.
- Make cached usage data last up to 5 minutes for smoother status refreshes.
- Bound Codex status probing so tmux refreshes stay responsive.

## v0.1.4 (2026-04-05)

- Reset time columns stay aligned even when a window has no reset time available

## v0.1.3 (2026-04-04)

- ANSI output now shows reset time for both the 5h and weekly windows
- ANSI output percentage values are right-aligned for consistent column width
- Weekly window label changed from `wk` to `week` in ANSI output

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
