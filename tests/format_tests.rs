use agent_usage::format;
use agent_usage::model::{Credits, ProviderId, ScopedWindow, Snapshot, Window};

const DIM: &str = "#[fg=colour245]";
const DIVIDER: &str = "#[fg=colour250]";
const CLAUDE_ORANGE: &str = "#[fg=#d97757]";
const GREEN: &str = "#[fg=colour114]";
const YELLOW: &str = "#[fg=colour221]";
const RED: &str = "#[fg=colour203]";
const ANSI_DIM: &str = "\x1b[38;5;245m";
const ANSI_CLAUDE_ORANGE: &str = "\x1b[38;2;217;119;87m";
const ANSI_GREEN: &str = "\x1b[38;5;114m";
const ANSI_YELLOW: &str = "\x1b[38;5;221m";
const ANSI_RESET: &str = "\x1b[0m";

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
        scoped: Vec::new(),
        observed_at_unix: 0,
    };
    // No resets_at_unix → no braille indicator
    assert_eq!(
        format::render(Some(&s)),
        format!("{DIM}Codex {DIM}5h:{YELLOW}72% {DIM}wk:{GREEN}41%")
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
        scoped: Vec::new(),
        observed_at_unix: 0,
    };
    assert_eq!(
        format::render(Some(&s)),
        format!("{DIM}Codex {DIM}5h:{GREEN}28% {DIM}sec:{DIM}n/a")
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
        scoped: Vec::new(),
        observed_at_unix: 0,
    };
    assert_eq!(
        format::render(Some(&s)),
        format!("{DIM}Codex {DIM}5h:{RED}95% {DIM}wk:{RED}80%")
    );
}

#[test]
fn render_grok_weekly_snapshot() {
    let s = Snapshot {
        provider: ProviderId::Grok,
        primary: None,
        secondary: Some(Window {
            used_percent: Some(1),
            window_minutes: Some(10080),
            resets_at_unix: None,
        }),
        credits: None,
        scoped: Vec::new(),
        observed_at_unix: 0,
    };
    assert_eq!(
        format::render(Some(&s)),
        format!("{DIM}Grok {DIM}wk:{GREEN}1%")
    );
}

#[test]
fn render_grok_compact_snapshot() {
    let s = Snapshot {
        provider: ProviderId::Grok,
        primary: None,
        secondary: Some(Window {
            used_percent: Some(1),
            window_minutes: Some(10080),
            resets_at_unix: None,
        }),
        credits: None,
        scoped: Vec::new(),
        observed_at_unix: 0,
    };
    assert_eq!(
        format::render_with_mode(Some(&s), format::ColorMode::TmuxCompact),
        format!("{DIM}G {GREEN}▁{DIM}▁")
    );
}

#[test]
fn render_unavailable_uses_provider_name() {
    assert_eq!(
        format::render_unavailable("Claude"),
        format!("{DIM}Claude  n/a")
    );
    assert_eq!(
        format::render_unavailable("Codex"),
        format!("{DIM}Codex  n/a")
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
        scoped: Vec::new(),
        observed_at_unix: 0,
    };
    assert_eq!(
        format::render(Some(&s)),
        format!("{DIM}Codex {DIM}pri:{YELLOW}50% {DIM}sec:{GREEN}25%")
    );
}

#[test]
fn render_claude_provider_has_orange_name() {
    let s = Snapshot {
        provider: ProviderId::Claude,
        primary: Some(Window {
            used_percent: Some(33),
            window_minutes: Some(300),
            resets_at_unix: None,
        }),
        secondary: None,
        credits: None,
        scoped: Vec::new(),
        observed_at_unix: 0,
    };
    assert_eq!(
        format::render(Some(&s)),
        format!("{CLAUDE_ORANGE}Claude {DIM}5h:{GREEN}33% {DIM}sec:{DIM}n/a")
    );
}

#[test]
fn render_shows_reset_indicator_for_both_windows() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let s = Snapshot {
        provider: ProviderId::Codex,
        primary: Some(Window {
            used_percent: Some(10),
            window_minutes: Some(300),
            resets_at_unix: Some(now + 150 * 60),
        }),
        secondary: Some(Window {
            used_percent: Some(50),
            window_minutes: Some(10080),
            resets_at_unix: Some(now + 3 * 86400 + 43200),
        }),
        credits: None,
        scoped: Vec::new(),
        observed_at_unix: now,
    };
    let rendered = format::render(Some(&s));
    assert_eq!(
        rendered,
        format!("{DIM}Codex {DIM}5h:{GREEN}10% {DIM}⡇ {DIM}wk:{YELLOW}50% {DIM}⡇")
    );
}

#[test]
fn render_compact_places_weekly_primary_in_weekly_slot() {
    let s = Snapshot {
        provider: ProviderId::Codex,
        primary: Some(Window {
            used_percent: Some(51),
            window_minutes: Some(10080),
            resets_at_unix: None,
        }),
        secondary: Some(Window {
            used_percent: None,
            window_minutes: None,
            resets_at_unix: None,
        }),
        credits: None,
        scoped: Vec::new(),
        observed_at_unix: 0,
    };
    assert_eq!(
        format::render_with_mode(Some(&s), format::ColorMode::TmuxCompact),
        format!("{DIM}O {DIM}·{DIM}▁{YELLOW}▄{DIM}▁")
    );
}

#[test]
fn render_compact_shows_reset_spark_for_both_windows() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let s = Snapshot {
        provider: ProviderId::Codex,
        primary: Some(Window {
            used_percent: Some(10),
            window_minutes: Some(300),
            resets_at_unix: Some(now + 150 * 60),
        }),
        secondary: Some(Window {
            used_percent: Some(50),
            window_minutes: Some(10080),
            resets_at_unix: Some(now + 3 * 86400 + 43200),
        }),
        credits: None,
        scoped: Vec::new(),
        observed_at_unix: now,
    };
    let rendered = format::render_with_mode(Some(&s), format::ColorMode::TmuxCompact);
    assert_eq!(rendered, format!("{DIM}O {GREEN}▁{DIM}▅{YELLOW}▄{DIM}▅"));
}

#[test]
fn render_scoped_window_after_plan_windows() {
    let s = Snapshot {
        provider: ProviderId::Claude,
        primary: Some(Window {
            used_percent: Some(26),
            window_minutes: Some(300),
            resets_at_unix: None,
        }),
        secondary: Some(Window {
            used_percent: Some(25),
            window_minutes: Some(10080),
            resets_at_unix: None,
        }),
        credits: None,
        scoped: vec![ScopedWindow {
            label: "Fable".to_string(),
            window: Window {
                used_percent: Some(27),
                window_minutes: Some(10080),
                resets_at_unix: None,
            },
        }],
        observed_at_unix: 0,
    };
    assert_eq!(
        format::render(Some(&s)),
        format!(
            "{CLAUDE_ORANGE}Claude {DIM}5h:{GREEN}26% {DIM}wk:{GREEN}25% {DIVIDER}│ {DIM}Fable:{GREEN}27%"
        )
    );
}

#[test]
fn render_multiple_scoped_windows_in_order() {
    let s = Snapshot {
        provider: ProviderId::Claude,
        primary: None,
        secondary: Some(Window {
            used_percent: Some(10),
            window_minutes: Some(10080),
            resets_at_unix: None,
        }),
        credits: None,
        scoped: vec![
            ScopedWindow {
                label: "Fable".to_string(),
                window: Window {
                    used_percent: Some(55),
                    window_minutes: Some(10080),
                    resets_at_unix: None,
                },
            },
            ScopedWindow {
                label: "Opus".to_string(),
                window: Window {
                    used_percent: Some(90),
                    window_minutes: Some(10080),
                    resets_at_unix: None,
                },
            },
        ],
        observed_at_unix: 0,
    };
    assert_eq!(
        format::render(Some(&s)),
        format!(
            "{CLAUDE_ORANGE}Claude {DIM}wk:{GREEN}10% {DIVIDER}│ {DIM}Fable:{YELLOW}55% {DIM}Opus:{RED}90%"
        )
    );
}

#[test]
fn render_scoped_window_in_ansi_with_only_secondary_plan_window() {
    let s = Snapshot {
        provider: ProviderId::Claude,
        primary: None,
        secondary: Some(Window {
            used_percent: Some(10),
            window_minutes: Some(10080),
            resets_at_unix: None,
        }),
        credits: None,
        scoped: vec![ScopedWindow {
            label: "Fable".to_string(),
            window: Window {
                used_percent: Some(55),
                window_minutes: Some(10080),
                resets_at_unix: None,
            },
        }],
        observed_at_unix: 0,
    };
    assert_eq!(
        format::render_with_mode(Some(&s), format::ColorMode::Ansi),
        format!(
            "{ANSI_CLAUDE_ORANGE}Claude  {ANSI_DIM}│ {ANSI_DIM}week {ANSI_GREEN} 10% {ANSI_GREEN}▁          {ANSI_DIM}│ {ANSI_DIM}Fable {ANSI_YELLOW} 55% {ANSI_YELLOW}▄         {ANSI_RESET}"
        )
    );
}

#[test]
fn render_scoped_window_in_compact_mode() {
    let s = Snapshot {
        provider: ProviderId::Claude,
        primary: None,
        secondary: Some(Window {
            used_percent: Some(10),
            window_minutes: Some(10080),
            resets_at_unix: None,
        }),
        credits: None,
        scoped: vec![ScopedWindow {
            label: "Fable".to_string(),
            window: Window {
                used_percent: Some(55),
                window_minutes: Some(10080),
                resets_at_unix: None,
            },
        }],
        observed_at_unix: 0,
    };
    assert_eq!(
        format::render_with_mode(Some(&s), format::ColorMode::TmuxCompact),
        format!("{CLAUDE_ORANGE}C {DIM}·{DIM}▁{GREEN}▁{DIM}▁{YELLOW}▄{DIM}▁")
    );
}

#[test]
fn render_without_scoped_windows_is_unchanged() {
    let s = Snapshot {
        provider: ProviderId::Claude,
        primary: Some(Window {
            used_percent: Some(26),
            window_minutes: Some(300),
            resets_at_unix: None,
        }),
        secondary: Some(Window {
            used_percent: Some(25),
            window_minutes: Some(10080),
            resets_at_unix: None,
        }),
        credits: None,
        scoped: Vec::new(),
        observed_at_unix: 0,
    };
    assert_eq!(
        format::render(Some(&s)),
        format!("{CLAUDE_ORANGE}Claude {DIM}5h:{GREEN}26% {DIM}wk:{GREEN}25%")
    );
}
