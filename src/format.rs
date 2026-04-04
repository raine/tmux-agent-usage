use crate::model::{ProviderId, Snapshot, Window};
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
    claude_orange: &'static str,
    green: &'static str,
    yellow: &'static str,
    red: &'static str,
    reset: &'static str,
}

const TMUX_THEME: Theme = Theme {
    dim: "#[fg=colour245]",
    claude_orange: "#[fg=#d97757]",
    green: "#[fg=colour114]",
    yellow: "#[fg=colour221]",
    red: "#[fg=colour203]",
    reset: "",
};

const ANSI_THEME: Theme = Theme {
    dim: "\x1b[38;5;245m",
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

fn render_percent(pct: Option<u8>, t: &Theme) -> String {
    match pct {
        Some(v) => format!("{}{v}%", percent_color(v, t)),
        None => format!("{}n/a", t.dim),
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
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {minutes:02}m")
    } else {
        format!("{minutes}m")
    }
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
    let reset = reset_indicator(s.secondary.as_ref(), t);

    match mode {
        ColorMode::TmuxCompact => {
            let short = short_name(s.provider);
            let pri_spark = s
                .primary
                .as_ref()
                .and_then(|w| w.used_percent)
                .map(|p| percent_spark(p, t))
                .unwrap_or_else(|| format!("{}·", t.dim));
            let sec_spark = s
                .secondary
                .as_ref()
                .and_then(|w| w.used_percent)
                .map(|p| percent_spark(p, t))
                .unwrap_or_else(|| format!("{}·", t.dim));
            let rst = reset_spark(s.secondary.as_ref(), t);
            format!("{name_color}{short} {pri_spark}{sec_spark}{rst} {}│ ", t.dim)
        }
        ColorMode::Tmux => {
            format!(
                "{name_color}{name} {}{pri_label}:{pri} {}{sec_label}:{sec}{reset}{} │ ",
                t.dim, t.dim, t.dim
            )
        }
        ColorMode::Ansi => {
            let padded_name = format!("{name:7}");

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

            let mut out = format!(
                "{name_color}{padded_name} {}│ {}{pri_label} {pri}{pri_spark} {}│ {}{sec_label} {sec}{sec_spark}",
                t.dim, t.dim, t.dim, t.dim
            );

            if let Some(w) = s.secondary.as_ref() {
                if let Some(resets_at) = w.resets_at_unix {
                    out.push_str(&format!(
                        " {}│ ↻ {}",
                        t.dim,
                        format_time_remaining(resets_at)
                    ));
                }
            }
            out.push_str(t.reset);
            out
        }
    }
}

/// Render a failure line for a specific provider.
pub fn render_unavailable(name: &str) -> String {
    render_unavailable_with_mode(name, ColorMode::Tmux)
}

pub fn render_unavailable_with_mode(name: &str, mode: ColorMode) -> String {
    let t = theme(mode);
    match mode {
        ColorMode::TmuxCompact => format!("{}{} · {}│ ", t.dim, name, t.dim),
        ColorMode::Tmux => format!("{}{}  n/a {}│", t.dim, name, t.dim),
        ColorMode::Ansi => format!("{}{:7} │ n/a{}", t.dim, name, t.reset),
    }
}
