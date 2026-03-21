//! Campaign command handlers.

use mkt_core::error::Result;
use mkt_core::models::{
    CampaignFilters, CampaignId, CampaignStatus, CreateCampaignInput, UpdateCampaignInput,
};
use mkt_core::output::{OutputFormat, format_output};
use mkt_core::provider::MarketingProvider;

use crate::cli::CampaignAction;

/// Execute a campaign action against the given provider.
pub async fn execute(
    action: &CampaignAction,
    provider: &impl MarketingProvider,
    output_format: OutputFormat,
    dry_run: bool,
) -> Result<String> {
    match action {
        CampaignAction::List {
            status,
            name,
            limit,
        } => {
            let filters = CampaignFilters {
                status: status.as_deref().map(parse_status),
                name_contains: name.clone(),
                limit: *limit,
                cursor: None,
            };
            let result = provider.list_campaigns(&filters).await?;
            format_output(&result.data, output_format)
        }
        CampaignAction::Get { id } => {
            let campaign = provider
                .get_campaign(&CampaignId::from(id.as_str()))
                .await?;
            format_output(&[campaign], output_format)
        }
        CampaignAction::Create {
            name,
            objective,
            status,
            file: _,
        } => {
            if dry_run {
                return Ok(format!(
                    "[dry-run] Would create campaign: name={name}, objective={objective}"
                ));
            }
            let input = CreateCampaignInput {
                name: name.clone(),
                objective: objective.clone(),
                status: status.as_deref().map(parse_status),
                ..Default::default()
            };
            let campaign = provider.create_campaign(&input).await?;
            format_output(&[campaign], output_format)
        }
        CampaignAction::Update { id, name, status } => {
            if dry_run {
                return Ok(format!("[dry-run] Would update campaign {id}"));
            }
            let input = UpdateCampaignInput {
                name: name.clone(),
                status: status.as_deref().map(parse_status),
                ..Default::default()
            };
            let campaign = provider
                .update_campaign(&CampaignId::from(id.as_str()), &input)
                .await?;
            format_output(&[campaign], output_format)
        }
        CampaignAction::Delete { id } => {
            if dry_run {
                return Ok(format!("[dry-run] Would delete campaign {id}"));
            }
            provider
                .delete_campaign(&CampaignId::from(id.as_str()))
                .await?;
            Ok(format!("Campaign {id} deleted."))
        }
    }
}

fn parse_status(s: &str) -> CampaignStatus {
    match s.to_lowercase().as_str() {
        "active" => CampaignStatus::Active,
        "paused" => CampaignStatus::Paused,
        "archived" => CampaignStatus::Archived,
        "draft" => CampaignStatus::Draft,
        "deleted" => CampaignStatus::Deleted,
        other => CampaignStatus::Other(other.to_string()),
    }
}
