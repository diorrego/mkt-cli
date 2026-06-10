//! TikTok for Business provider for the `mkt` marketing CLI.
//!
//! Talks to the Business API v1.3: `Access-Token` header auth, the
//! `{"code","message","request_id","data"}` envelope (HTTP 200 even on
//! errors), campaign endpoints, integrated reporting, and DMP audiences.

#![warn(missing_docs)]

mod client;
mod error;
pub mod mapping;
mod provider;

pub use client::TikTokClient;
pub use provider::TikTokProvider;
