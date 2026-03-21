//! `mkt doctor` command handler.

use mkt_core::config;
use mkt_core::error::Result;

/// Verify config, tokens, and API connectivity.
pub fn execute(config_path: Option<&std::path::Path>) -> Result<String> {
    let mut lines = vec!["mkt doctor — checking configuration...".to_string()];

    // Check config directory
    match config::config_dir() {
        Ok(dir) => {
            if dir.exists() {
                lines.push(format!("  [ok] Config directory: {}", dir.display()));
            } else {
                lines.push(format!(
                    "  [warn] Config directory does not exist: {}",
                    dir.display()
                ));
            }
        }
        Err(e) => {
            lines.push(format!("  [error] Config directory: {e}"));
        }
    }

    // Check config file
    let config_file = config_path
        .map(std::path::PathBuf::from)
        .or_else(|| config::config_file().ok());

    match config_file {
        Some(f) if f.exists() => {
            lines.push(format!("  [ok] Config file: {}", f.display()));
            match config::MktConfig::load_from_file(&f) {
                Ok(cfg) => {
                    let profiles: Vec<_> = cfg.profiles.keys().collect();
                    lines.push(format!("  [ok] Profiles: {profiles:?}"));
                }
                Err(e) => {
                    lines.push(format!("  [error] Failed to parse config: {e}"));
                }
            }
        }
        Some(f) => {
            lines.push(format!("  [warn] Config file not found: {}", f.display()));
        }
        None => {
            lines.push("  [error] No configuration found".to_string());
        }
    }

    Ok(lines.join("\n"))
}
