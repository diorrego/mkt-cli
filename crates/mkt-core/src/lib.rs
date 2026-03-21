//! Core library for the `mkt` multi-platform marketing CLI.
//!
//! This crate provides the shared traits, domain models, configuration,
//! output formatting, HTTP client, and error types used across all
//! provider crates.
#![warn(missing_docs)]

pub mod auth;
pub mod config;
pub mod error;
pub mod http;
pub mod models;
pub mod output;
pub mod provider;
