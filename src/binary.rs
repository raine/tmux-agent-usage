use std::{env, path::PathBuf, process::Command};

/// Resolve a binary by name, checking an optional env override first.
pub fn resolve(name: &str, env_override: Option<&str>) -> Option<String> {
    // Explicit override via env var
    if let Some(var) = env_override {
        if let Ok(bin) = env::var(var) {
            if is_executable(&bin) {
                return Some(bin);
            }
        }
    }

    // Try bare name first (works if PATH is correct)
    if which(name).is_some() {
        return Some(name.to_string());
    }

    // Probe login shell PATH for global installs
    if let Some(path) = login_shell_which(name) {
        return Some(path);
    }

    None
}

fn which(name: &str) -> Option<PathBuf> {
    Command::new("which")
        .arg(name)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            let s = String::from_utf8(o.stdout).ok()?;
            Some(PathBuf::from(s.trim()))
        })
}

fn login_shell_which(name: &str) -> Option<String> {
    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    Command::new(&shell)
        .args(["-lc", &format!("which {name}")])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            let s = String::from_utf8(o.stdout).ok()?;
            let trimmed = s.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
}

fn is_executable(path: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}
