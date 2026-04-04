use agent_usage::format::ColorMode;
use agent_usage::{cache, format, model, provider};
use std::env;

const TTL_SECS: u64 = 55;

fn main() {
    let args: Vec<String> = env::args().collect();

    let debug = args.iter().any(|a| a == "--debug-probe");
    let tmux = args.iter().any(|a| a == "--tmux");
    let mode = if tmux {
        ColorMode::Tmux
    } else {
        ColorMode::Ansi
    };

    // Resolve provider: CLI arg > env var > default (codex)
    let provider_arg = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .nth(1)
        .map(|s| s.as_str());
    let provider_id = provider_arg
        .and_then(provider::parse_provider_arg)
        .or_else(|| {
            env::var("AGENT_USAGE_PROVIDER")
                .ok()
                .as_deref()
                .and_then(provider::parse_provider_arg)
        })
        .unwrap_or(model::ProviderId::Codex);

    let p = match provider::by_id(provider_id) {
        Some(p) => p,
        None => {
            println!(
                "{}",
                format::render_unavailable_with_mode(provider_id.display_name(), mode)
            );
            return;
        }
    };

    if debug {
        match p.refresh() {
            Ok(s) => {
                println!("{}", serde_json::to_string_pretty(&s).unwrap());
            }
            Err(e) => {
                eprintln!("probe failed: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    let path = cache::path_for(provider_id);
    let lock = cache::lock_path(&path);
    let cached = cache::load(&path);

    // Fresh cache: print and exit fast
    if let Some(s) = cached.as_ref() {
        if cache::is_fresh(s, TTL_SECS) {
            println!("{}", format::render_with_mode(Some(s), mode));
            return;
        }
    }

    // Stale cache exists: print it now (no tmux lag), then try to refresh
    if let Some(s) = cached.as_ref() {
        println!("{}", format::render_with_mode(Some(s), mode));

        if let Some(_lock_file) = cache::try_lock(&lock) {
            if let Ok(snapshot) = p.refresh() {
                let _ = cache::save(&path, &snapshot);
            }
        }
        return;
    }

    // No cache at all: must probe synchronously (first run experience)
    match p.refresh() {
        Ok(snapshot) => {
            let _ = cache::save(&path, &snapshot);
            println!("{}", format::render_with_mode(Some(&snapshot), mode));
        }
        Err(_) => {
            println!(
                "{}",
                format::render_unavailable_with_mode(p.display_name(), mode)
            );
        }
    }
}
