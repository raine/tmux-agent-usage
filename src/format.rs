use crate::model::Snapshot;

fn window_label(minutes: Option<u16>, fallback: &str) -> &str {
    match minutes {
        Some(300) => "5h",
        Some(10080) => "wk",
        _ => fallback,
    }
}

pub fn render(snapshot: Option<&Snapshot>) -> String {
    let Some(s) = snapshot else {
        return "n/a".to_string();
    };

    let name = s.provider.display_name();

    let pri_label = window_label(s.primary.as_ref().and_then(|w| w.window_minutes), "pri");
    let sec_label = window_label(s.secondary.as_ref().and_then(|w| w.window_minutes), "sec");

    let pri = s
        .primary
        .as_ref()
        .and_then(|w| w.used_percent)
        .map(|v| format!("{v}%"))
        .unwrap_or_else(|| "n/a".to_string());
    let sec = s
        .secondary
        .as_ref()
        .and_then(|w| w.used_percent)
        .map(|v| format!("{v}%"))
        .unwrap_or_else(|| "n/a".to_string());

    let mut out = format!("{name} {pri_label}:{pri} {sec_label}:{sec}");

    if let Some(cr) = &s.credits {
        if cr.is_unlimited {
            out.push_str(" cr:unlim");
        } else if let Some(rem) = cr.remaining {
            out.push_str(&format!(" cr:${rem:.2}"));
        }
    }

    out
}

/// Render a failure line for a specific provider.
pub fn render_unavailable(name: &str) -> String {
    format!("{name} n/a")
}
