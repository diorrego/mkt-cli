//! Conversion between LinkedIn Marketing API JSON and domain models.
//!
//! All functions here are pure transformations with no I/O. LinkedIn uses
//! Rest.li conventions: money amounts as strings, epoch-millisecond
//! timestamps, URN references, and `PARTIAL_UPDATE` patches.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use mkt_core::error::{MktError, Result};
use mkt_core::models::{
    Budget, BudgetKind, Campaign, CampaignId, CampaignStatus, CreateCampaignInput, InsightsReport,
    InsightsRow, MetricValue, UpdateCampaignInput,
};

// ── Status mapping ─────────────────────────────────────────

/// Map a LinkedIn campaign status string to a domain status.
pub fn linkedin_status_to_domain(status: &str) -> CampaignStatus {
    match status {
        "ACTIVE" => CampaignStatus::Active,
        "PAUSED" => CampaignStatus::Paused,
        "ARCHIVED" => CampaignStatus::Archived,
        "DRAFT" => CampaignStatus::Draft,
        "PENDING_DELETION" | "REMOVED" => CampaignStatus::Deleted,
        other => CampaignStatus::Other(other.to_string()),
    }
}

/// Map a domain status to the LinkedIn status string.
pub fn domain_status_to_linkedin(status: &CampaignStatus) -> String {
    match status {
        CampaignStatus::Active => "ACTIVE".to_string(),
        CampaignStatus::Paused => "PAUSED".to_string(),
        CampaignStatus::Archived => "ARCHIVED".to_string(),
        CampaignStatus::Draft => "DRAFT".to_string(),
        CampaignStatus::Deleted => "PENDING_DELETION".to_string(),
        CampaignStatus::Other(s) => s.clone(),
    }
}

// ── Campaign from API ──────────────────────────────────────

/// Convert a LinkedIn campaign JSON object into a domain [`Campaign`].
///
/// # Errors
///
/// Returns [`MktError::ApiError`] if required fields (`id`, `name`) are
/// missing from the object.
pub fn linkedin_campaign_to_domain(raw: &serde_json::Value) -> Result<Campaign> {
    let id = raw["id"]
        .as_i64()
        .map(|v| v.to_string())
        .or_else(|| raw["id"].as_str().map(String::from))
        .ok_or_else(|| missing_field("id"))?;
    let name = raw["name"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| missing_field("name"))?;
    let status_str = raw["status"].as_str().unwrap_or("UNKNOWN");
    let objective = raw["objectiveType"].as_str().unwrap_or("").to_string();

    // Money amounts are strings per the Rest.li convention.
    let budget = raw["dailyBudget"].as_object().and_then(|b| {
        let amount = b.get("amount")?.as_str()?.parse::<f64>().ok()?;
        let currency = b
            .get("currencyCode")
            .and_then(|c| c.as_str())
            .unwrap_or("USD");
        Some(Budget {
            amount,
            currency: currency.to_string(),
            kind: BudgetKind::Daily,
        })
    });

    let created_at =
        epoch_ms(&raw["changeAuditStamps"]["created"]["time"]).unwrap_or_else(Utc::now);
    let updated_at = epoch_ms(&raw["changeAuditStamps"]["lastModified"]["time"]);

    Ok(Campaign {
        id: CampaignId(id),
        provider: "linkedin".to_string(),
        name,
        status: linkedin_status_to_domain(status_str),
        objective,
        budget,
        created_at,
        updated_at,
        raw: Some(raw.clone()),
    })
}

// ── Campaign to API ────────────────────────────────────────

/// Build the JSON body for a LinkedIn create-campaign request.
///
/// `objective` carries the LinkedIn `objectiveType` (`LEAD_GENERATION`,
/// `WEBSITE_VISIT`, `BRAND_AWARENESS`, ...). The `campaignGroup` URN is
/// required by LinkedIn and must come through `input.extra`. New campaigns
/// default to `PAUSED` so spend is an explicit decision.
///
/// # Errors
///
/// Returns [`MktError::ValidationError`] if `extra.campaignGroup` is
/// missing or the budget is absent.
pub fn domain_to_linkedin_create_campaign(
    input: &CreateCampaignInput,
    ad_account_id: &str,
    now_epoch_ms: i64,
) -> Result<serde_json::Value> {
    let campaign_group = input
        .extra
        .as_ref()
        .and_then(|e| e["campaignGroup"].as_str())
        .ok_or_else(|| MktError::ValidationError {
            field: "campaignGroup".into(),
            message: "LinkedIn requires a campaign group URN; pass \
                      --extra '{\"campaignGroup\":\"urn:li:sponsoredCampaignGroup:<id>\"}'"
                .into(),
        })?;

    let budget = input
        .budget
        .as_ref()
        .ok_or_else(|| MktError::ValidationError {
            field: "budget".into(),
            message: "a daily budget is required to create a LinkedIn campaign".into(),
        })?;

    let status = input
        .status
        .as_ref()
        .map_or_else(|| "PAUSED".to_string(), domain_status_to_linkedin);

    let mut body = serde_json::json!({
        "account": format!("urn:li:sponsoredAccount:{ad_account_id}"),
        "campaignGroup": campaign_group,
        "name": input.name,
        "type": "SPONSORED_UPDATES",
        "objectiveType": input.objective,
        "costType": "CPC",
        "status": status,
        "dailyBudget": {
            // Rest.li money amounts are strings.
            "amount": format_amount(budget.amount),
            "currencyCode": budget.currency,
        },
        "locale": { "country": "US", "language": "en" },
        "runSchedule": { "start": now_epoch_ms },
    });

    if let Some(extra) = &input.extra {
        if let (Some(base), Some(overlay)) = (body.as_object_mut(), extra.as_object()) {
            for (k, v) in overlay {
                base.insert(k.clone(), v.clone());
            }
        }
    }

    Ok(body)
}

/// Build the Rest.li `PARTIAL_UPDATE` patch body for an update.
pub fn domain_to_linkedin_update_patch(input: &UpdateCampaignInput) -> serde_json::Value {
    let mut set = serde_json::Map::new();

    if let Some(name) = &input.name {
        set.insert("name".into(), serde_json::Value::String(name.clone()));
    }
    if let Some(status) = &input.status {
        set.insert(
            "status".into(),
            serde_json::Value::String(domain_status_to_linkedin(status)),
        );
    }
    if let Some(budget) = &input.budget {
        set.insert(
            "dailyBudget".into(),
            serde_json::json!({
                "amount": format_amount(budget.amount),
                "currencyCode": budget.currency,
            }),
        );
    }

    serde_json::json!({ "patch": { "$set": set } })
}

// ── Insights ───────────────────────────────────────────────

/// Convert an adAnalytics response into a domain [`InsightsReport`].
///
/// `costInLocalCurrency` (a string) becomes the numeric `cost` metric;
/// integer counters map directly; `dateRange.start` becomes the `date`
/// dimension and the first pivot URN becomes `pivot`.
///
/// # Errors
///
/// Currently infallible; `Result` is kept for parity with other mapping
/// functions.
#[allow(clippy::unnecessary_wraps)] // Result kept for parity with other mapping fns
pub fn linkedin_analytics_to_domain(resp: &serde_json::Value) -> Result<InsightsReport> {
    let rows: Vec<InsightsRow> = resp["elements"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|el| {
            let mut dimensions = HashMap::new();
            let mut metrics = HashMap::new();

            if let Some(obj) = el.as_object() {
                for (key, val) in obj {
                    match key.as_str() {
                        "dateRange" => {
                            if let Some(date) = format_date(&val["start"]) {
                                dimensions.insert("date".to_string(), date);
                            }
                        }
                        "pivotValues" => {
                            if let Some(pivot) = val.as_array().and_then(|a| a.first()) {
                                if let Some(s) = pivot.as_str() {
                                    dimensions.insert("pivot".to_string(), s.to_string());
                                }
                            }
                        }
                        "costInLocalCurrency" => {
                            if let Some(v) = val.as_str().and_then(|s| s.parse::<f64>().ok()) {
                                metrics.insert(
                                    "cost".to_string(),
                                    MetricValue {
                                        value: v,
                                        formatted: None,
                                    },
                                );
                            }
                        }
                        _ => {
                            if let Some(v) = val.as_f64() {
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
            }

            InsightsRow {
                dimensions,
                metrics,
            }
        })
        .collect();

    Ok(InsightsReport {
        provider: "linkedin".to_string(),
        date_range: None,
        rows,
        raw: Some(resp.clone()),
    })
}

// ── Helpers ────────────────────────────────────────────────

/// Format a money amount the way LinkedIn expects: integral values
/// without a trailing `.0`, fractional values as-is.
fn format_amount(amount: f64) -> String {
    if (amount.fract()).abs() < f64::EPSILON {
        #[allow(clippy::cast_possible_truncation)] // integral check above
        let int = amount as i64;
        int.to_string()
    } else {
        amount.to_string()
    }
}

/// Parse an epoch-millisecond JSON number into a UTC datetime.
fn epoch_ms(value: &serde_json::Value) -> Option<DateTime<Utc>> {
    value.as_i64().and_then(DateTime::from_timestamp_millis)
}

/// Format a `{year, month, day}` object as `YYYY-MM-DD`.
fn format_date(value: &serde_json::Value) -> Option<String> {
    let year = value["year"].as_i64()?;
    let month = value["month"].as_i64()?;
    let day = value["day"].as_i64()?;
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

fn missing_field(field: &str) -> MktError {
    MktError::ApiError {
        provider: "linkedin".into(),
        status: 0,
        message: format!("missing field '{field}' in API response"),
        retry_after: None,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_linkedin_status_to_domain() {
        assert_eq!(linkedin_status_to_domain("ACTIVE"), CampaignStatus::Active);
        assert_eq!(linkedin_status_to_domain("PAUSED"), CampaignStatus::Paused);
        assert_eq!(
            linkedin_status_to_domain("ARCHIVED"),
            CampaignStatus::Archived
        );
        assert_eq!(linkedin_status_to_domain("DRAFT"), CampaignStatus::Draft);
        assert_eq!(
            linkedin_status_to_domain("PENDING_DELETION"),
            CampaignStatus::Deleted
        );
        assert_eq!(
            linkedin_status_to_domain("COMPLETED"),
            CampaignStatus::Other("COMPLETED".into())
        );
    }

    #[test]
    fn test_domain_status_to_linkedin() {
        assert_eq!(domain_status_to_linkedin(&CampaignStatus::Active), "ACTIVE");
        assert_eq!(domain_status_to_linkedin(&CampaignStatus::Draft), "DRAFT");
        assert_eq!(
            domain_status_to_linkedin(&CampaignStatus::Deleted),
            "PENDING_DELETION"
        );
    }

    #[test]
    fn test_linkedin_campaign_to_domain() {
        let raw = serde_json::json!({
            "id": 145_282_384,
            "name": "Test",
            "status": "ACTIVE",
            "objectiveType": "LEAD_GENERATION",
            "dailyBudget": { "amount": "18", "currencyCode": "USD" },
            "changeAuditStamps": {
                "created": { "time": 1_767_225_600_000_i64 },
                "lastModified": { "time": 1_772_495_400_000_i64 }
            }
        });
        let c = linkedin_campaign_to_domain(&raw).expect("should parse");
        assert_eq!(c.id.0, "145282384");
        assert_eq!(c.provider, "linkedin");
        assert_eq!(c.status, CampaignStatus::Active);
        assert_eq!(c.objective, "LEAD_GENERATION");
        let b = c.budget.expect("budget maps");
        assert!((b.amount - 18.0).abs() < f64::EPSILON);
        assert_eq!(c.created_at.format("%Y-%m-%d").to_string(), "2026-01-01");
        assert!(c.updated_at.is_some());
    }

    #[test]
    fn test_linkedin_campaign_missing_name_is_error() {
        let raw = serde_json::json!({ "id": 1 });
        assert!(linkedin_campaign_to_domain(&raw).is_err());
    }

    #[test]
    fn test_create_campaign_body() {
        let input = CreateCampaignInput {
            name: "X".into(),
            objective: "WEBSITE_VISIT".into(),
            status: None,
            budget: Some(Budget {
                amount: 18.0,
                currency: "USD".into(),
                kind: BudgetKind::Daily,
            }),
            extra: Some(serde_json::json!({
                "campaignGroup": "urn:li:sponsoredCampaignGroup:603030884",
            })),
        };
        let body =
            domain_to_linkedin_create_campaign(&input, "506333826", 1_767_225_600_000).expect("ok");
        assert_eq!(body["account"], "urn:li:sponsoredAccount:506333826");
        assert_eq!(body["status"], "PAUSED", "must default to paused");
        assert_eq!(body["dailyBudget"]["amount"], "18", "money is a string");
        assert_eq!(body["runSchedule"]["start"], 1_767_225_600_000_i64);
        assert_eq!(body["objectiveType"], "WEBSITE_VISIT");
    }

    #[test]
    fn test_create_campaign_requires_campaign_group() {
        let input = CreateCampaignInput {
            name: "X".into(),
            objective: "WEBSITE_VISIT".into(),
            status: None,
            budget: Some(Budget {
                amount: 18.0,
                currency: "USD".into(),
                kind: BudgetKind::Daily,
            }),
            extra: None,
        };
        let err = domain_to_linkedin_create_campaign(&input, "1", 0).expect_err("must fail");
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn test_create_campaign_requires_budget() {
        let input = CreateCampaignInput {
            name: "X".into(),
            objective: "WEBSITE_VISIT".into(),
            status: None,
            budget: None,
            extra: Some(serde_json::json!({
                "campaignGroup": "urn:li:sponsoredCampaignGroup:1",
            })),
        };
        let err = domain_to_linkedin_create_campaign(&input, "1", 0).expect_err("must fail");
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn test_update_patch_shape() {
        let input = UpdateCampaignInput {
            name: Some("Renamed".into()),
            status: Some(CampaignStatus::Paused),
            budget: Some(Budget {
                amount: 30.5,
                currency: "USD".into(),
                kind: BudgetKind::Daily,
            }),
            extra: None,
        };
        let patch = domain_to_linkedin_update_patch(&input);
        assert_eq!(patch["patch"]["$set"]["name"], "Renamed");
        assert_eq!(patch["patch"]["$set"]["status"], "PAUSED");
        assert_eq!(patch["patch"]["$set"]["dailyBudget"]["amount"], "30.5");
    }

    #[test]
    fn test_analytics_mapping() {
        let resp = serde_json::json!({
            "elements": [{
                "impressions": 165,
                "clicks": 11,
                "costInLocalCurrency": "19.91833",
                "dateRange": { "start": { "year": 2026, "month": 3, "day": 1 } },
                "pivotValues": ["urn:li:sponsoredCampaign:1"]
            }]
        });
        let report = linkedin_analytics_to_domain(&resp).expect("should parse");
        let row = &report.rows[0];
        assert!((row.metrics["cost"].value - 19.91833).abs() < 1e-9);
        assert!((row.metrics["impressions"].value - 165.0).abs() < f64::EPSILON);
        assert_eq!(row.dimensions["date"], "2026-03-01");
        assert_eq!(row.dimensions["pivot"], "urn:li:sponsoredCampaign:1");
    }

    #[test]
    fn test_format_amount() {
        assert_eq!(format_amount(18.0), "18");
        assert_eq!(format_amount(30.5), "30.5");
    }
}
