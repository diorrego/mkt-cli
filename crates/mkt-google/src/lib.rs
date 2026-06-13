//! Google Ads provider for the `mkt` marketing CLI.
//!
//! Talks to the Google Ads API REST interface (v24): GAQL searches via
//! `googleAds:search`, updates/removes via the per-resource `:mutate`
//! endpoints, and campaign creation via the atomic `googleAds:mutate`
//! endpoint (budget + campaign in one transactional request).

#![warn(missing_docs)]

mod auth;
mod client;
mod error;
pub mod mapping;
mod provider;

pub use auth::{GOOGLE_TOKEN_URL, fetch_access_token};
pub use client::GoogleClient;
pub use provider::GoogleProvider;
