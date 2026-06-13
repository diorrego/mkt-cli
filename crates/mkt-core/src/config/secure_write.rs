//! Secure config-file writing and permission checks.
//!
//! The config file can hold access tokens, so it must be owner-only from
//! the first byte: creating it and then chmod-ing leaves a window where
//! the umask decides who can read it.

use std::path::Path;

use crate::error::Result;

/// Write `content` to `path`, creating the file owner-only (0600).
///
/// On Unix the mode applies from the first byte and pre-existing files
/// are tightened to 0600 as well; on other platforms this is a plain
/// write (Windows relies on ACLs).
///
/// # Errors
///
/// Returns an error if the file cannot be created or written.
pub fn write_private(path: &Path, content: &str) -> Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write as _;
        use std::os::unix::fs::OpenOptionsExt as _;
        use std::os::unix::fs::PermissionsExt as _;

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(content.as_bytes())?;
        // `mode` only applies on creation; an existing file keeps its old
        // permissions, so tighten those too.
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// A warning message when `path` is readable or writable by group/other
/// (the file may contain tokens). `None` when permissions are fine, the
/// file does not exist, or the platform has no Unix modes.
#[must_use]
pub fn permissions_warning(path: &Path) -> Option<String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(path).ok()?.permissions().mode();
        if mode & 0o077 != 0 {
            return Some(format!(
                "{} is accessible by other users (mode {:03o}); run: chmod 600 {}",
                path.display(),
                mode & 0o777,
                path.display()
            ));
        }
        None
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        None
    }
}

#[cfg(all(test, unix))]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use std::os::unix::fs::PermissionsExt as _;

    use super::*;

    fn temp_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("mkt-secure-write-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn new_file_is_created_owner_only() {
        let path = temp_path();
        write_private(&path, "secret = true").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        std::fs::remove_file(&path).unwrap();
        assert_eq!(mode, 0o600, "new config files must be 0600, got {mode:03o}");
    }

    #[test]
    fn existing_loose_file_is_tightened() {
        let path = temp_path();
        std::fs::write(&path, "old").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        write_private(&path, "new").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        let content = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(mode, 0o600);
        assert_eq!(content, "new");
    }

    #[test]
    fn warning_fires_only_for_loose_permissions() {
        let path = temp_path();
        write_private(&path, "x").unwrap();
        assert!(permissions_warning(&path).is_none());

        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        let warning = permissions_warning(&path);
        std::fs::remove_file(&path).unwrap();
        let warning = warning.expect("0644 must warn");
        assert!(warning.contains("chmod 600"), "{warning}");
        assert!(warning.contains("644"), "{warning}");
    }

    #[test]
    fn missing_file_does_not_warn() {
        assert!(permissions_warning(Path::new("/nonexistent/mkt-test")).is_none());
    }
}
