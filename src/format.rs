use crate::model::{ProviderId, Snapshot, Window};
use std::time::{SystemTime, UNIX_EPOCH};

const DIM: &str = "#[fg=colour245]";
const CLAUDE_ORANGE: &str = "#[fg=#d97757]";
const GREEN: &str = "#[fg=colour114]";
const YELLOW: &str = "#[fg=colour221]";
const RED: &str = "#[fg=colour203]";

// Braille characters: 8 levels from empty to full (bottom-up fill)
const BRAILLE_LEVELS: &[char] = &['⠀', '⡀', '⡄', '⡆', '⡇', '⣇', '⣧', '⣷', '⣿'];

fn percent_color(pct: u8) -> &'static str {
    match pct {
        0..=49 => GREEN,
        50..=79 => YELLOW,
        _ => RED,
    }
}

fn window_label(minutes: Option<u16>, fallback: &str) -> &str {
    match minutes {
        Some(300) => "5h",
        Some(10080) => "wk",
        _ => fallback,
    }
}

fn render_percent(pct: Option<u8>) -> String {
    match pct {
        Some(v) => format!("{}{v}%", percent_color(v)),
        None => format!("{DIM}n/a"),
    }
}

/// Braille indicator showing time remaining until window reset.
/// Fuller = more time left, emptier = resetting soon.
fn reset_indicator(window: Option<&Window>) -> String {
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

    format!(" {DIM}{ch}")
}

pub fn render(snapshot: Option<&Snapshot>) -> String {
    let Some(s) = snapshot else {
        return format!("{DIM}n/a");
    };

    let name = s.provider.display_name();
    let name_color = match s.provider {
        ProviderId::Claude => CLAUDE_ORANGE,
        _ => DIM,
    };

    let pri_label = window_label(s.primary.as_ref().and_then(|w| w.window_minutes), "pri");
    let sec_label = window_label(s.secondary.as_ref().and_then(|w| w.window_minutes), "sec");

    let pri = render_percent(s.primary.as_ref().and_then(|w| w.used_percent));
    let sec = render_percent(s.secondary.as_ref().and_then(|w| w.used_percent));
    let reset = reset_indicator(s.secondary.as_ref());

    format!("{name_color}{name} {DIM}{pri_label}:{pri} {DIM}{sec_label}:{sec}{reset}{DIM} │ ")
}

/// Render a failure line for a specific provider.
pub fn render_unavailable(name: &str) -> String {
    format!("{DIM}{name} n/a {DIM}│")
}
