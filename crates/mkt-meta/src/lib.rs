//! Meta (Facebook/Instagram) provider for the `mkt` marketing CLI.
//!
//! Implements the [`MarketingProvider`](mkt_core::provider::MarketingProvider)
//! trait from `mkt-core` using the Meta Graph API and Marketing API.
#![warn(missing_docs)]

mod client;
mod error;
mod mapping;
mod provider;

pub use client::MetaClient;
pub use provider::MetaProvider;
