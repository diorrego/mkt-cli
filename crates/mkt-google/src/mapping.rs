//! Conversion between Google Ads API REST JSON and domain models.
//!
//! All functions here are pure transformations with no I/O. GAQL result
//! rows (`{"campaign": {...}, "campaignBudget": {...}, "metrics": {...}}`)
//! map into the unified domain models from `mkt-core`.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use mkt_core::error::{MktError, Result};
use mkt_core::models::{
    Budget, BudgetKind, Campaign, CampaignId, CampaignStatus, CreateCampaignInput, InsightsReport,
    InsightsRow, MetricValue, UpdateCampaignInput,
};

/// The GAQL field list for campaign queries.
pub const CAMPAIGN_GAQL_FIELDS: &str = "campaign.id, campaign.name, campaign.status, \
     campaign.advertising_channel_type, campaign.start_date, campaign_budget.amount_micros";

/// Micros per currency unit in Google Ads money fields.
const MICROS_PER_UNIT: f64 = 1_000_000.0;

// ── Status mapping ─────────────────────────────────────────

/// Map a Google Ads campaign status string to a domain status.
pub fn google_status_to_domain(status: &str) -> CampaignStatus {
    match status {
        "ENABLED" => CampaignStatus::Active,
        "PAUSED" => CampaignStatus::Paused,
        "REMOVED" => CampaignStatus::Deleted,
        other => CampaignStatus::Other(other.to_string()),
    }
}

/// Map a domain status to the Google Ads status string.
///
/// Google has no archived state; archived maps to `REMOVED` and draft to
/// `PAUSED`.
pub fn domain_status_to_google(status: &CampaignStatus) -> String {
    match status {
        CampaignStatus::Active => "ENABLED".to_string(),
        CampaignStatus::Paused | CampaignStatus::Draft => "PAUSED".to_string(),
        CampaignStatus::Archived | CampaignStatus::Deleted => "REMOVED".to_string(),
        CampaignStatus::Other(s) => s.clone(),
    }
}

// ── Campaign from API ──────────────────────────────────────

/// Convert one GAQL search result row into a domain [`Campaign`].
///
/// # Errors
///
/// Returns [`MktError::ApiError`] if required fields (`campaign.id`,
/// `campaign.name`) are missing from the row.
pub fn google_row_to_campaign(row: &serde_json::Value) -> Result<Campaign> {
    let campaign = &row["campaign"];
    let id = field_str(campaign, "id")?;
    let name = field_str(campaign, "name")?;
    let status_str = campaign["status"].as_str().unwrap_or("UNKNOWN");
    let objective = campaign["advertisingChannelType"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let budget = row["campaignBudget"]["amountMicros"]
        .as_str()
        .and_then(|micros| micros.parse::<f64>().ok())
        .map(|micros| Budget {
            amount: micros / MICROS_PER_UNIT,
            currency: "USD".to_string(),
            kind: BudgetKind::Daily,
        });

    // Google campaigns expose no creation timestamp via GAQL; use the
    // campaign start date at midnight UTC as the closest stable value.
    let created_at = campaign["startDate"]
        .as_str()
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map_or_else(Utc::now, |dt| DateTime::from_naive_utc_and_offset(dt, Utc));

    Ok(Campaign {
        id: CampaignId(id),
        provider: "google".to_string(),
        name,
        status: google_status_to_domain(status_str),
        objective,
        budget,
        created_at,
        updated_at: None,
        raw: Some(row.clone()),
    })
}

// ── Campaign to API ────────────────────────────────────────

/// Build the campaign resource name `customers/{cid}/campaigns/{id}`.
pub fn campaign_resource_name(customer_id: &str, campaign_id: &str) -> String {
    format!("customers/{customer_id}/campaigns/{campaign_id}")
}

/// Escape a user-supplied value for use inside a GAQL `LIKE` pattern.
///
/// Backslashes and quotes are escaped, and the `LIKE` wildcards `%` and
/// `_` are wrapped in brackets so they match literally — a campaign
/// named "50% off" must not act as a wildcard.
#[must_use]
pub fn escape_gaql_like(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => escaped.push_str("\\\\"),
            '\'' => escaped.push_str("\\'"),
            '%' => escaped.push_str("[%]"),
            '_' => escaped.push_str("[_]"),
            '[' => escaped.push_str("[[]"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// Campaign fields that are members of the bidding-strategy oneof. An
/// explicit strategy in `extra` must replace the `manualCpc` default —
/// the API rejects operations carrying two members.
const BIDDING_STRATEGY_FIELDS: &[&str] = &[
    "biddingStrategy",
    "commission",
    "manualCpa",
    "manualCpc",
    "manualCpm",
    "manualCpv",
    "maximizeConversionValue",
    "maximizeConversions",
    "percentCpc",
    "targetCpa",
    "targetCpm",
    "targetImpressionShare",
    "targetRoas",
    "targetSpend",
];

/// Build the campaign `create` JSON object for a mutate operation.
///
/// `objective` carries the advertising channel type (`SEARCH`, `DISPLAY`,
/// `PERFORMANCE_MAX`, ...). New campaigns default to `PAUSED` so spend is
/// an explicit decision, and to no EU political advertising — a
/// declaration the API requires on every campaign create; declare via
/// `extra` when the campaign does contain it.
fn campaign_create_object(
    input: &CreateCampaignInput,
    budget_resource_name: &str,
) -> serde_json::Value {
    let status = input
        .status
        .as_ref()
        .map_or_else(|| "PAUSED".to_string(), domain_status_to_google);

    let mut create = serde_json::json!({
        "name": input.name,
        "status": status,
        "advertisingChannelType": input.objective.to_uppercase(),
        "campaignBudget": budget_resource_name,
        "manualCpc": {},
        "containsEuPoliticalAdvertising": "DOES_NOT_CONTAIN_EU_POLITICAL_ADVERTISING",
    });

    if let Some(extra) = &input.extra {
        if let (Some(base), Some(overlay)) = (create.as_object_mut(), extra.as_object()) {
            let replaces_bidding = overlay
                .keys()
                .any(|k| k != "manualCpc" && BIDDING_STRATEGY_FIELDS.contains(&k.as_str()));
            if replaces_bidding && !overlay.contains_key("manualCpc") {
                base.remove("manualCpc");
            }
            for (k, v) in overlay {
                base.insert(k.clone(), v.clone());
            }
        }
    }

    create
}

/// Temporary resource ID linking the budget to the campaign inside one
/// atomic `googleAds:mutate` request. Google resolves negative IDs to the
/// resource created earlier in the same operation array.
const TEMP_BUDGET_ID: &str = "-1";

/// Build the `googleAds:mutate` operation array creating a campaign
/// budget and its campaign atomically.
///
/// The budget is created under the temporary resource name
/// `customers/{customer_id}/campaignBudgets/-1` and referenced by the
/// campaign operation in the same request, so a campaign failure cannot
/// leave an orphaned budget behind. The budget operation must precede
/// the campaign operation: temporary IDs only resolve backwards within
/// the array.
///
/// # Errors
///
/// Returns [`MktError::ValidationError`] if `input` carries no budget —
/// Google Ads requires one per campaign.
pub fn atomic_create_operations(
    input: &CreateCampaignInput,
    customer_id: &str,
) -> Result<serde_json::Value> {
    let budget = input
        .budget
        .as_ref()
        .ok_or_else(|| MktError::ValidationError {
            field: "budget".into(),
            message: "a budget is required to create a Google Ads campaign".into(),
        })?;

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    // budgets are positive and far below u64::MAX micros
    let micros = (budget.amount * MICROS_PER_UNIT).round() as u64;
    let temp_budget_resource = format!("customers/{customer_id}/campaignBudgets/{TEMP_BUDGET_ID}");

    Ok(serde_json::json!([
        {
            "campaignBudgetOperation": {
                "create": {
                    "resourceName": temp_budget_resource,
                    "name": format!("{} — budget", input.name),
                    "amountMicros": micros.to_string(),
                    "deliveryMethod": "STANDARD",
                }
            }
        },
        {
            "campaignOperation": {
                "create": campaign_create_object(input, &temp_budget_resource)
            }
        },
    ]))
}

/// Build the `campaigns:mutate` update operation with its field mask.
pub fn campaign_update_operation(
    customer_id: &str,
    campaign_id: &str,
    input: &UpdateCampaignInput,
) -> serde_json::Value {
    let mut update = serde_json::json!({
        "resourceName": campaign_resource_name(customer_id, campaign_id),
    });
    let mut mask: Vec<&str> = Vec::new();

    if let Some(name) = &input.name {
        update["name"] = serde_json::Value::String(name.clone());
        mask.push("name");
    }
    if let Some(status) = &input.status {
        update["status"] = serde_json::Value::String(domain_status_to_google(status));
        mask.push("status");
    }

    serde_json::json!([{
        "updateMask": mask.join(","),
        "update": update,
    }])
}

/// Extract the numeric campaign ID from an atomic `googleAds:mutate`
/// response.
///
/// Scans `mutateOperationResponses` for the entry carrying a
/// `campaignResult` — matching by key rather than by position keeps the
/// extraction correct regardless of how operations were ordered.
///
/// # Errors
///
/// Returns [`MktError::ApiError`] if no entry carries a
/// `campaignResult.resourceName`.
pub fn campaign_id_from_atomic_mutate(resp: &serde_json::Value) -> Result<String> {
    resp["mutateOperationResponses"]
        .as_array()
        .and_then(|responses| {
            responses
                .iter()
                .find_map(|entry| entry["campaignResult"]["resourceName"].as_str())
        })
        .and_then(|name| name.rsplit('/').next())
        .map(String::from)
        .ok_or_else(|| MktError::ApiError {
            provider: "google".into(),
            status: 0,
            message: "atomic mutate response missing campaignResult.resourceName".into(),
            retry_after: None,
        })
}

// ── Insights ───────────────────────────────────────────────

/// Convert a metrics GAQL search response into a domain [`InsightsReport`].
///
/// `costMicros` is converted to currency units under the metric name
/// `cost`. Campaign id/name and segment fields become dimensions.
///
/// # Errors
///
/// Currently infallible; `Result` is kept for parity with other mapping
/// functions.
#[allow(clippy::unnecessary_wraps)] // Result kept for parity with other mapping fns
pub fn google_insights_to_domain(resp: &serde_json::Value) -> Result<InsightsReport> {
    let rows: Vec<InsightsRow> = resp["results"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|row| {
            let mut dimensions = HashMap::new();
            let mut metrics = HashMap::new();

            if let Some(campaign) = row["campaign"].as_object() {
                if let Some(id) = campaign.get("id").and_then(|v| v.as_str()) {
                    dimensions.insert("campaign.id".to_string(), id.to_string());
                }
                if let Some(name) = campaign.get("name").and_then(|v| v.as_str()) {
                    dimensions.insert("campaign.name".to_string(), name.to_string());
                }
            }

            if let Some(segments) = row["segments"].as_object() {
                for (key, val) in segments {
                    if let Some(s) = val.as_str() {
                        dimensions.insert(key.clone(), s.to_string());
                    }
                }
            }

            if let Some(metric_obj) = row["metrics"].as_object() {
                for (key, val) in metric_obj {
                    let parsed = val
                        .as_f64()
                        .or_else(|| val.as_str().and_then(|s| s.parse::<f64>().ok()));
                    if let Some(v) = parsed {
                        if key == "costMicros" {
                            metrics.insert(
                                "cost".to_string(),
                                MetricValue {
                                    value: v / MICROS_PER_UNIT,
                                    formatted: None,
                                },
                            );
                        } else {
                            metrics.insert(
                                key.clone(),
                                MetricValue {
                                    value: v,
                                    formatted: None,
                                },
                            );
                        }
                    }
                }
            }

            InsightsRow {
                dimensions,
                metrics,
            }
        })
        .collect();

    Ok(InsightsReport {
        provider: "google".to_string(),
        date_range: None,
        rows,
        raw: Some(resp.clone()),
    })
}

// ── Helpers ────────────────────────────────────────────────

/// Extract a required string field from a JSON object.
fn field_str(value: &serde_json::Value, field: &str) -> Result<String> {
    value[field]
        .as_str()
        .map(String::from)
        .ok_or_else(|| MktError::ApiError {
            provider: "google".into(),
            status: 0,
            message: format!("missing field 'campaign.{field}' in API response"),
            retry_after: None,
        })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn sample_row() -> serde_json::Value {
        serde_json::json!({
            "campaign": {
                "resourceName": "customers/123/campaigns/456",
                "id": "456",
                "name": "Test Campaign",
                "status": "ENABLED",
                "advertisingChannelType": "SEARCH",
                "startDate": "2026-01-15"
            },
            "campaignBudget": { "amountMicros": "5000000" }
        })
    }

    #[test]
    fn test_google_status_to_domain() {
        assert_eq!(google_status_to_domain("ENABLED"), CampaignStatus::Active);
        assert_eq!(google_status_to_domain("PAUSED"), CampaignStatus::Paused);
        assert_eq!(google_status_to_domain("REMOVED"), CampaignStatus::Deleted);
        assert_eq!(
            google_status_to_domain("UNKNOWN"),
            CampaignStatus::Other("UNKNOWN".into())
        );
    }

    #[test]
    fn test_domain_status_to_google() {
        assert_eq!(domain_status_to_google(&CampaignStatus::Active), "ENABLED");
        assert_eq!(domain_status_to_google(&CampaignStatus::Paused), "PAUSED");
        assert_eq!(domain_status_to_google(&CampaignStatus::Draft), "PAUSED");
        assert_eq!(
            domain_status_to_google(&CampaignStatus::Archived),
            "REMOVED"
        );
        assert_eq!(domain_status_to_google(&CampaignStatus::Deleted), "REMOVED");
    }

    #[test]
    fn test_google_row_to_campaign() {
        let c = google_row_to_campaign(&sample_row()).expect("should parse");
        assert_eq!(c.id.0, "456");
        assert_eq!(c.provider, "google");
        assert_eq!(c.name, "Test Campaign");
        assert_eq!(c.status, CampaignStatus::Active);
        assert_eq!(c.objective, "SEARCH");
        let budget = c.budget.expect("budget should map");
        assert!((budget.amount - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_google_row_missing_id_is_error() {
        let row = serde_json::json!({ "campaign": { "name": "No ID" } });
        assert!(google_row_to_campaign(&row).is_err());
    }

    #[test]
    fn test_atomic_create_operations_budget_precedes_campaign() {
        let input = CreateCampaignInput {
            name: "X".into(),
            objective: "search".into(),
            status: None,
            budget: Some(Budget {
                amount: 12.34,
                currency: "USD".into(),
                kind: BudgetKind::Daily,
            }),
            extra: None,
        };
        let ops = atomic_create_operations(&input, "123").expect("budget present");

        let budget_create = &ops[0]["campaignBudgetOperation"]["create"];
        assert_eq!(
            budget_create["resourceName"],
            "customers/123/campaignBudgets/-1"
        );
        assert_eq!(budget_create["amountMicros"], "12340000");
        assert_eq!(budget_create["name"], "X — budget");
        assert_eq!(budget_create["deliveryMethod"], "STANDARD");

        let campaign_create = &ops[1]["campaignOperation"]["create"];
        assert_eq!(
            campaign_create["campaignBudget"], "customers/123/campaignBudgets/-1",
            "campaign must reference the budget's temporary resource name"
        );
        assert_eq!(campaign_create["status"], "PAUSED");
        assert_eq!(campaign_create["advertisingChannelType"], "SEARCH");
        assert_eq!(
            campaign_create["containsEuPoliticalAdvertising"],
            "DOES_NOT_CONTAIN_EU_POLITICAL_ADVERTISING"
        );
    }

    #[test]
    fn test_atomic_create_operations_without_budget_is_validation_error() {
        let input = CreateCampaignInput {
            name: "X".into(),
            objective: "SEARCH".into(),
            status: None,
            budget: None,
            extra: None,
        };
        let err = atomic_create_operations(&input, "123").expect_err("budget is required");
        assert!(
            matches!(err, MktError::ValidationError { .. }),
            "got: {err}"
        );
    }

    #[test]
    fn test_atomic_create_operations_extra_bidding_replaces_manual_cpc() {
        let input = CreateCampaignInput {
            name: "X".into(),
            objective: "PERFORMANCE_MAX".into(),
            status: None,
            budget: Some(Budget {
                amount: 5.0,
                currency: "USD".into(),
                kind: BudgetKind::Daily,
            }),
            extra: Some(serde_json::json!({ "maximizeConversionValue": {} })),
        };
        let ops = atomic_create_operations(&input, "123").expect("budget present");
        let create = &ops[1]["campaignOperation"]["create"];
        assert!(create.get("maximizeConversionValue").is_some());
        assert!(
            create.get("manualCpc").is_none(),
            "default manualCpc must yield to the explicit strategy: {create}"
        );
    }

    #[test]
    fn test_campaign_id_from_atomic_mutate_finds_campaign_result_by_key() {
        let resp = serde_json::json!({
            "mutateOperationResponses": [
                { "campaignBudgetResult": {
                    "resourceName": "customers/123/campaignBudgets/555"
                }},
                { "campaignResult": {
                    "resourceName": "customers/123/campaigns/789"
                }}
            ]
        });
        assert_eq!(
            campaign_id_from_atomic_mutate(&resp).expect("should extract"),
            "789"
        );
    }

    #[test]
    fn test_campaign_id_from_atomic_mutate_missing_campaign_result_is_error() {
        let resp = serde_json::json!({
            "mutateOperationResponses": [
                { "campaignBudgetResult": {
                    "resourceName": "customers/123/campaignBudgets/555"
                }}
            ]
        });
        let err = campaign_id_from_atomic_mutate(&resp).expect_err("no campaignResult");
        assert!(err.to_string().contains("campaignResult"), "got: {err}");
    }

    #[test]
    fn test_campaign_update_operation_mask_matches_fields() {
        let input = UpdateCampaignInput {
            name: Some("Renamed".into()),
            status: Some(CampaignStatus::Paused),
            budget: None,
            extra: None,
        };
        let ops = campaign_update_operation("123", "456", &input);
        assert_eq!(ops[0]["updateMask"], "name,status");
        assert_eq!(ops[0]["update"]["name"], "Renamed");
        assert_eq!(ops[0]["update"]["status"], "PAUSED");
        assert_eq!(
            ops[0]["update"]["resourceName"],
            "customers/123/campaigns/456"
        );
    }

    #[test]
    fn test_google_insights_to_domain_converts_cost_micros() {
        let resp = serde_json::json!({
            "results": [{
                "campaign": { "id": "1", "name": "C" },
                "metrics": { "impressions": "100", "costMicros": "2500000" },
                "segments": { "date": "2026-03-01" }
            }]
        });
        let report = google_insights_to_domain(&resp).expect("should parse");
        assert_eq!(report.rows.len(), 1);
        let row = &report.rows[0];
        assert!((row.metrics["cost"].value - 2.5).abs() < f64::EPSILON);
        assert!((row.metrics["impressions"].value - 100.0).abs() < f64::EPSILON);
        assert_eq!(
            row.dimensions.get("date").map(String::as_str),
            Some("2026-03-01")
        );
    }

    #[test]
    fn test_escape_gaql_like_neutralizes_wildcards_and_quotes() {
        assert_eq!(escape_gaql_like("50% off"), "50[%] off");
        assert_eq!(escape_gaql_like("foo_bar"), "foo[_]bar");
        assert_eq!(escape_gaql_like("it's"), "it\\'s");
        assert_eq!(escape_gaql_like("a[b]"), "a[[]b]");
        assert_eq!(escape_gaql_like("plain"), "plain");
    }
}
