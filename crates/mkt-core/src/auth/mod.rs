//! Authentication and token resolution.
//!
//! Handles resolving API tokens from environment variables and
//! configuration files, wrapping them in `secrecy::SecretString`
//! to prevent accidental logging.

mod token;

pub use token::*;
