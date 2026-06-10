//! Realistic LinkedIn Marketing API JSON fixtures for unit and integration
//! tests.
//!
//! Responses mirror the versioned REST API (`Linkedin-Version: 202605`):
//! Rest.li 2.0 conventions, money amounts as strings, epoch-millisecond
//! timestamps, and cursor pagination via `metadata.nextPageToken`.

/// A successful `GET /rest/adAccounts/{id}/adCampaigns?q=search` response
/// with two campaigns and a next-page cursor.
pub fn campaigns_search_response() -> serde_json::Value {
    serde_json::json!({
        "elements": [
            {
                "id": 145_282_384,
                "account": "urn:li:sponsoredAccount:506333826",
                "campaignGroup": "urn:li:sponsoredCampaignGroup:603030884",
                "name": "Lead Gen — DACH Q2",
                "type": "SPONSORED_UPDATES",
                "status": "ACTIVE",
                "objectiveType": "LEAD_GENERATION",
                "costType": "CPC",
                "dailyBudget": { "amount": "18", "currencyCode": "USD" },
                "unitCost": { "amount": "15", "currencyCode": "USD" },
                "locale": { "country": "US", "language": "en" },
                "creativeSelection": "OPTIMIZED",
                "offsiteDeliveryEnabled": false,
                "audienceExpansionEnabled": false,
                "runSchedule": { "start": 1_767_225_600_000_i64 },
                "changeAuditStamps": {
                    "created": { "time": 1_767_225_600_000_i64 },
                    "lastModified": { "time": 1_772_495_400_000_i64 }
                },
                "version": { "versionTag": "1" }
            },
            {
                "id": 145_282_385,
                "account": "urn:li:sponsoredAccount:506333826",
                "campaignGroup": "urn:li:sponsoredCampaignGroup:603030884",
                "name": "Brand Awareness — Global",
                "type": "SPONSORED_UPDATES",
                "status": "PAUSED",
                "objectiveType": "BRAND_AWARENESS",
                "costType": "CPM",
                "dailyBudget": { "amount": "50", "currencyCode": "USD" },
                "locale": { "country": "US", "language": "en" },
                "version": { "versionTag": "3" }
            }
        ],
        "metadata": { "nextPageToken": "DgGerr1iVQreCJVjZDOW" }
    })
}

/// A successful `GET /rest/adAccounts/{id}/adCampaigns/{campaignId}` response.
pub fn campaign_get_response() -> serde_json::Value {
    serde_json::json!({
        "id": 145_282_384,
        "account": "urn:li:sponsoredAccount:506333826",
        "campaignGroup": "urn:li:sponsoredCampaignGroup:603030884",
        "name": "Lead Gen — DACH Q2",
        "type": "SPONSORED_UPDATES",
        "status": "ACTIVE",
        "objectiveType": "LEAD_GENERATION",
        "costType": "CPC",
        "dailyBudget": { "amount": "18", "currencyCode": "USD" },
        "unitCost": { "amount": "15", "currencyCode": "USD" },
        "locale": { "country": "US", "language": "en" },
        "runSchedule": { "start": 1_767_225_600_000_i64 },
        "changeAuditStamps": {
            "created": { "time": 1_767_225_600_000_i64 },
            "lastModified": { "time": 1_772_495_400_000_i64 }
        }
    })
}

/// A successful `GET /rest/adAnalytics?q=analytics` response with two
/// daily rows for one campaign.
pub fn analytics_response() -> serde_json::Value {
    serde_json::json!({
        "elements": [
            {
                "impressions": 165,
                "clicks": 11,
                "costInLocalCurrency": "19.91833",
                "landingPageClicks": 8,
                "dateRange": {
                    "start": { "year": 2026, "month": 3, "day": 1 },
                    "end": { "year": 2026, "month": 3, "day": 1 }
                },
                "pivotValues": ["urn:li:sponsoredCampaign:145282384"]
            },
            {
                "impressions": 220,
                "clicks": 17,
                "costInLocalCurrency": "27.50",
                "landingPageClicks": 12,
                "dateRange": {
                    "start": { "year": 2026, "month": 3, "day": 2 },
                    "end": { "year": 2026, "month": 3, "day": 2 }
                },
                "pivotValues": ["urn:li:sponsoredCampaign:145282384"]
            }
        ],
        "paging": { "count": 10, "start": 0, "links": [] }
    })
}

/// A standard LinkedIn error response (e.g. invalid token).
pub fn api_error_response() -> serde_json::Value {
    serde_json::json!({
        "status": 401,
        "serviceErrorCode": 65600,
        "code": "REVOKED_ACCESS_TOKEN",
        "message": "The token used in the request has been revoked by the user"
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn campaigns_search_response_has_two_elements() {
        let v = campaigns_search_response();
        assert_eq!(v["elements"].as_array().expect("array").len(), 2);
    }

    #[test]
    fn campaigns_search_has_next_page_token() {
        let v = campaigns_search_response();
        assert!(v["metadata"]["nextPageToken"].is_string());
    }

    #[test]
    fn campaign_budget_amount_is_string() {
        let v = campaign_get_response();
        assert!(v["dailyBudget"]["amount"].is_string());
    }

    #[test]
    fn campaign_created_time_is_epoch_millis() {
        let v = campaign_get_response();
        assert!(v["changeAuditStamps"]["created"]["time"].is_i64());
    }

    #[test]
    fn analytics_cost_is_string() {
        let v = analytics_response();
        assert!(v["elements"][0]["costInLocalCurrency"].is_string());
        assert!(v["elements"][0]["impressions"].is_i64());
    }

    #[test]
    fn api_error_has_code_and_status() {
        let v = api_error_response();
        assert_eq!(v["status"], 401);
        assert!(v["code"].is_string());
    }
}
