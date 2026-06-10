//! JSON fixtures for testing API response parsing.
//!
//! Each sub-module corresponds to a marketing platform and exposes functions
//! that return [`serde_json::Value`] matching the real API response shape.
//! Fixtures are built with `serde_json::json!` so they are embedded at
//! compile time and require no filesystem access at runtime.

pub mod google;
pub mod linkedin;
pub mod meta;
