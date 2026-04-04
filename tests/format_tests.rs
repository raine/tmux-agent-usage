use agent_usage::format;
use agent_usage::model::{Credits, ProviderId, Snapshot, Window};

const DIM: &str = "#[fg=colour245]";
const GREEN: &str = "#[fg=colour114]";
const YELLOW: &str = "#[fg=colour221]";
const RED: &str = "#[fg=colour203]";

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
    assert_eq!(
        format::render(Some(&s)),
        format!("{DIM}Codex 5h:{YELLOW}72% {DIM}wk:{GREEN}41% {DIM}│")
    );
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
    assert_eq!(
        format::render(Some(&s)),
        format!("{DIM}Codex 5h:{GREEN}28% {DIM}sec:{DIM}n/a {DIM}│")
    );
}

#[test]
fn render_high_usage_is_red() {
    let s = Snapshot {
        provider: ProviderId::Codex,
        primary: Some(Window {
            used_percent: Some(95),
            window_minutes: Some(300),
            resets_at_unix: None,
        }),
        secondary: Some(Window {
            used_percent: Some(80),
            window_minutes: Some(10080),
            resets_at_unix: None,
        }),
        credits: None,
        observed_at_unix: 0,
    };
    assert_eq!(
        format::render(Some(&s)),
        format!("{DIM}Codex 5h:{RED}95% {DIM}wk:{RED}80% {DIM}│")
    );
}

#[test]
fn render_none_snapshot() {
    assert_eq!(format::render(None), format!("{DIM}n/a"));
}

#[test]
fn render_unavailable_uses_provider_name() {
    assert_eq!(
        format::render_unavailable("Claude"),
        format!("{DIM}Claude n/a {DIM}│")
    );
    assert_eq!(
        format::render_unavailable("Codex"),
        format!("{DIM}Codex n/a {DIM}│")
    );
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
    assert_eq!(
        format::render(Some(&s)),
        format!("{DIM}Codex pri:{YELLOW}50% {DIM}sec:{GREEN}25% {DIM}│")
    );
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
    assert_eq!(
        format::render(Some(&s)),
        format!("{DIM}Claude 5h:{GREEN}33% {DIM}sec:{DIM}n/a {DIM}│")
    );
}
