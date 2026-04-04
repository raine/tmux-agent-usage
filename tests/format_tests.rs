use agent_usage::format;
use agent_usage::model::{Credits, ProviderId, Snapshot, Window};

#[test]
fn render_full_codex_snapshot() {
    let s = Snapshot {
        provider: ProviderId::Codex,
        primary: Some(Window {
            used_percent: Some(72),
            window_minutes: Some(300),
            resets_at_unix: None,
        }),
        secondary: Some(Window {
            used_percent: Some(41),
            window_minutes: Some(10080),
            resets_at_unix: None,
        }),
        credits: Some(Credits {
            remaining: Some(18.20),
            is_unlimited: false,
        }),
        observed_at_unix: 0,
    };
    assert_eq!(format::render(Some(&s)), "Codex 5h:72% wk:41% cr:$18.20");
}

#[test]
fn render_partial_snapshot() {
    let s = Snapshot {
        provider: ProviderId::Codex,
        primary: Some(Window {
            used_percent: Some(28),
            window_minutes: Some(300),
            resets_at_unix: None,
        }),
        secondary: None,
        credits: None,
        observed_at_unix: 0,
    };
    assert_eq!(format::render(Some(&s)), "Codex 5h:28% sec:n/a");
}

#[test]
fn render_unlimited_credits() {
    let s = Snapshot {
        provider: ProviderId::Codex,
        primary: Some(Window {
            used_percent: Some(10),
            window_minutes: Some(300),
            resets_at_unix: None,
        }),
        secondary: Some(Window {
            used_percent: Some(5),
            window_minutes: Some(10080),
            resets_at_unix: None,
        }),
        credits: Some(Credits {
            remaining: None,
            is_unlimited: true,
        }),
        observed_at_unix: 0,
    };
    assert_eq!(format::render(Some(&s)), "Codex 5h:10% wk:5% cr:unlim");
}

#[test]
fn render_none_snapshot() {
    assert_eq!(format::render(None), "n/a");
}

#[test]
fn render_unavailable_uses_provider_name() {
    assert_eq!(format::render_unavailable("Claude"), "Claude n/a");
    assert_eq!(format::render_unavailable("Codex"), "Codex n/a");
}

#[test]
fn render_unknown_window_minutes_uses_fallback() {
    let s = Snapshot {
        provider: ProviderId::Codex,
        primary: Some(Window {
            used_percent: Some(50),
            window_minutes: Some(999),
            resets_at_unix: None,
        }),
        secondary: Some(Window {
            used_percent: Some(25),
            window_minutes: None,
            resets_at_unix: None,
        }),
        credits: None,
        observed_at_unix: 0,
    };
    assert_eq!(format::render(Some(&s)), "Codex pri:50% sec:25%");
}

#[test]
fn render_claude_provider() {
    let s = Snapshot {
        provider: ProviderId::Claude,
        primary: Some(Window {
            used_percent: Some(33),
            window_minutes: Some(300),
            resets_at_unix: None,
        }),
        secondary: None,
        credits: None,
        observed_at_unix: 0,
    };
    assert_eq!(format::render(Some(&s)), "Claude 5h:33% sec:n/a");
}
