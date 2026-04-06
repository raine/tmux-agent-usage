use agent_usage::format::ColorMode;
use agent_usage::{cache, format, model, provider};
use std::env;


fn main() {
    let args: Vec<String> = env::args().collect();

    let debug = args.iter().any(|a| a == "--debug-probe");
    let tmux = args.iter().any(|a| a == "--tmux");
    let compact = args.iter().any(|a| a == "--compact");
    let mode = if compact {
        ColorMode::TmuxCompact
    } else if tmux {
        ColorMode::Tmux
    } else {
        ColorMode::Ansi
    };

    // Resolve provider: CLI arg > env var > None (show all)
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
        });

    match provider_id {
        Some(id) => run_single(id, mode, debug),
        None => {
            if debug {
                // Debug all providers
                for p in provider::registry() {
                    match p.refresh() {
                        Ok(s) => println!("{}", serde_json::to_string_pretty(&s).unwrap()),
                        Err(e) => eprintln!("{}: probe failed: {e}", p.display_name()),
                    }
                }
            } else {
                // Show all providers
                for p in provider::registry() {
                    let output = get_output(p.as_ref(), mode);
                    println!("{output}");
                }
            }
        }
    }
}

fn run_single(provider_id: model::ProviderId, mode: ColorMode, debug: bool) {
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
            Ok(s) => println!("{}", serde_json::to_string_pretty(&s).unwrap()),
            Err(e) => {
                eprintln!("probe failed: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    println!("{}", get_output(p.as_ref(), mode));
}

fn get_output(p: &dyn provider::Provider, mode: ColorMode) -> String {
    let path = cache::path_for(p.id());
    let lock = cache::lock_path(&path);
    let cached = cache::load(&path);

    // Fresh cache: return immediately
    if let Some(s) = cached.as_ref() {
        if cache::is_fresh(s, p.ttl_secs()) {
            return format::render_with_mode(Some(s), mode);
        }
    }

    // Stale cache: use it, try refresh
    if let Some(s) = cached.as_ref() {
        let output = format::render_with_mode(Some(s), mode);
        if let Some(_lock_file) = cache::try_lock(&lock) {
            match p.refresh() {
                Ok(snapshot) => {
                    let _ = cache::save(&path, &snapshot);
                }
                Err(_) => {
                    // Touch timestamp so we don't retry every call
                    let mut touched = s.clone();
                    touched.observed_at_unix = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64;
                    let _ = cache::save(&path, &touched);
                }
            }
        }
        return output;
    }

    // No cache: probe synchronously
    match p.refresh() {
        Ok(snapshot) => {
            let _ = cache::save(&path, &snapshot);
            format::render_with_mode(Some(&snapshot), mode)
        }
        Err(_) => format::render_unavailable_with_mode(p.display_name(), mode),
    }
}
