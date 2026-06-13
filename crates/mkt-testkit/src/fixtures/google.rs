//! Realistic Google Ads API (REST) JSON fixtures for unit and integration
//! tests.
//!
//! Responses mirror the shape of the Google Ads API v24 REST interface:
//! camelCase field names, string-encoded int64 values (`amountMicros`),
//! and `results` arrays from `googleAds:search`.

/// A successful `POST customers/{cid}/googleAds:search` response for a
/// campaign list query, with two campaigns and their budgets.
pub fn campaigns_search_response() -> serde_json::Value {
    serde_json::json!({
        "results": [
            {
                "campaign": {
                    "resourceName": "customers/1234567890/campaigns/111111",
                    "id": "111111",
                    "name": "Search — Brand Terms",
                    "status": "ENABLED",
                    "advertisingChannelType": "SEARCH",
                    "startDate": "2026-01-15",
                    "endDate": "2037-12-30"
                },
                "campaignBudget": {
                    "resourceName": "customers/1234567890/campaignBudgets/999001",
                    "amountMicros": "5000000"
                }
            },
            {
                "campaign": {
                    "resourceName": "customers/1234567890/campaigns/222222",
                    "id": "222222",
                    "name": "PMax — Spring Sale",
                    "status": "PAUSED",
                    "advertisingChannelType": "PERFORMANCE_MAX",
                    "startDate": "2026-02-01"
                },
                "campaignBudget": {
                    "resourceName": "customers/1234567890/campaignBudgets/999002",
                    "amountMicros": "10000000"
                }
            }
        ],
        "fieldMask": "campaign.id,campaign.name,campaign.status",
        "nextPageToken": "CJ4Q"
    })
}

/// A successful `campaignBudgets:mutate` response.
pub fn budget_mutate_response() -> serde_json::Value {
    serde_json::json!({
        "results": [
            { "resourceName": "customers/1234567890/campaignBudgets/999003" }
        ]
    })
}

/// A successful `campaigns:mutate` response (create/update/remove).
pub fn campaign_mutate_response() -> serde_json::Value {
    serde_json::json!({
        "results": [
            { "resourceName": "customers/1234567890/campaigns/333333" }
        ]
    })
}

/// A successful atomic `googleAds:mutate` response for a combined
/// budget + campaign creation (`responseContentType: MUTABLE_RESOURCE`).
///
/// Operation responses arrive in the same order as the request
/// operations: budget first, then campaign.
pub fn atomic_mutate_response() -> serde_json::Value {
    serde_json::json!({
        "mutateOperationResponses": [
            {
                "campaignBudgetResult": {
                    "resourceName": "customers/1234567890/campaignBudgets/999003"
                }
            },
            {
                "campaignResult": {
                    "resourceName": "customers/1234567890/campaigns/333333"
                }
            }
        ]
    })
}

/// A `googleAds:search` response for an insights (metrics) query.
pub fn insights_search_response() -> serde_json::Value {
    serde_json::json!({
        "results": [
            {
                "campaign": { "resourceName": "customers/1234567890/campaigns/111111", "id": "111111", "name": "Search — Brand Terms" },
                "metrics": {
                    "impressions": "45230",
                    "clicks": "1876",
                    "costMicros": "234560000",
                    "ctr": 0.0415
                },
                "segments": { "date": "2026-03-01" }
            },
            {
                "campaign": { "resourceName": "customers/1234567890/campaigns/111111", "id": "111111", "name": "Search — Brand Terms" },
                "metrics": {
                    "impressions": "52100",
                    "clicks": "2134",
                    "costMicros": "278900000",
                    "ctr": 0.0410
                },
                "segments": { "date": "2026-03-02" }
            }
        ],
        "fieldMask": "metrics.impressions,metrics.clicks"
    })
}

/// A standard Google Ads API error response (HTTP 400/401/403).
pub fn api_error_response() -> serde_json::Value {
    serde_json::json!({
        "error": {
            "code": 401,
            "message": "Request had invalid authentication credentials.",
            "status": "UNAUTHENTICATED",
            "details": [
                {
                    "@type": "type.googleapis.com/google.ads.googleads.v24.errors.GoogleAdsFailure",
                    "errors": [
                        {
                            "errorCode": { "authenticationError": "OAUTH_TOKEN_INVALID" },
                            "message": "The OAuth access token is invalid."
                        }
                    ]
                }
            ]
        }
    })
}

/// A successful `OAuth2` token endpoint response for a refresh-token exchange.
pub fn oauth_token_response() -> serde_json::Value {
    serde_json::json!({
        "access_token": "ya29.test-access-token",
        "expires_in": 3599,
        "scope": "https://www.googleapis.com/auth/adwords",
        "token_type": "Bearer"
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn campaigns_search_response_has_two_results() {
        let v = campaigns_search_response();
        assert_eq!(v["results"].as_array().expect("array").len(), 2);
    }

    #[test]
    fn campaigns_search_amount_micros_is_string() {
        let v = campaigns_search_response();
        assert!(v["results"][0]["campaignBudget"]["amountMicros"].is_string());
    }

    #[test]
    fn campaign_mutate_response_has_resource_name() {
        let v = campaign_mutate_response();
        let name = v["results"][0]["resourceName"].as_str().expect("string");
        assert!(name.contains("/campaigns/"));
    }

    #[test]
    fn atomic_mutate_response_budget_result_precedes_campaign_result() {
        let v = atomic_mutate_response();
        let responses = v["mutateOperationResponses"].as_array().expect("array");
        assert_eq!(responses.len(), 2);
        assert!(
            responses[0]["campaignBudgetResult"]["resourceName"]
                .as_str()
                .expect("string")
                .contains("/campaignBudgets/")
        );
        assert!(
            responses[1]["campaignResult"]["resourceName"]
                .as_str()
                .expect("string")
                .contains("/campaigns/")
        );
    }

    #[test]
    fn api_error_response_has_status() {
        let v = api_error_response();
        assert_eq!(v["error"]["status"], "UNAUTHENTICATED");
        assert_eq!(v["error"]["code"], 401);
    }

    #[test]
    fn insights_search_response_metrics_are_present() {
        let v = insights_search_response();
        assert!(v["results"][0]["metrics"]["impressions"].is_string());
        assert!(v["results"][0]["segments"]["date"].is_string());
    }

    #[test]
    fn oauth_token_response_has_access_token() {
        let v = oauth_token_response();
        assert!(v["access_token"].is_string());
    }
}
