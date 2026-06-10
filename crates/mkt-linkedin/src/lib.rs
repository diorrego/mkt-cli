//! LinkedIn Marketing provider for the `mkt` marketing CLI.
//!
//! Talks to the versioned LinkedIn REST API (`Linkedin-Version: 202605`)
//! using Rest.li 2.0 conventions: finder queries, `PARTIAL_UPDATE`
//! patches, string-typed money amounts, and URN references.

#![warn(missing_docs)]

mod client;
mod error;
pub mod mapping;
mod provider;

pub use client::{LinkedInClient, WriteResponse};
pub use provider::LinkedInProvider;
