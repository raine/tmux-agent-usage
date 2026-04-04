use crate::model::Snapshot;

const DIM: &str = "#[fg=colour245]";
const GREEN: &str = "#[fg=colour114]";
const YELLOW: &str = "#[fg=colour221]";
const RED: &str = "#[fg=colour203]";

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

pub fn render(snapshot: Option<&Snapshot>) -> String {
    let Some(s) = snapshot else {
        return format!("{DIM}n/a");
    };

    let name = s.provider.display_name();

    let pri_label = window_label(s.primary.as_ref().and_then(|w| w.window_minutes), "pri");
    let sec_label = window_label(s.secondary.as_ref().and_then(|w| w.window_minutes), "sec");

    let pri = render_percent(s.primary.as_ref().and_then(|w| w.used_percent));
    let sec = render_percent(s.secondary.as_ref().and_then(|w| w.used_percent));

    format!("{DIM}{name} {pri_label}:{pri} {DIM}{sec_label}:{sec} {DIM}│")
}

/// Render a failure line for a specific provider.
pub fn render_unavailable(name: &str) -> String {
    format!("{DIM}{name} n/a {DIM}│")
}
