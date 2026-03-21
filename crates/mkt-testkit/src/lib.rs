//! Shared test utilities, mocks, and fixtures for the `mkt` workspace.
//!
//! Provides [`mock_provider::MockProvider`], JSON fixtures, wiremock helpers,
//! and custom assertion helpers used across all crate test suites.
//!
//! # Modules
//!
//! - [`fixtures`] — Compile-time embedded JSON fixture data for Meta Graph API
//!   responses, with both raw string and parsed-value accessors.
//! - [`mock_provider`] — In-process configurable mock implementing
//!   [`mkt_core::provider::MarketingProvider`].
//! - [`http`] — [`wiremock`] response template factories for Meta API endpoints.
//! - [`assertions`] — Domain-aware assertion helpers with clear failure messages.
#![warn(missing_docs)]

pub mod assertions;
pub mod fixtures;
pub mod http;
pub mod mock_provider;
