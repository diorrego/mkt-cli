//! XDG-compliant config path resolution.

use std::path::PathBuf;

use crate::error::{MktError, Result};

/// Returns the mkt config directory.
///
/// Resolution order:
/// 1. `MKT_CONFIG_DIR` environment variable
/// 2. `$XDG_CONFIG_HOME/mkt` (Linux/macOS)
/// 3. `%APPDATA%\mkt` (Windows)
pub fn config_dir() -> Result<PathBuf> {
    if let Ok(dir) = std::env::var("MKT_CONFIG_DIR") {
        return Ok(PathBuf::from(dir));
    }

    dirs::config_dir()
        .map(|d| d.join("mkt"))
        .ok_or_else(|| MktError::ConfigError("Could not determine config directory".into()))
}

/// Returns the path to `config.toml`.
pub fn config_file() -> Result<PathBuf> {
    config_dir().map(|d| d.join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_dir_falls_back_to_xdg() {
        // When MKT_CONFIG_DIR is not set, should use dirs::config_dir()
        // This test only works if MKT_CONFIG_DIR is not already set
        if std::env::var("MKT_CONFIG_DIR").is_err() {
            let dir = config_dir().expect("should resolve from XDG");
            assert!(dir.to_string_lossy().contains("mkt"));
        }
    }

    #[test]
    fn config_file_is_inside_config_dir() {
        if std::env::var("MKT_CONFIG_DIR").is_err() {
            let dir = config_dir().expect("dir");
            let file = config_file().expect("file");
            assert_eq!(file, dir.join("config.toml"));
        }
    }
}
