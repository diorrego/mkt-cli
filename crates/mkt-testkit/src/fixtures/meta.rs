//! Realistic Meta Graph API JSON fixtures for use in unit and integration tests.
//!
//! All responses mirror the actual shape returned by the Meta Marketing API
//! v24.0. Field names, status strings, and date formats follow the official
//! API documentation so that deserialization logic can be tested against
//! production-equivalent data.

/// A successful `GET /act_{ad_account_id}/campaigns` response with two campaigns.
///
/// Includes pagination cursors and a `next` link as the real API provides.
pub fn campaigns_response() -> serde_json::Value {
    serde_json::json!({
        "data": [
            {
                "id": "120330000000000001",
                "name": "Summer Sale 2026",
                "status": "ACTIVE",
                "objective": "OUTCOME_LEADS",
                "daily_budget": "5000",
                "created_time": "2026-01-15T10:30:00+0000",
                "updated_time": "2026-03-01T14:22:00+0000"
            },
            {
                "id": "120330000000000002",
                "name": "Brand Awareness Q1",
                "status": "PAUSED",
                "objective": "OUTCOME_AWARENESS",
                "lifetime_budget": "100000",
                "created_time": "2026-02-01T08:00:00+0000",
                "updated_time": null
            }
        ],
        "paging": {
            "cursors": {
                "before": "QVFIUjJNVkJOSU",
                "after": "QVFIUkx5NW5k"
            },
            "next": "https://graph.facebook.com/v24.0/act_123/campaigns?after=QVFIUkx5NW5k"
        }
    })
}

/// A successful `POST /act_{ad_account_id}/campaigns` response confirming creation.
///
/// The Meta API returns only `id` and `success` on creation; subsequent reads
/// fetch the full campaign object.
pub fn campaign_create_response() -> serde_json::Value {
    serde_json::json!({
        "id": "120330000000000099",
        "success": true
    })
}

/// A standard Meta Graph API error response (e.g. invalid access token).
///
/// Matches the `{"error": {...}}` envelope that the Meta API wraps all
/// error payloads in, including the `error_subcode` and `fbtrace_id` fields
/// used for support escalation.
pub fn api_error_response() -> serde_json::Value {
    serde_json::json!({
        "error": {
            "message": "Invalid OAuth access token - Cannot parse access token",
            "type": "OAuthException",
            "code": 190,
            "error_subcode": 460,
            "error_user_title": "Invalid access token",
            "error_user_msg": "Your access token has expired. Please re-authenticate.",
            "fbtrace_id": "A1b2C3d4E5f6G7h8"
        }
    })
}

/// A successful `GET /{campaign_id}/insights` response with performance metrics.
///
/// Includes impressions, clicks, spend, CPM, and CPC broken down by a
/// single day, mirroring the shape returned by the Meta Insights API.
pub fn insights_response() -> serde_json::Value {
    serde_json::json!({
        "data": [
            {
                "campaign_id": "120330000000000001",
                "campaign_name": "Summer Sale 2026",
                "impressions": "45230",
                "clicks": "1876",
                "spend": "234.56",
                "cpm": "5.19",
                "cpc": "0.125",
                "ctr": "4.15",
                "reach": "38200",
                "frequency": "1.18",
                "date_start": "2026-03-01",
                "date_stop": "2026-03-01"
            },
            {
                "campaign_id": "120330000000000001",
                "campaign_name": "Summer Sale 2026",
                "impressions": "52100",
                "clicks": "2134",
                "spend": "278.90",
                "cpm": "5.35",
                "cpc": "0.131",
                "ctr": "4.10",
                "reach": "43800",
                "frequency": "1.19",
                "date_start": "2026-03-02",
                "date_stop": "2026-03-02"
            }
        ],
        "paging": {
            "cursors": {
                "before": "QVFIUjJNaW5zaWdo",
                "after": "QVFIUkx5aW5zaWdo"
            }
        }
    })
}

/// A successful `POST /{page_id}/feed` response for an organic page post.
///
/// Returns the composite `{page_id}_{post_id}` identifier that the Meta
/// Pages API uses, along with the optional `story_fbid` used for Story posts.
pub fn page_post_response() -> serde_json::Value {
    serde_json::json!({
        "id": "123456789_987654321",
        "post_id": "987654321",
        "story_fbid": null
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_campaigns_response_has_data_array() {
        let v = campaigns_response();
        let data = v["data"].as_array().expect("data should be an array");
        assert_eq!(data.len(), 2);
    }

    #[test]
    fn test_campaigns_response_first_campaign_is_active() {
        let v = campaigns_response();
        assert_eq!(v["data"][0]["status"], "ACTIVE");
    }

    #[test]
    fn test_campaigns_response_second_campaign_is_paused() {
        let v = campaigns_response();
        assert_eq!(v["data"][1]["status"], "PAUSED");
    }

    #[test]
    fn test_campaigns_response_has_pagination_cursors() {
        let v = campaigns_response();
        assert!(v["paging"]["cursors"]["before"].is_string());
        assert!(v["paging"]["cursors"]["after"].is_string());
    }

    #[test]
    fn test_campaigns_response_paging_next_is_string() {
        let v = campaigns_response();
        assert!(v["paging"]["next"].is_string());
    }

    #[test]
    fn test_campaign_create_response_has_id() {
        let v = campaign_create_response();
        assert!(v["id"].is_string());
        assert_eq!(v["success"], true);
    }

    #[test]
    fn test_api_error_response_has_error_envelope() {
        let v = api_error_response();
        assert!(v["error"].is_object());
        assert!(v["error"]["code"].is_number());
        assert!(v["error"]["type"].is_string());
        assert!(v["error"]["fbtrace_id"].is_string());
    }

    #[test]
    fn test_api_error_response_code_is_190() {
        let v = api_error_response();
        assert_eq!(v["error"]["code"], 190);
    }

    #[test]
    fn test_insights_response_has_data_rows() {
        let v = insights_response();
        let data = v["data"].as_array().expect("data should be an array");
        assert_eq!(data.len(), 2);
    }

    #[test]
    fn test_insights_response_rows_have_required_metrics() {
        let v = insights_response();
        let row = &v["data"][0];
        assert!(row["impressions"].is_string());
        assert!(row["clicks"].is_string());
        assert!(row["spend"].is_string());
        assert!(row["cpm"].is_string());
        assert!(row["cpc"].is_string());
    }

    #[test]
    fn test_insights_response_rows_have_date_range() {
        let v = insights_response();
        let row = &v["data"][0];
        assert!(row["date_start"].is_string());
        assert!(row["date_stop"].is_string());
    }

    #[test]
    fn test_page_post_response_has_composite_id() {
        let v = page_post_response();
        let id = v["id"].as_str().expect("id should be a string");
        // Meta page post IDs are in the format {page_id}_{post_id}
        assert!(
            id.contains('_'),
            "post id should be composite (page_id_post_id)"
        );
    }

    #[test]
    fn test_page_post_response_has_post_id() {
        let v = page_post_response();
        assert!(v["post_id"].is_string());
    }
}
