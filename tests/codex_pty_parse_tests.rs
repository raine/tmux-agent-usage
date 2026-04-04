use agent_usage::provider::codex::pty::parse_percent;

#[test]
fn parse_simple_percent() {
    assert_eq!(parse_percent("  80% left"), Some(80));
}

#[test]
fn parse_percent_at_start() {
    assert_eq!(parse_percent("100% used"), Some(100));
}

#[test]
fn parse_percent_zero() {
    assert_eq!(parse_percent("0% remaining"), Some(0));
}

#[test]
fn parse_no_percent_returns_none() {
    assert_eq!(parse_percent("no percentage here"), None);
}

#[test]
fn parse_percent_with_ansi_stripped() {
    // After ANSI stripping, this is just "  42% left"
    assert_eq!(parse_percent("  42% left"), Some(42));
}

#[test]
fn parse_empty_string() {
    assert_eq!(parse_percent(""), None);
}

#[test]
fn parse_percent_embedded_in_text() {
    assert_eq!(parse_percent("5h limit: 65% left"), Some(65));
}
