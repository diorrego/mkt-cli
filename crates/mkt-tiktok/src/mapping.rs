//! Conversion between TikTok Business API v1.3 JSON and domain models.
//!
//! All functions here are pure transformations with no I/O. v1.3 quirks:
//! numeric IDs arrive as JSON numbers in responses but are sent as strings
//! in requests; report metric values are strings; timestamps are
//! `"YYYY-MM-DD HH:MM:SS"` in UTC.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDateTime, Utc};
use mkt_core::error::{MktError, Result};
use mkt_core::models::{
    Audience, AudienceId, AudienceType, Budget, BudgetKind, Campaign, CampaignId, CampaignStatus,
    CreateCampaignInput, InsightsReport, InsightsRow, MetricValue,
};

// ── Status mapping ─────────────────────────────────────────

/// Map a TikTok `operation_status` to a domain status.
pub fn tiktok_status_to_domain(status: &str) -> CampaignStatus {
    match status {
        "ENABLE" => CampaignStatus::Active,
        "DISABLE" => CampaignStatus::Paused,
        "DELETE" => CampaignStatus::Deleted,
        other => CampaignStatus::Other(other.to_string()),
    }
}

/// Map a domain status to a TikTok `operation_status`.
///
/// TikTok has no archived/draft states; both map to `DISABLE`.
pub fn domain_status_to_tiktok(status: &CampaignStatus) -> String {
    match status {
        CampaignStatus::Active => "ENABLE".to_string(),
        CampaignStatus::Paused | CampaignStatus::Draft | CampaignStatus::Archived => {
            "DISABLE".to_string()
        }
        CampaignStatus::Deleted => "DELETE".to_string(),
        CampaignStatus::Other(s) => s.clone(),
    }
}

/// Map a domain status to the `secondary_status` filter value used by
/// `GET /campaign/get/` filtering.
pub const fn domain_status_to_filter(status: &CampaignStatus) -> &'static str {
    match status {
        CampaignStatus::Active => "CAMPAIGN_STATUS_ENABLE",
        CampaignStatus::Deleted => "CAMPAIGN_STATUS_DELETE",
        _ => "CAMPAIGN_STATUS_DISABLE",
    }
}

// ── Campaign from API ──────────────────────────────────────

/// Read an ID that may arrive as a JSON number or string.
fn id_to_string(value: &serde_json::Value) -> Option<String> {
    value
        .as_i64()
        .map(|v| v.to_string())
        .or_else(|| value.as_str().map(String::from))
}

/// Convert a v1.3 campaign list item into a domain [`Campaign`].
///
/// # Errors
///
/// Returns [`MktError::ApiError`] if required fields (`campaign_id`,
/// `campaign_name`) are missing.
pub fn tiktok_campaign_to_domain(raw: &serde_json::Value) -> Result<Campaign> {
    let id = id_to_string(&raw["campaign_id"]).ok_or_else(|| missing_field("campaign_id"))?;
    let name = raw["campaign_name"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| missing_field("campaign_name"))?;
    let status_str = raw["operation_status"].as_str().unwrap_or("UNKNOWN");
    let objective = raw["objective_type"].as_str().unwrap_or("").to_string();

    let budget = raw["budget"].as_f64().and_then(|amount| {
        let kind = match raw["budget_mode"].as_str() {
            Some("BUDGET_MODE_DAY" | "BUDGET_MODE_DYNAMIC_DAILY_BUDGET") => BudgetKind::Daily,
            Some("BUDGET_MODE_TOTAL") => BudgetKind::Lifetime,
            // BUDGET_MODE_INFINITE or unknown: no meaningful budget amount.
            _ => return None,
        };
        Some(Budget {
            amount,
            currency: "USD".to_string(),
            kind,
        })
    });

    let created_at = parse_tiktok_time(raw["create_time"].as_str()).unwrap_or_else(Utc::now);
    let updated_at = parse_tiktok_time(raw["modify_time"].as_str());

    Ok(Campaign {
        id: CampaignId(id),
        provider: "tiktok".to_string(),
        name,
        status: tiktok_status_to_domain(status_str),
        objective,
        budget,
        created_at,
        updated_at,
        raw: Some(raw.clone()),
    })
}

// ── Campaign to API ────────────────────────────────────────

/// Build the JSON body for `POST /campaign/create/`.
///
/// `objective` carries the TikTok `objective_type` (`TRAFFIC`,
/// `LEAD_GENERATION`, `WEB_CONVERSIONS`, ...). New campaigns default to
/// `DISABLE` (paused) so spend is an explicit decision — note this
/// inverts TikTok's own default of `ENABLE`. `request_id` is TikTok's
/// idempotency token: retries reusing it do not create a second campaign.
pub fn domain_to_tiktok_create_campaign(
    input: &CreateCampaignInput,
    advertiser_id: &str,
    request_id: &str,
) -> serde_json::Value {
    let operation_status = input
        .status
        .as_ref()
        .map_or_else(|| "DISABLE".to_string(), domain_status_to_tiktok);

    let mut body = serde_json::json!({
        "advertiser_id": advertiser_id,
        "campaign_name": input.name,
        "objective_type": input.objective,
        "operation_status": operation_status,
        "request_id": request_id,
    });

    if let Some(budget) = &input.budget {
        let mode = match budget.kind {
            BudgetKind::Daily => "BUDGET_MODE_DAY",
            BudgetKind::Lifetime => "BUDGET_MODE_TOTAL",
        };
        body["budget_mode"] = serde_json::Value::String(mode.to_string());
        body["budget"] = serde_json::json!(budget.amount);
    } else {
        body["budget_mode"] = serde_json::Value::String("BUDGET_MODE_INFINITE".to_string());
    }

    if let Some(extra) = &input.extra {
        if let (Some(base), Some(overlay)) = (body.as_object_mut(), extra.as_object()) {
            for (k, v) in overlay {
                base.insert(k.clone(), v.clone());
            }
        }
    }

    body
}

// ── Audiences ──────────────────────────────────────────────

/// Convert a DMP custom audience list response into domain [`Audience`]s.
///
/// # Errors
///
/// Returns [`MktError::ApiError`] if a required field is missing from any
/// audience object.
pub fn tiktok_audiences_to_domain(data: &serde_json::Value) -> Result<Vec<Audience>> {
    data["list"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|raw| {
            let id = id_to_string(&raw["custom_audience_id"])
                .or_else(|| id_to_string(&raw["audience_id"]))
                .ok_or_else(|| missing_field("custom_audience_id"))?;
            let name = raw["name"]
                .as_str()
                .map(String::from)
                .ok_or_else(|| missing_field("name"))?;

            Ok(Audience {
                id: AudienceId(id),
                provider: "tiktok".to_string(),
                name,
                description: raw["audience_type"].as_str().map(String::from),
                size: raw["cover_num"].as_u64(),
                audience_type: AudienceType::Custom,
                created_at: parse_tiktok_time(raw["create_time"].as_str()).unwrap_or_else(Utc::now),
                raw: Some(raw.clone()),
            })
        })
        .collect()
}

// ── Insights ───────────────────────────────────────────────

/// Convert a `GET /report/integrated/get/` response into a domain
/// [`InsightsReport`].
///
/// Metric values arrive as strings and are parsed to numbers; dimension
/// values (numeric IDs, `stat_time_day`) become string dimensions, with
/// `stat_time_day` shortened to a `date` dimension (`YYYY-MM-DD`).
///
/// # Errors
///
/// Currently infallible; `Result` is kept for parity with other mapping
/// functions.
#[allow(clippy::unnecessary_wraps)] // Result kept for parity with other mapping fns
pub fn tiktok_report_to_domain(data: &serde_json::Value) -> Result<InsightsReport> {
    let rows: Vec<InsightsRow> = data["list"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|row| {
            let mut dimensions = HashMap::new();
            let mut metrics = HashMap::new();

            if let Some(dims) = row["dimensions"].as_object() {
                for (key, val) in dims {
                    let str_val = val
                        .as_str()
                        .map(String::from)
                        .or_else(|| val.as_i64().map(|v| v.to_string()));
                    if let Some(v) = str_val {
                        if key == "stat_time_day" {
                            // "2026-03-01 00:00:00" -> "2026-03-01"
                            let day = v.split(' ').next().unwrap_or(&v).to_string();
                            dimensions.insert("date".to_string(), day);
                        } else {
                            dimensions.insert(key.clone(), v);
                        }
                    }
                }
            }

            if let Some(metric_obj) = row["metrics"].as_object() {
                for (key, val) in metric_obj {
                    let parsed = val
                        .as_str()
                        .and_then(|s| s.parse::<f64>().ok())
                        .or_else(|| val.as_f64());
                    if let Some(v) = parsed {
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

            InsightsRow {
                dimensions,
                metrics,
            }
        })
        .collect();

    Ok(InsightsReport {
        provider: "tiktok".to_string(),
        date_range: None,
        rows,
        raw: Some(data.clone()),
    })
}

// ── Helpers ────────────────────────────────────────────────

/// Parse a TikTok `"YYYY-MM-DD HH:MM:SS"` UTC timestamp.
fn parse_tiktok_time(value: Option<&str>) -> Option<DateTime<Utc>> {
    value
        .and_then(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").ok())
        .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
}

fn missing_field(field: &str) -> MktError {
    MktError::ApiError {
        provider: "tiktok".into(),
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
    fn test_tiktok_status_to_domain() {
        assert_eq!(tiktok_status_to_domain("ENABLE"), CampaignStatus::Active);
        assert_eq!(tiktok_status_to_domain("DISABLE"), CampaignStatus::Paused);
        assert_eq!(tiktok_status_to_domain("DELETE"), CampaignStatus::Deleted);
        assert_eq!(
            tiktok_status_to_domain("X"),
            CampaignStatus::Other("X".into())
        );
    }

    #[test]
    fn test_domain_status_to_tiktok() {
        assert_eq!(domain_status_to_tiktok(&CampaignStatus::Active), "ENABLE");
        assert_eq!(domain_status_to_tiktok(&CampaignStatus::Paused), "DISABLE");
        assert_eq!(
            domain_status_to_tiktok(&CampaignStatus::Archived),
            "DISABLE"
        );
        assert_eq!(domain_status_to_tiktok(&CampaignStatus::Deleted), "DELETE");
    }

    #[test]
    fn test_campaign_mapping_with_numeric_id() {
        let raw = serde_json::json!({
            "campaign_id": 1_680_018_437_245_954_i64,
            "campaign_name": "Test",
            "objective_type": "TRAFFIC",
            "budget": 50.0,
            "budget_mode": "BUDGET_MODE_DAY",
            "operation_status": "ENABLE",
            "create_time": "2026-01-13 13:44:30",
            "modify_time": "2026-03-01 09:12:00"
        });
        let c = tiktok_campaign_to_domain(&raw).expect("should parse");
        assert_eq!(c.id.0, "1680018437245954");
        assert_eq!(c.provider, "tiktok");
        assert_eq!(c.status, CampaignStatus::Active);
        assert_eq!(c.objective, "TRAFFIC");
        let b = c.budget.expect("budget maps");
        assert!((b.amount - 50.0).abs() < f64::EPSILON);
        assert_eq!(b.kind, BudgetKind::Daily);
        assert!(c.updated_at.is_some());
    }

    #[test]
    fn test_campaign_mapping_infinite_budget_is_none() {
        let raw = serde_json::json!({
            "campaign_id": "1",
            "campaign_name": "X",
            "budget": 0.0,
            "budget_mode": "BUDGET_MODE_INFINITE",
            "operation_status": "ENABLE",
            "create_time": "2026-01-01 00:00:00"
        });
        let c = tiktok_campaign_to_domain(&raw).expect("should parse");
        assert!(c.budget.is_none());
    }

    #[test]
    fn test_campaign_mapping_missing_name_is_error() {
        let raw = serde_json::json!({ "campaign_id": 1 });
        assert!(tiktok_campaign_to_domain(&raw).is_err());
    }

    #[test]
    fn test_create_body_defaults_to_disable() {
        let input = CreateCampaignInput {
            name: "X".into(),
            objective: "TRAFFIC".into(),
            status: None,
            budget: Some(Budget {
                amount: 50.0,
                currency: "USD".into(),
                kind: BudgetKind::Daily,
            }),
            extra: None,
        };
        let body = domain_to_tiktok_create_campaign(&input, "123", "req-1");
        assert_eq!(body["advertiser_id"], "123");
        assert_eq!(body["request_id"], "req-1");
        assert_eq!(body["operation_status"], "DISABLE");
        assert_eq!(body["budget_mode"], "BUDGET_MODE_DAY");
        assert!((body["budget"].as_f64().expect("budget") - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_create_body_without_budget_is_infinite() {
        let input = CreateCampaignInput {
            name: "X".into(),
            objective: "REACH".into(),
            status: None,
            budget: None,
            extra: None,
        };
        let body = domain_to_tiktok_create_campaign(&input, "123", "req-2");
        assert_eq!(body["budget_mode"], "BUDGET_MODE_INFINITE");
        assert!(body.get("budget").is_none());
    }

    #[test]
    fn test_audiences_mapping() {
        let data = serde_json::json!({
            "list": [{
                "custom_audience_id": 102_400_000_001_i64,
                "name": "Test Audience",
                "audience_type": "Customer File",
                "cover_num": 48_000,
                "create_time": "2026-02-15 16:30:00"
            }]
        });
        let audiences = tiktok_audiences_to_domain(&data).expect("should parse");
        assert_eq!(audiences.len(), 1);
        assert_eq!(audiences[0].id.0, "102400000001");
        assert_eq!(audiences[0].size, Some(48_000));
    }

    #[test]
    fn test_report_mapping_parses_string_metrics() {
        let data = serde_json::json!({
            "list": [{
                "dimensions": {
                    "campaign_id": 1_680_018_437_245_954_i64,
                    "stat_time_day": "2026-03-01 00:00:00"
                },
                "metrics": { "spend": "234.56", "impressions": "45230" }
            }]
        });
        let report = tiktok_report_to_domain(&data).expect("should parse");
        let row = &report.rows[0];
        assert!((row.metrics["spend"].value - 234.56).abs() < 1e-9);
        assert_eq!(row.dimensions["date"], "2026-03-01");
        assert_eq!(row.dimensions["campaign_id"], "1680018437245954");
    }
}
