//! Media upload command handlers.

use mkt_core::error::Result;
use mkt_core::output::{OutputFormat, format_output};
use mkt_core::provider::MarketingProvider;

use crate::cli::MediaAction;

/// Execute a media action.
pub async fn execute(
    action: &MediaAction,
    provider: &impl MarketingProvider,
    output_format: OutputFormat,
    dry_run: bool,
) -> Result<String> {
    match action {
        MediaAction::UploadImage { file: _, url } => {
            if dry_run {
                return Ok("[dry-run] Would upload image".into());
            }
            let input = mkt_core::models::UploadImageInput {
                url: url.clone(),
                ..Default::default()
            };
            let asset = provider.upload_image(&input).await?;
            format_output(&[asset], output_format)
        }
        MediaAction::UploadVideo {
            file: _,
            url,
            title,
        } => {
            if dry_run {
                return Ok("[dry-run] Would upload video".into());
            }
            let input = mkt_core::models::UploadVideoInput {
                url: url.clone(),
                title: title.clone(),
                ..Default::default()
            };
            let asset = provider.upload_video(&input).await?;
            format_output(&[asset], output_format)
        }
    }
}
