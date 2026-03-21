//! Configuration loading and profile management.
//!
//! Reads `config.toml` from the XDG config directory and supports
//! environment variable overrides for sensitive values like API tokens.

mod paths;
mod profile;

pub use paths::*;
pub use profile::*;
