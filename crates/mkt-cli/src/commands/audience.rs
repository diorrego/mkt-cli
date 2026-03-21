//! Audience command handlers.

use mkt_core::error::Result;
use mkt_core::output::{OutputFormat, format_output};
use mkt_core::provider::MarketingProvider;

use crate::cli::AudienceAction;

/// Execute an audience action.
pub async fn execute(
    action: &AudienceAction,
    provider: &impl MarketingProvider,
    output_format: OutputFormat,
) -> Result<String> {
    match action {
        AudienceAction::List => {
            let audiences = provider.list_audiences().await?;
            format_output(&audiences, output_format)
        }
        AudienceAction::Create { name, description } => {
            let input = mkt_core::models::CreateAudienceInput {
                name: name.clone(),
                description: description.clone(),
                ..Default::default()
            };
            let audience = provider.create_audience(&input).await?;
            format_output(&[audience], output_format)
        }
    }
}
