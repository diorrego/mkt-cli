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
        PostAction::Promote { id, adset, name } => {
            if dry_run {
                return Ok(format!(
                    "[dry-run] Would promote post {id} in ad set {adset} (ad created paused)"
                ));
            }
            let input = mkt_core::models::PromotePostInput {
                adset_id: adset.clone(),
                name: name.clone(),
                extra: None,
            };
            let ad = provider
                .promote_post(&mkt_core::models::PostId::from(id.as_str()), &input)
                .await?;
            format_output(&[ad], output_format)
        }
    }
}
