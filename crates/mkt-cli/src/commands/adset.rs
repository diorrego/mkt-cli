//! Ad set command handlers.

use mkt_core::error::{MktError, Result};
use mkt_core::models::{AdSetStatus, Budget, BudgetKind, CampaignId, CreateAdSetInput};
use mkt_core::output::{OutputFormat, format_output};
use mkt_core::provider::MarketingProvider;

use crate::cli::AdsetAction;

/// Execute an ad set action against the given provider.
pub async fn execute(
    action: &AdsetAction,
    provider: &impl MarketingProvider,
    output_format: OutputFormat,
    dry_run: bool,
) -> Result<String> {
    match action {
        AdsetAction::List { campaign } => {
            let result = provider
                .list_adsets(&CampaignId::from(campaign.as_str()))
                .await?;
            format_output(&result.data, output_format)
        }
        AdsetAction::Create {
            campaign,
            name,
            status,
            targeting,
            daily_budget,
            optimization_goal,
            billing_event,
        } => {
            if dry_run {
                return Ok(format!(
                    "[dry-run] Would create ad set: campaign={campaign}, name={name}"
                ));
            }
            let targeting_json = targeting
                .as_deref()
                .map(serde_json::from_str)
                .transpose()
                .map_err(|e: serde_json::Error| MktError::ValidationError {
                    field: "targeting".into(),
                    message: format!("invalid JSON: {e}"),
                })?;

            let mut extra = serde_json::Map::new();
            if let Some(goal) = optimization_goal {
                extra.insert(
                    "optimization_goal".into(),
                    serde_json::Value::String(goal.clone()),
                );
            }
            if let Some(event) = billing_event {
                extra.insert(
                    "billing_event".into(),
                    serde_json::Value::String(event.clone()),
                );
            }

            let input = CreateAdSetInput {
                campaign_id: CampaignId::from(campaign.as_str()),
                name: name.clone(),
                status: status.as_deref().map(parse_status),
                targeting: targeting_json,
                budget: daily_budget.map(|amount| Budget {
                    amount,
                    currency: "USD".into(),
                    kind: BudgetKind::Daily,
                }),
                extra: if extra.is_empty() {
                    None
                } else {
                    Some(serde_json::Value::Object(extra))
                },
            };
            let adset = provider.create_adset(&input).await?;
            format_output(&[adset], output_format)
        }
    }
}

fn parse_status(s: &str) -> AdSetStatus {
    match s.to_lowercase().as_str() {
        "active" => AdSetStatus::Active,
        "paused" => AdSetStatus::Paused,
        "archived" => AdSetStatus::Archived,
        "deleted" => AdSetStatus::Deleted,
        other => AdSetStatus::Other(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_status_known_values() {
        assert_eq!(parse_status("active"), AdSetStatus::Active);
        assert_eq!(parse_status("PAUSED"), AdSetStatus::Paused);
        assert_eq!(parse_status("Archived"), AdSetStatus::Archived);
        assert_eq!(parse_status("deleted"), AdSetStatus::Deleted);
    }

    #[test]
    fn parse_status_unknown_maps_to_other() {
        assert_eq!(
            parse_status("pending_review"),
            AdSetStatus::Other("pending_review".into())
        );
    }
}
