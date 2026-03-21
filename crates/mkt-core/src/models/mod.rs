//! Domain models for the `mkt` marketing CLI.
//!
//! All models are provider-agnostic. Each provider crate contains a
//! `mapping` module that converts between API-specific representations
//! and these unified models.

mod ad;
mod adset;
mod audience;
mod campaign;
mod common;
mod creative;
mod insight;
mod media;
mod post;

pub use ad::*;
pub use adset::*;
pub use audience::*;
pub use campaign::*;
pub use common::*;
pub use creative::*;
pub use insight::*;
pub use media::*;
pub use post::*;
