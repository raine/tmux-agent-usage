use crate::model::{ProviderId, ScopedWindow, Snapshot, Window};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorMode {
    Tmux,
    TmuxCompact,
    Ansi,
}

// Spark bars: 8 levels from low to full
const SPARK_LEVELS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

struct Theme {
    dim: &'static str,
    /// Dividers only. Brighter than `dim`, which at colour245 sits close
    /// enough to a dark status-bar background (dracula's is #282a36) to be
    /// hard to make out, and a divider nobody can see is not doing its job.
    divider: &'static str,
    claude_orange: &'static str,
    green: &'static str,
    yellow: &'static str,
    red: &'static str,
    reset: &'static str,
}

const TMUX_THEME: Theme = Theme {
    dim: "#[fg=colour245]",
    divider: "#[fg=colour250]",
    claude_orange: "#[fg=#d97757]",
    green: "#[fg=colour114]",
    yellow: "#[fg=colour221]",
    red: "#[fg=colour203]",
    reset: "",
};

const ANSI_THEME: Theme = Theme {
    dim: "\x1b[38;5;245m",
    divider: "\x1b[38;5;250m",
    claude_orange: "\x1b[38;2;217;119;87m",
    green: "\x1b[38;5;114m",
    yellow: "\x1b[38;5;221m",
    red: "\x1b[38;5;203m",
    reset: "\x1b[0m",
};

// Braille characters: 8 levels from empty to full (bottom-up fill)
const BRAILLE_LEVELS: &[char] = &['⠀', '⡀', '⡄', '⡆', '⡇', '⣇', '⣧', '⣷', '⣿'];

fn theme(mode: ColorMode) -> &'static Theme {
    match mode {
        ColorMode::Tmux | ColorMode::TmuxCompact => &TMUX_THEME,
        ColorMode::Ansi => &ANSI_THEME,
    }
}

fn short_name(provider: ProviderId) -> &'static str {
    match provider {
        ProviderId::Codex => "O",
        ProviderId::Claude => "C",
        ProviderId::Zai => "Z",
        ProviderId::Grok => "G",
    }
}

fn percent_spark(pct: u8, t: &Theme) -> String {
    let idx = (pct as usize * (SPARK_LEVELS.len() - 1)) / 100;
    let ch = SPARK_LEVELS[idx];
    format!("{}{ch}", percent_color(pct, t))
}

/// Spark bar for time remaining (inverted: more time left = taller bar).
/// Uses dim color since it's supplementary info.
fn reset_spark(window: Option<&Window>, t: &Theme) -> String {
    let Some(w) = window else {
        return format!("{}▁", t.dim);
    };
    let Some(resets_at) = w.resets_at_unix else {
        return format!("{}▁", t.dim);
    };
    let Some(window_mins) = w.window_minutes else {
        return format!("{}▁", t.dim);
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let remaining_secs = (resets_at - now).max(0) as f64;
    let total_secs = window_mins as f64 * 60.0;
    let fraction = (remaining_secs / total_secs).clamp(0.0, 1.0);

    let idx = (fraction * (SPARK_LEVELS.len() - 1) as f64).round() as usize;
    let ch = SPARK_LEVELS[idx];
    format!("{}{ch}", t.dim)
}

fn percent_color(pct: u8, t: &Theme) -> &'static str {
    match pct {
        0..=49 => t.green,
        50..=79 => t.yellow,
        _ => t.red,
    }
}

fn window_label(minutes: Option<u16>, fallback: &str) -> &str {
    match minutes {
        Some(300) => "5h",
        Some(10080) => "wk",
        _ => fallback,
    }
}

fn window_label_long(minutes: Option<u16>, fallback: &str) -> &str {
    match minutes {
        Some(300) => "  5h",
        Some(10080) => "week",
        _ => fallback,
    }
}

fn render_percent(pct: Option<u8>, t: &Theme) -> String {
    match pct {
        Some(v) => format!("{}{v}%", percent_color(v, t)),
        None => format!("{}n/a", t.dim),
    }
}

fn render_percent_aligned(pct: Option<u8>, t: &Theme) -> String {
    match pct {
        Some(v) => format!("{}{v:>3}%", percent_color(v, t)),
        None => format!("{} n/a", t.dim),
    }
}

fn reset_indicator(window: Option<&Window>, t: &Theme) -> String {
    let Some(w) = window else {
        return String::new();
    };
    let Some(resets_at) = w.resets_at_unix else {
        return String::new();
    };
    let Some(window_mins) = w.window_minutes else {
        return String::new();
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let remaining_secs = (resets_at - now).max(0) as f64;
    let total_secs = window_mins as f64 * 60.0;
    let fraction = (remaining_secs / total_secs).clamp(0.0, 1.0);

    let idx = (fraction * (BRAILLE_LEVELS.len() - 1) as f64).round() as usize;
    let ch = BRAILLE_LEVELS[idx];

    format!(" {}{ch}", t.dim)
}

fn format_time_remaining(resets_at: i64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let remaining = (resets_at - now).max(0);
    let days = remaining / 86400;
    let hours = (remaining % 86400) / 3600;
    let minutes = (remaining % 3600) / 60;
    let s = if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {minutes:02}m")
    } else {
        format!("{minutes}m")
    };
    format!("{s:>6}")
}

pub fn render(snapshot: Option<&Snapshot>) -> String {
    render_with_mode(snapshot, ColorMode::Tmux)
}

pub fn render_with_mode(snapshot: Option<&Snapshot>, mode: ColorMode) -> String {
    let t = theme(mode);
    let Some(s) = snapshot else {
        return format!("{}n/a{}", t.dim, t.reset);
    };

    let name = s.provider.display_name();
    let name_color = match s.provider {
        ProviderId::Claude => t.claude_orange,
        _ => t.dim,
    };

    let pri_label = window_label(s.primary.as_ref().and_then(|w| w.window_minutes), "pri");
    let sec_label = window_label(s.secondary.as_ref().and_then(|w| w.window_minutes), "sec");

    let pri = render_percent(s.primary.as_ref().and_then(|w| w.used_percent), t);
    let sec = render_percent(s.secondary.as_ref().and_then(|w| w.used_percent), t);
    let pri_reset = reset_indicator(s.primary.as_ref(), t);
    let sec_reset = reset_indicator(s.secondary.as_ref(), t);

    match mode {
        ColorMode::TmuxCompact => {
            let short = short_name(s.provider);
            if s.provider == ProviderId::Grok {
                let sec_spark = s
                    .secondary
                    .as_ref()
                    .and_then(|w| w.used_percent)
                    .map(|p| percent_spark(p, t))
                    .unwrap_or_else(|| format!("{}·", t.dim));
                let sec_rst = reset_spark(s.secondary.as_ref(), t);
                return format!("{name_color}{short} {sec_spark}{sec_rst}");
            }

            let windows = [s.primary.as_ref(), s.secondary.as_ref()];
            let pri_window = windows
                .iter()
                .flatten()
                .find(|w| w.window_minutes == Some(300))
                .copied();
            let sec_window = windows
                .iter()
                .flatten()
                .find(|w| w.window_minutes == Some(10080))
                .copied();
            let pri_spark = pri_window
                .and_then(|w| w.used_percent)
                .map(|p| percent_spark(p, t))
                .unwrap_or_else(|| format!("{}·", t.dim));
            let sec_spark = sec_window
                .and_then(|w| w.used_percent)
                .map(|p| percent_spark(p, t))
                .unwrap_or_else(|| format!("{}·", t.dim));
            let pri_rst = reset_spark(pri_window, t);
            let sec_rst = reset_spark(sec_window, t);
            // Scoped windows get a spark pair each with no label. Compact
            // mode is glyph-only by design, and their order is the API's.
            let scoped: String = s
                .scoped
                .iter()
                .map(|sw| {
                    let spark = sw
                        .window
                        .used_percent
                        .map(|p| percent_spark(p, t))
                        .unwrap_or_else(|| format!("{}·", t.dim));
                    format!("{spark}{}", reset_spark(Some(&sw.window), t))
                })
                .collect();
            format!("{name_color}{short} {pri_spark}{pri_rst}{sec_spark}{sec_rst}{scoped}")
        }
        ColorMode::Tmux => {
            let scoped = render_scoped_tmux(&s.scoped, t);
            if s.primary.is_none() && s.secondary.is_some() {
                return format!(
                    "{name_color}{name} {}{sec_label}:{sec}{sec_reset}{scoped}",
                    t.dim
                );
            }
            format!(
                "{name_color}{name} {}{pri_label}:{pri}{pri_reset} {}{sec_label}:{sec}{sec_reset}{scoped}",
                t.dim, t.dim
            )
        }
        ColorMode::Ansi => {
            let padded_name = format!("{name:7}");
            if s.primary.is_none() && s.secondary.is_some() {
                let sec_label_long =
                    window_label_long(s.secondary.as_ref().and_then(|w| w.window_minutes), "sec");
                let sec_a =
                    render_percent_aligned(s.secondary.as_ref().and_then(|w| w.used_percent), t);
                let sec_spark = s
                    .secondary
                    .as_ref()
                    .and_then(|w| w.used_percent)
                    .map(|p| format!(" {}", percent_spark(p, t)))
                    .unwrap_or_default();
                let sec_reset = s
                    .secondary
                    .as_ref()
                    .and_then(|w| w.resets_at_unix)
                    .map(|r| format!(" {}↻ {}", t.dim, format_time_remaining(r)))
                    .unwrap_or_else(|| " ".repeat(9));
                let scoped = render_scoped_ansi(&s.scoped, t);
                return format!(
                    "{name_color}{padded_name} {}│ {}{sec_label_long} {sec_a}{sec_spark}{sec_reset}{scoped}{}",
                    t.dim, t.dim, t.reset
                );
            }
            let pri_label_long =
                window_label_long(s.primary.as_ref().and_then(|w| w.window_minutes), "pri");
            let sec_label_long =
                window_label_long(s.secondary.as_ref().and_then(|w| w.window_minutes), "sec");
            let pri_a = render_percent_aligned(s.primary.as_ref().and_then(|w| w.used_percent), t);
            let sec_a =
                render_percent_aligned(s.secondary.as_ref().and_then(|w| w.used_percent), t);

            let pri_spark = s
                .primary
                .as_ref()
                .and_then(|w| w.used_percent)
                .map(|p| format!(" {}", percent_spark(p, t)))
                .unwrap_or_default();

            let sec_spark = s
                .secondary
                .as_ref()
                .and_then(|w| w.used_percent)
                .map(|p| format!(" {}", percent_spark(p, t)))
                .unwrap_or_default();

            let pri_reset = s
                .primary
                .as_ref()
                .and_then(|w| w.resets_at_unix)
                .map(|r| format!(" {}↻ {}", t.dim, format_time_remaining(r)))
                .unwrap_or_else(|| " ".repeat(9));

            let sec_reset = s
                .secondary
                .as_ref()
                .and_then(|w| w.resets_at_unix)
                .map(|r| format!(" {}↻ {}", t.dim, format_time_remaining(r)))
                .unwrap_or_else(|| " ".repeat(9));

            let scoped = render_scoped_ansi(&s.scoped, t);
            format!(
                "{name_color}{padded_name} {}│ {}{pri_label_long} {pri_a}{pri_spark}{pri_reset} {}│ {}{sec_label_long} {sec_a}{sec_spark}{sec_reset}{scoped}{}",
                t.dim, t.dim, t.dim, t.dim, t.reset
            )
        }
    }
}

/// Scoped windows for the tmux status line: ` │ Fable:27% ⣧`.
///
/// The scope label identifies each per-model weekly limit, so repeating "wk"
/// for each window would be noise.
///
/// A single `│` marks the boundary between plan-wide and scoped windows,
/// matching the column separators in ANSI mode.
fn render_scoped_tmux(scoped: &[ScopedWindow], t: &Theme) -> String {
    if scoped.is_empty() {
        return String::new();
    }
    let windows: String = scoped
        .iter()
        .map(|sw| {
            format!(
                " {}{}:{}{}",
                t.dim,
                sw.label,
                render_percent(sw.window.used_percent, t),
                reset_indicator(Some(&sw.window), t)
            )
        })
        .collect();
    format!(" {}│{windows}", t.divider)
}

/// Scoped windows for the wide ANSI line, matching the column layout of the
/// primary and secondary windows.
fn render_scoped_ansi(scoped: &[ScopedWindow], t: &Theme) -> String {
    scoped
        .iter()
        .map(|sw| {
            let pct = render_percent_aligned(sw.window.used_percent, t);
            let spark = sw
                .window
                .used_percent
                .map(|p| format!(" {}", percent_spark(p, t)))
                .unwrap_or_default();
            let reset = sw
                .window
                .resets_at_unix
                .map(|r| format!(" {}↻ {}", t.dim, format_time_remaining(r)))
                .unwrap_or_else(|| " ".repeat(9));
            format!(" {}│ {}{:4} {pct}{spark}{reset}", t.dim, t.dim, sw.label)
        })
        .collect()
}

/// Render a failure line for a specific provider.
pub fn render_unavailable(name: &str) -> String {
    render_unavailable_with_mode(name, ColorMode::Tmux)
}

pub fn render_unavailable_with_mode(name: &str, mode: ColorMode) -> String {
    let t = theme(mode);
    match mode {
        ColorMode::TmuxCompact => format!("{}{} ·", t.dim, name),
        ColorMode::Tmux => format!("{}{}  n/a", t.dim, name),
        ColorMode::Ansi => format!("{}{:7} │ n/a{}", t.dim, name, t.reset),
    }
}
