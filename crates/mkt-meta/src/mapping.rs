//! Conversion between Meta Graph API JSON and domain models.
//!
//! All functions here are pure transformations with no I/O.  They are
//! called by [`crate::provider::MetaProvider`] after the raw JSON has
//! been fetched (or before it is sent).

use chrono::{DateTime, Utc};
use std::collections::HashMap;

use mkt_core::error::{MktError, Result};
use mkt_core::models::{
    Ad, AdId, AdSet, AdSetId, AdSetStatus, AdStatus, Audience, AudienceId, AudienceType,
    AudienceUser, Budget, BudgetKind, Campaign, CampaignId, CampaignStatus, CreateAdSetInput,
    CreateAudienceInput, CreateCampaignInput, CreativeId, InsightsReport, InsightsRow, MetricValue,
    Post, PostId, UpdateCampaignInput,
};
use mkt_core::pii;

/// The set of fields requested from the campaigns endpoint.
pub const CAMPAIGN_FIELDS: &str = "id,name,status,objective,daily_budget,lifetime_budget,\
     created_time,updated_time";

/// The set of fields requested from the adsets endpoint.
pub const ADSET_FIELDS: &str = "id,campaign_id,name,status,targeting,daily_budget,\
     lifetime_budget,billing_event,optimization_goal,created_time,updated_time";

/// The set of fields requested from the ads endpoint.
pub const AD_FIELDS: &str = "id,adset_id,name,status,creative,created_time,updated_time";

// ── Status mapping ─────────────────────────────────────────

/// Map a Meta API uppercase status string to a domain status.
pub fn meta_status_to_domain(status: &str) -> CampaignStatus {
    match status {
        "ACTIVE" => CampaignStatus::Active,
        "PAUSED" => CampaignStatus::Paused,
        "ARCHIVED" => CampaignStatus::Archived,
        "DELETED" => CampaignStatus::Deleted,
        other => CampaignStatus::Other(other.to_string()),
    }
}

/// Map a domain status back to the Meta API uppercase string.
pub fn domain_status_to_meta(status: &CampaignStatus) -> String {
    match status {
        CampaignStatus::Active => "ACTIVE".to_string(),
        CampaignStatus::Paused | CampaignStatus::Draft => "PAUSED".to_string(),
        CampaignStatus::Archived => "ARCHIVED".to_string(),
        CampaignStatus::Deleted => "DELETED".to_string(),
        CampaignStatus::Other(s) => s.clone(),
    }
}

// ── Campaign from API ──────────────────────────────────────

/// Convert a raw Graph API campaign JSON object into a domain [`Campaign`].
///
/// # Errors
///
/// Returns [`MktError::ApiError`] if required fields (`id`, `name`)
/// are missing from the response.
pub fn meta_campaign_to_domain(raw: &serde_json::Value) -> Result<Campaign> {
    let id = extract_str(raw, "id")?;
    let name = extract_str(raw, "name")?;
    let status_str = raw["status"].as_str().unwrap_or("UNKNOWN");
    let objective = raw["objective"].as_str().unwrap_or("").to_string();

    let budget = parse_budget(raw);
    let created_at = parse_datetime(raw, "created_time")?;
    let updated_at = parse_optional_datetime(raw, "updated_time");

    Ok(Campaign {
        id: CampaignId(id),
        provider: "meta".to_string(),
        name,
        status: meta_status_to_domain(status_str),
        objective,
        budget,
        created_at,
        updated_at,
        raw: Some(raw.clone()),
    })
}

// ── Campaign to API ────────────────────────────────────────

/// Build the JSON body for a Meta create-campaign request.
pub fn domain_to_meta_create_campaign(input: &CreateCampaignInput) -> serde_json::Value {
    let mut body = serde_json::json!({
        "name": input.name,
        "objective": input.objective,
        "special_ad_categories": [],
    });

    if let Some(status) = &input.status {
        body["status"] = serde_json::Value::String(domain_status_to_meta(status));
    }

    apply_budget(&mut body, input.budget.as_ref());

    if let Some(extra) = &input.extra {
        merge_json(&mut body, extra);
    }

    body
}

/// Build the JSON body for a Meta update-campaign request.
pub fn domain_to_meta_update_campaign(input: &UpdateCampaignInput) -> serde_json::Value {
    let mut body = serde_json::json!({});

    if let Some(name) = &input.name {
        body["name"] = serde_json::Value::String(name.clone());
    }
    if let Some(status) = &input.status {
        body["status"] = serde_json::Value::String(domain_status_to_meta(status));
    }

    apply_budget(&mut body, input.budget.as_ref());

    if let Some(extra) = &input.extra {
        merge_json(&mut body, extra);
    }

    body
}

// ── Ad Sets ───────────────────────────────────────────────

/// Map a Meta API uppercase ad set status string to a domain status.
pub fn meta_adset_status_to_domain(status: &str) -> AdSetStatus {
    match status {
        "ACTIVE" => AdSetStatus::Active,
        "PAUSED" => AdSetStatus::Paused,
        "ARCHIVED" => AdSetStatus::Archived,
        "DELETED" => AdSetStatus::Deleted,
        other => AdSetStatus::Other(other.to_string()),
    }
}

/// Map a domain ad set status back to the Meta API uppercase string.
pub fn domain_adset_status_to_meta(status: &AdSetStatus) -> String {
    match status {
        AdSetStatus::Active => "ACTIVE".to_string(),
        AdSetStatus::Paused => "PAUSED".to_string(),
        AdSetStatus::Archived => "ARCHIVED".to_string(),
        AdSetStatus::Deleted => "DELETED".to_string(),
        AdSetStatus::Other(s) => s.clone(),
    }
}

/// Convert a raw Graph API ad set JSON object into a domain [`AdSet`].
///
/// # Errors
///
/// Returns [`MktError::ApiError`] if required fields (`id`, `name`,
/// `campaign_id`) are missing from the response.
pub fn meta_adset_to_domain(raw: &serde_json::Value) -> Result<AdSet> {
    let id = extract_str(raw, "id")?;
    let name = extract_str(raw, "name")?;
    let campaign_id = extract_str(raw, "campaign_id")?;
    let status_str = raw["status"].as_str().unwrap_or("UNKNOWN");

    let targeting = raw.get("targeting").filter(|t| !t.is_null()).cloned();
    let budget = parse_budget(raw);
    let created_at = parse_datetime(raw, "created_time")?;
    let updated_at = parse_optional_datetime(raw, "updated_time");

    Ok(AdSet {
        id: AdSetId(id),
        provider: "meta".to_string(),
        campaign_id: CampaignId(campaign_id),
        name,
        status: meta_adset_status_to_domain(status_str),
        targeting,
        budget,
        created_at,
        updated_at,
        raw: Some(raw.clone()),
    })
}

/// Build the JSON body for a Meta create-adset request.
pub fn domain_to_meta_create_adset(input: &CreateAdSetInput) -> serde_json::Value {
    let mut body = serde_json::json!({
        "campaign_id": input.campaign_id.0,
        "name": input.name,
    });

    if let Some(status) = &input.status {
        body["status"] = serde_json::Value::String(domain_adset_status_to_meta(status));
    }

    if let Some(targeting) = &input.targeting {
        body["targeting"] = targeting.clone();
    }

    apply_budget(&mut body, input.budget.as_ref());

    if let Some(extra) = &input.extra {
        merge_json(&mut body, extra);
    }

    body
}

// ── Ads ───────────────────────────────────────────────────

/// Map a Meta API uppercase ad status string to a domain status.
pub fn meta_ad_status_to_domain(status: &str) -> AdStatus {
    match status {
        "ACTIVE" => AdStatus::Active,
        "PAUSED" => AdStatus::Paused,
        "ARCHIVED" => AdStatus::Archived,
        "DELETED" => AdStatus::Deleted,
        other => AdStatus::Other(other.to_string()),
    }
}

/// Convert a raw Graph API ad JSON object into a domain [`Ad`].
///
/// # Errors
///
/// Returns [`MktError::ApiError`] if required fields (`id`, `name`,
/// `adset_id`) are missing from the response.
pub fn meta_ad_to_domain(raw: &serde_json::Value) -> Result<Ad> {
    let id = extract_str(raw, "id")?;
    let name = extract_str(raw, "name")?;
    let adset_id = extract_str(raw, "adset_id")?;
    let status_str = raw["status"].as_str().unwrap_or("UNKNOWN");

    let creative_id = raw["creative"]["id"]
        .as_str()
        .map(|s| CreativeId(s.to_string()));
    let created_at = parse_datetime(raw, "created_time")?;
    let updated_at = parse_optional_datetime(raw, "updated_time");

    Ok(Ad {
        id: AdId(id),
        provider: "meta".to_string(),
        adset_id: AdSetId(adset_id),
        name,
        status: meta_ad_status_to_domain(status_str),
        creative_id,
        created_at,
        updated_at,
        raw: Some(raw.clone()),
    })
}

// ── Audience user upload ──────────────────────────────────

/// Build the `payload` body for a Meta `POST /{audience_id}/users` request.
///
/// Identifiers are normalized and SHA-256 hashed per Meta's customer file
/// requirements ([`mkt_core::pii`]). The schema only includes columns for
/// which at least one user has a value; missing cells are empty strings.
/// `EXTERN_ID` values are not hashed, per the Meta contract.
pub fn domain_users_to_meta_payload(users: &[AudienceUser]) -> serde_json::Value {
    let has_email = users.iter().any(|u| u.email.is_some());
    let has_phone = users.iter().any(|u| u.phone.is_some());
    let has_extern = users.iter().any(|u| u.external_id.is_some());

    let mut schema: Vec<&str> = Vec::new();
    if has_email {
        schema.push("EMAIL");
    }
    if has_phone {
        schema.push("PHONE");
    }
    if has_extern {
        schema.push("EXTERN_ID");
    }

    let data: Vec<Vec<String>> = users
        .iter()
        .map(|u| {
            let mut row = Vec::new();
            if has_email {
                row.push(u.email.as_deref().map(pii::hash_email).unwrap_or_default());
            }
            if has_phone {
                row.push(u.phone.as_deref().map(pii::hash_phone).unwrap_or_default());
            }
            if has_extern {
                row.push(u.external_id.clone().unwrap_or_default());
            }
            row
        })
        .collect();

    serde_json::json!({
        "payload": {
            "schema": schema,
            "data": data,
        }
    })
}

// ── Audiences ─────────────────────────────────────────────

/// Convert a Meta customaudiences list response to a vector of domain [`Audience`]s.
///
/// # Errors
///
/// Returns [`MktError::ApiError`] if a required field is missing from any audience object.
pub fn meta_audiences_to_domain(resp: &serde_json::Value) -> Result<Vec<Audience>> {
    resp["data"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(meta_audience_to_domain)
        .collect()
}

/// Convert a single Meta custom audience JSON object into a domain [`Audience`].
///
/// # Errors
///
/// Returns [`MktError::ApiError`] if required fields (`id`, `name`) are missing.
pub fn meta_audience_to_domain(raw: &serde_json::Value) -> Result<Audience> {
    let id = extract_str(raw, "id")?;
    let name = extract_str(raw, "name")?;
    let description = raw["description"].as_str().map(String::from);
    let size = raw["approximate_count"].as_u64();
    let subtype = raw["subtype"].as_str().unwrap_or("CUSTOM");
    let audience_type = match subtype {
        "LOOKALIKE" => AudienceType::Lookalike,
        "CUSTOM" => AudienceType::Custom,
        other => AudienceType::Other(other.to_string()),
    };

    // Meta custom audiences don't always have a created_time; use now as fallback.
    let created_at = raw["time_created"]
        .as_str()
        .and_then(|s| s.parse::<DateTime<Utc>>().ok())
        .unwrap_or_else(Utc::now);

    Ok(Audience {
        id: AudienceId(id),
        provider: "meta".to_string(),
        name,
        description,
        size,
        audience_type,
        created_at,
        raw: Some(raw.clone()),
    })
}

/// Build the JSON body for a Meta create-audience request.
pub fn domain_to_meta_create_audience(input: &CreateAudienceInput) -> serde_json::Value {
    let mut body = serde_json::json!({
        "name": input.name,
        "subtype": "CUSTOM",
        "customer_file_source": "USER_PROVIDED_ONLY",
    });

    if let Some(desc) = &input.description {
        body["description"] = serde_json::Value::String(desc.clone());
    }

    if let Some(extra) = &input.extra {
        merge_json(&mut body, extra);
    }

    body
}

// ── Insights ──────────────────────────────────────────────

/// Convert a Meta insights API response into a domain [`InsightsReport`].
///
/// # Errors
///
/// Returns [`MktError::ApiError`] if the response structure is unexpected.
#[allow(clippy::unnecessary_wraps)] // Result kept for API consistency with other mapping fns
pub fn meta_insights_to_domain(resp: &serde_json::Value) -> Result<InsightsReport> {
    let rows: Vec<InsightsRow> = resp["data"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|entry| {
            let mut dimensions = HashMap::new();
            let mut metrics = HashMap::new();

            if let Some(obj) = entry.as_object() {
                for (key, val) in obj {
                    // Skip Meta metadata fields.
                    if key == "date_start" || key == "date_stop" {
                        dimensions.insert(key.clone(), val.as_str().unwrap_or("").to_string());
                        continue;
                    }
                    if key == "actions" || key == "action_values" {
                        // Flatten action arrays into prefixed metrics.
                        if let Some(actions) = val.as_array() {
                            for action in actions {
                                if let (Some(action_type), Some(action_val)) =
                                    (action["action_type"].as_str(), action["value"].as_str())
                                {
                                    let metric_name = format!("{key}.{action_type}");
                                    if let Ok(v) = action_val.parse::<f64>() {
                                        metrics.insert(
                                            metric_name,
                                            MetricValue {
                                                value: v,
                                                formatted: None,
                                            },
                                        );
                                    }
                                }
                            }
                        }
                        continue;
                    }
                    // Try to parse numeric values as metrics.
                    if let Some(s) = val.as_str() {
                        if let Ok(v) = s.parse::<f64>() {
                            metrics.insert(
                                key.clone(),
                                MetricValue {
                                    value: v,
                                    formatted: None,
                                },
                            );
                            continue;
                        }
                        // Non-numeric strings are dimensions (e.g. breakdowns).
                        dimensions.insert(key.clone(), s.to_string());
                    } else if let Some(v) = val.as_f64() {
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
        provider: "meta".to_string(),
        date_range: None,
        rows,
        raw: Some(resp.clone()),
    })
}

// ── Posts ──────────────────────────────────────────────────

/// Convert a Meta page post or IG media JSON object into a domain [`Post`].
///
/// # Errors
///
/// Returns [`MktError::ApiError`] if the `id` field is missing.
pub fn meta_post_to_domain(raw: &serde_json::Value, platform: &str) -> Result<Post> {
    let id = extract_str(raw, "id")?;
    let message = raw["message"].as_str().map(String::from);
    let link = raw["link"]
        .as_str()
        .or_else(|| raw["permalink_url"].as_str())
        .map(String::from);
    let permalink = raw["permalink_url"]
        .as_str()
        .or_else(|| raw["permalink"].as_str())
        .map(String::from);
    let image_url = raw["full_picture"]
        .as_str()
        .or_else(|| raw["media_url"].as_str())
        .map(String::from);

    let created_at = raw["created_time"]
        .as_str()
        .or_else(|| raw["timestamp"].as_str())
        .and_then(|s| s.parse::<DateTime<Utc>>().ok())
        .unwrap_or_else(Utc::now);

    Ok(Post {
        id: PostId(id),
        provider: "meta".to_string(),
        platform: platform.to_string(),
        message,
        link,
        image_url,
        permalink,
        created_at,
        raw: Some(raw.clone()),
    })
}

// ── Helpers ────────────────────────────────────────────────

/// Extract a required string field or return an API error.
fn extract_str(value: &serde_json::Value, field: &str) -> Result<String> {
    value[field]
        .as_str()
        .map(String::from)
        .ok_or_else(|| MktError::ApiError {
            provider: "meta".into(),
            status: 0,
            message: format!("missing field '{field}' in API response"),
            retry_after: None,
        })
}

/// Parse a required ISO 8601 datetime field.
fn parse_datetime(value: &serde_json::Value, field: &str) -> Result<DateTime<Utc>> {
    let s = extract_str(value, field)?;
    s.parse::<DateTime<Utc>>().map_err(|e| MktError::ApiError {
        provider: "meta".into(),
        status: 0,
        message: format!("invalid datetime in '{field}': {e}"),
        retry_after: None,
    })
}

/// Parse an optional ISO 8601 datetime field, returning `None` on
/// absence or parse failure.
fn parse_optional_datetime(value: &serde_json::Value, field: &str) -> Option<DateTime<Utc>> {
    value[field]
        .as_str()
        .and_then(|s| s.parse::<DateTime<Utc>>().ok())
}

/// Try to extract a budget from `daily_budget` or `lifetime_budget`.
fn parse_budget(raw: &serde_json::Value) -> Option<Budget> {
    if let Some(daily) = raw["daily_budget"].as_str() {
        if let Ok(amount) = daily.parse::<f64>() {
            return Some(Budget {
                amount,
                currency: "USD".to_string(),
                kind: BudgetKind::Daily,
            });
        }
    }
    if let Some(lifetime) = raw["lifetime_budget"].as_str() {
        if let Ok(amount) = lifetime.parse::<f64>() {
            return Some(Budget {
                amount,
                currency: "USD".to_string(),
                kind: BudgetKind::Lifetime,
            });
        }
    }
    None
}

/// Write budget fields into a JSON body.
fn apply_budget(body: &mut serde_json::Value, budget: Option<&Budget>) {
    if let Some(b) = budget {
        let key = match b.kind {
            BudgetKind::Daily => "daily_budget",
            BudgetKind::Lifetime => "lifetime_budget",
        };
        body[key] = serde_json::Value::String(b.amount.to_string());
    }
}

/// Shallow-merge `extra` into `body`.
fn merge_json(body: &mut serde_json::Value, extra: &serde_json::Value) {
    if let (Some(base), Some(overlay)) = (body.as_object_mut(), extra.as_object()) {
        for (k, v) in overlay {
            base.insert(k.clone(), v.clone());
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_meta_status_to_domain() {
        assert_eq!(meta_status_to_domain("ACTIVE"), CampaignStatus::Active);
        assert_eq!(meta_status_to_domain("PAUSED"), CampaignStatus::Paused);
        assert_eq!(meta_status_to_domain("ARCHIVED"), CampaignStatus::Archived);
        assert_eq!(meta_status_to_domain("DELETED"), CampaignStatus::Deleted);
        let other = meta_status_to_domain("IN_PROCESS");
        assert_eq!(other, CampaignStatus::Other("IN_PROCESS".into()));
    }

    #[test]
    fn test_domain_status_to_meta() {
        assert_eq!(domain_status_to_meta(&CampaignStatus::Active), "ACTIVE");
        assert_eq!(domain_status_to_meta(&CampaignStatus::Paused), "PAUSED");
        assert_eq!(domain_status_to_meta(&CampaignStatus::Archived), "ARCHIVED");
        assert_eq!(domain_status_to_meta(&CampaignStatus::Deleted), "DELETED");
        assert_eq!(domain_status_to_meta(&CampaignStatus::Draft), "PAUSED");
        assert_eq!(
            domain_status_to_meta(&CampaignStatus::Other("X".into())),
            "X"
        );
    }

    #[test]
    fn test_meta_campaign_to_domain() {
        let raw = serde_json::json!({
            "id": "120210123456789", "name": "Summer Sale 2026",
            "status": "ACTIVE", "objective": "OUTCOME_LEADS",
            "daily_budget": "5000",
            "created_time": "2026-03-01T12:00:00+0000",
            "updated_time": "2026-03-15T09:30:00+0000"
        });
        let c = meta_campaign_to_domain(&raw).expect("should parse");
        assert_eq!(c.id.0, "120210123456789");
        assert_eq!(c.provider, "meta");
        assert_eq!(c.name, "Summer Sale 2026");
        assert_eq!(c.status, CampaignStatus::Active);
        assert_eq!(c.objective, "OUTCOME_LEADS");
        let b = c.budget.as_ref().expect("should have budget");
        assert!((b.amount - 5000.0).abs() < f64::EPSILON);
        assert_eq!(b.kind, BudgetKind::Daily);
        assert!(c.updated_at.is_some());
        assert!(c.raw.is_some());
    }

    #[test]
    fn test_meta_campaign_to_domain_minimal() {
        let raw = serde_json::json!({
            "id": "999", "name": "Minimal", "created_time": "2026-01-01T00:00:00+0000"
        });
        let c = meta_campaign_to_domain(&raw).expect("should parse");
        assert_eq!(c.id.0, "999");
        assert_eq!(c.name, "Minimal");
        assert_eq!(c.status, CampaignStatus::Other("UNKNOWN".into()));
        assert!(c.objective.is_empty());
        assert!(c.budget.is_none());
        assert!(c.updated_at.is_none());
    }

    #[test]
    fn test_meta_campaign_to_domain_missing_id() {
        let raw = serde_json::json!({"name": "No ID", "created_time": "2026-01-01T00:00:00+0000"});
        assert!(meta_campaign_to_domain(&raw).is_err());
    }

    #[test]
    fn test_domain_to_meta_create_campaign() {
        let input = CreateCampaignInput {
            name: "Test".into(),
            objective: "OUTCOME_SALES".into(),
            status: Some(CampaignStatus::Paused),
            budget: Some(Budget {
                amount: 1000.0,
                currency: "USD".into(),
                kind: BudgetKind::Daily,
            }),
            extra: None,
        };
        let body = domain_to_meta_create_campaign(&input);
        assert_eq!(body["name"], "Test");
        assert_eq!(body["objective"], "OUTCOME_SALES");
        assert_eq!(body["status"], "PAUSED");
        assert_eq!(body["daily_budget"], "1000");
    }

    #[test]
    fn test_domain_to_meta_update_campaign() {
        let input = UpdateCampaignInput {
            name: Some("Renamed".into()),
            status: None,
            budget: None,
            extra: None,
        };
        let body = domain_to_meta_update_campaign(&input);
        assert_eq!(body["name"], "Renamed");
        assert!(body.get("status").is_none());
    }

    // ── Ad set mapping tests ───────────────────────────────

    #[test]
    fn test_meta_adset_status_to_domain() {
        assert_eq!(meta_adset_status_to_domain("ACTIVE"), AdSetStatus::Active);
        assert_eq!(meta_adset_status_to_domain("PAUSED"), AdSetStatus::Paused);
        assert_eq!(
            meta_adset_status_to_domain("ARCHIVED"),
            AdSetStatus::Archived
        );
        assert_eq!(meta_adset_status_to_domain("DELETED"), AdSetStatus::Deleted);
        assert_eq!(
            meta_adset_status_to_domain("IN_PROCESS"),
            AdSetStatus::Other("IN_PROCESS".into())
        );
    }

    #[test]
    fn test_domain_adset_status_to_meta() {
        assert_eq!(domain_adset_status_to_meta(&AdSetStatus::Active), "ACTIVE");
        assert_eq!(domain_adset_status_to_meta(&AdSetStatus::Paused), "PAUSED");
        assert_eq!(
            domain_adset_status_to_meta(&AdSetStatus::Archived),
            "ARCHIVED"
        );
        assert_eq!(
            domain_adset_status_to_meta(&AdSetStatus::Deleted),
            "DELETED"
        );
        assert_eq!(
            domain_adset_status_to_meta(&AdSetStatus::Other("X".into())),
            "X"
        );
    }

    #[test]
    fn test_meta_adset_to_domain() {
        let raw = serde_json::json!({
            "id": "23845600000000001",
            "campaign_id": "120330000000000001",
            "name": "Lookalike — Email List 1%",
            "status": "ACTIVE",
            "daily_budget": "2500",
            "targeting": { "age_min": 25, "geo_locations": { "countries": ["US"] } },
            "billing_event": "IMPRESSIONS",
            "optimization_goal": "LEAD_GENERATION",
            "created_time": "2026-01-15T10:35:00+0000",
            "updated_time": "2026-02-20T09:10:00+0000"
        });
        let adset = meta_adset_to_domain(&raw).expect("should parse");
        assert_eq!(adset.id.0, "23845600000000001");
        assert_eq!(adset.provider, "meta");
        assert_eq!(adset.campaign_id.0, "120330000000000001");
        assert_eq!(adset.status, AdSetStatus::Active);
        let targeting = adset.targeting.expect("targeting should be present");
        assert_eq!(targeting["age_min"], 25);
        let budget = adset.budget.expect("daily_budget should map");
        assert!((budget.amount - 2500.0).abs() < f64::EPSILON);
        assert_eq!(budget.kind, BudgetKind::Daily);
        assert!(adset.updated_at.is_some());
        assert!(adset.raw.is_some());
    }

    #[test]
    fn test_meta_adset_to_domain_minimal() {
        let raw = serde_json::json!({
            "id": "1", "campaign_id": "2", "name": "Minimal",
            "created_time": "2026-01-01T00:00:00+0000"
        });
        let adset = meta_adset_to_domain(&raw).expect("should parse");
        assert_eq!(adset.status, AdSetStatus::Other("UNKNOWN".into()));
        assert!(adset.targeting.is_none());
        assert!(adset.budget.is_none());
        assert!(adset.updated_at.is_none());
    }

    #[test]
    fn test_meta_adset_to_domain_null_targeting_is_none() {
        let raw = serde_json::json!({
            "id": "1", "campaign_id": "2", "name": "Null targeting",
            "targeting": null,
            "created_time": "2026-01-01T00:00:00+0000"
        });
        let adset = meta_adset_to_domain(&raw).expect("should parse");
        assert!(adset.targeting.is_none());
    }

    #[test]
    fn test_meta_adset_to_domain_missing_campaign_id() {
        let raw = serde_json::json!({
            "id": "1", "name": "No campaign",
            "created_time": "2026-01-01T00:00:00+0000"
        });
        assert!(meta_adset_to_domain(&raw).is_err());
    }

    #[test]
    fn test_domain_to_meta_create_adset() {
        let input = CreateAdSetInput {
            campaign_id: CampaignId::from("camp_1"),
            name: "New Ad Set".into(),
            status: Some(AdSetStatus::Paused),
            targeting: Some(serde_json::json!({ "geo_locations": { "countries": ["US"] } })),
            budget: Some(Budget {
                amount: 1500.0,
                currency: "USD".into(),
                kind: BudgetKind::Daily,
            }),
            extra: Some(serde_json::json!({
                "billing_event": "IMPRESSIONS",
                "optimization_goal": "LINK_CLICKS",
            })),
        };
        let body = domain_to_meta_create_adset(&input);
        assert_eq!(body["campaign_id"], "camp_1");
        assert_eq!(body["name"], "New Ad Set");
        assert_eq!(body["status"], "PAUSED");
        assert_eq!(body["daily_budget"], "1500");
        assert_eq!(body["targeting"]["geo_locations"]["countries"][0], "US");
        assert_eq!(body["billing_event"], "IMPRESSIONS");
        assert_eq!(body["optimization_goal"], "LINK_CLICKS");
    }

    #[test]
    fn test_domain_to_meta_create_adset_minimal() {
        let input = CreateAdSetInput {
            campaign_id: CampaignId::from("camp_2"),
            name: "Minimal".into(),
            status: None,
            targeting: None,
            budget: None,
            extra: None,
        };
        let body = domain_to_meta_create_adset(&input);
        assert_eq!(body["campaign_id"], "camp_2");
        assert!(body.get("status").is_none());
        assert!(body.get("targeting").is_none());
        assert!(body.get("daily_budget").is_none());
    }

    // ── Audience mapping tests ─────────────────────────────

    #[test]
    fn test_meta_audience_to_domain() {
        let raw = serde_json::json!({
            "id": "23842000001",
            "name": "Top Customers",
            "description": "Our best buyers",
            "approximate_count": 15000,
            "subtype": "CUSTOM"
        });
        let aud = meta_audience_to_domain(&raw).expect("should parse");
        assert_eq!(aud.id.0, "23842000001");
        assert_eq!(aud.name, "Top Customers");
        assert_eq!(aud.description.as_deref(), Some("Our best buyers"));
        assert_eq!(aud.size, Some(15000));
        assert_eq!(aud.audience_type, AudienceType::Custom);
        assert_eq!(aud.provider, "meta");
    }

    #[test]
    fn test_meta_audience_to_domain_lookalike() {
        let raw = serde_json::json!({
            "id": "23842000002",
            "name": "Lookalike US",
            "subtype": "LOOKALIKE"
        });
        let aud = meta_audience_to_domain(&raw).expect("should parse");
        assert_eq!(aud.audience_type, AudienceType::Lookalike);
    }

    #[test]
    fn test_meta_audiences_to_domain() {
        let resp = serde_json::json!({
            "data": [
                {"id": "1", "name": "Aud 1", "subtype": "CUSTOM"},
                {"id": "2", "name": "Aud 2", "subtype": "LOOKALIKE"}
            ]
        });
        let auds = meta_audiences_to_domain(&resp).expect("should parse");
        assert_eq!(auds.len(), 2);
        assert_eq!(auds[0].name, "Aud 1");
        assert_eq!(auds[1].audience_type, AudienceType::Lookalike);
    }

    #[test]
    fn test_meta_audience_missing_id() {
        let raw = serde_json::json!({"name": "No ID"});
        assert!(meta_audience_to_domain(&raw).is_err());
    }

    #[test]
    fn test_domain_to_meta_create_audience() {
        let input = CreateAudienceInput {
            name: "Newsletter Subscribers".into(),
            description: Some("From Mailchimp".into()),
            audience_type: AudienceType::Custom,
            extra: None,
        };
        let body = domain_to_meta_create_audience(&input);
        assert_eq!(body["name"], "Newsletter Subscribers");
        assert_eq!(body["description"], "From Mailchimp");
        assert_eq!(body["subtype"], "CUSTOM");
    }

    // ── Insights mapping tests ─────────────────────────────

    #[test]
    fn test_meta_insights_to_domain() {
        let resp = serde_json::json!({
            "data": [{
                "impressions": "1234",
                "clicks": "56",
                "spend": "78.90",
                "date_start": "2026-03-01",
                "date_stop": "2026-03-15"
            }]
        });
        let report = meta_insights_to_domain(&resp).expect("should parse");
        assert_eq!(report.provider, "meta");
        assert_eq!(report.rows.len(), 1);
        let row = &report.rows[0];
        assert!((row.metrics["impressions"].value - 1234.0).abs() < f64::EPSILON);
        assert!((row.metrics["clicks"].value - 56.0).abs() < f64::EPSILON);
        assert_eq!(
            row.dimensions.get("date_start").map(String::as_str),
            Some("2026-03-01")
        );
    }

    #[test]
    fn test_meta_insights_empty_data() {
        let resp = serde_json::json!({"data": []});
        let report = meta_insights_to_domain(&resp).expect("should parse");
        assert!(report.rows.is_empty());
    }

    // ── Post mapping tests ─────────────────────────────────

    #[test]
    fn test_meta_post_to_domain_facebook() {
        let raw = serde_json::json!({
            "id": "123456789_987654321",
            "message": "Hello world!",
            "permalink_url": "https://fb.com/123/posts/987",
            "created_time": "2026-03-20T10:00:00+0000"
        });
        let post = meta_post_to_domain(&raw, "facebook").expect("should parse");
        assert_eq!(post.id.0, "123456789_987654321");
        assert_eq!(post.platform, "facebook");
        assert_eq!(post.message.as_deref(), Some("Hello world!"));
        assert!(post.permalink.is_some());
    }

    #[test]
    fn test_meta_post_to_domain_instagram() {
        let raw = serde_json::json!({
            "id": "17895695668004550",
            "timestamp": "2026-03-20T10:00:00+0000",
            "permalink": "https://www.instagram.com/p/abc123/"
        });
        let post = meta_post_to_domain(&raw, "instagram").expect("should parse");
        assert_eq!(post.platform, "instagram");
        assert!(post.permalink.is_some());
    }

    #[test]
    fn test_meta_post_missing_id() {
        let raw = serde_json::json!({"message": "No ID"});
        assert!(meta_post_to_domain(&raw, "facebook").is_err());
    }
}
