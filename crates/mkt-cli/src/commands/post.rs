//! Post command handlers.

use mkt_core::error::Result;
use mkt_core::output::{OutputFormat, format_output};
use mkt_core::provider::MarketingProvider;

use crate::cli::PostAction;

/// Execute a post action.
pub async fn execute(
    action: &PostAction,
    provider: &impl MarketingProvider,
    output_format: OutputFormat,
    dry_run: bool,
) -> Result<String> {
    match action {
        PostAction::Create {
            platform,
            message,
            image_url,
            link,
        } => {
            if dry_run {
                return Ok(format!("[dry-run] Would create {platform} post"));
            }
            let input = mkt_core::models::PublishPostInput {
                platform: platform.clone(),
                message: message.clone(),
                link: link.clone(),
                image_url: image_url.clone(),
                ..Default::default()
            };
            let post = provider.publish_post(&input).await?;
            format_output(&[post], output_format)
        }
        PostAction::Promote { id } => {
            if dry_run {
                return Ok(format!("[dry-run] Would promote post {id}"));
            }
            Ok("Post promotion is not yet fully implemented.".into())
        }
    }
}
