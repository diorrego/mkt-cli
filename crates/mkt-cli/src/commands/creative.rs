//! Creative command handlers.

use mkt_core::error::Result;
use mkt_core::output::{OutputFormat, format_output};
use mkt_core::provider::MarketingProvider;

use crate::cli::CreativeAction;

/// Execute a creative action.
pub async fn execute(
    action: &CreativeAction,
    provider: &impl MarketingProvider,
    output_format: OutputFormat,
    dry_run: bool,
) -> Result<String> {
    match action {
        CreativeAction::Create {
            name,
            body,
            image_url,
            link_url,
        } => {
            if dry_run {
                return Ok(format!("[dry-run] Would create creative: {name}"));
            }
            let input = mkt_core::models::CreateCreativeInput {
                name: name.clone(),
                body: body.clone(),
                image_url: image_url.clone(),
                link_url: link_url.clone(),
                ..Default::default()
            };
            let creative = provider.create_creative(&input).await?;
            format_output(&[creative], output_format)
        }
        CreativeAction::DarkPost {
            message,
            link,
            image_url,
        } => {
            if dry_run {
                return Ok("[dry-run] Would create dark post".into());
            }
            let input = mkt_core::models::CreateDarkPostInput {
                page_id: String::new(),
                message: message.clone(),
                link: link.clone(),
                image_url: image_url.clone(),
                ..Default::default()
            };
            let creative = provider.create_dark_post(&input).await?;
            format_output(&[creative], output_format)
        }
    }
}
