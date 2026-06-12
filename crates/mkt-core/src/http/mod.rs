//! Shared HTTP client and rate limiting utilities.
//!
//! Provides a configured `reqwest::Client` builder and a token-bucket
//! rate limiter used by all provider crates.

mod client;
mod rate_limit;
mod retry;

pub use client::*;
pub use rate_limit::*;
pub use retry::*;
