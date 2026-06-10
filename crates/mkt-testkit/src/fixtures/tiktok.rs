//! Realistic TikTok Business API v1.3 JSON fixtures for unit and
//! integration tests.
//!
//! Responses mirror the v1.3 envelope: `{"code", "message", "request_id",
//! "data"}` with HTTP 200 even on logical errors (`code != 0`), numeric
//! IDs in responses, string metric values in reports, and `page_info`
//! pagination.

/// A successful `GET /campaign/get/` response with two campaigns.
pub fn campaigns_get_response() -> serde_json::Value {
    serde_json::json!({
        "code": 0,
        "message": "OK",
        "request_id": "2026061015295501023125104093250",
        "data": {
            "list": [
                {
                    "campaign_id": 1_680_018_437_245_954_i64,
                    "campaign_name": "Spark Ads — Spring Drop",
                    "advertiser_id": 7_106_541_027_904_733_185_i64,
                    "campaign_type": "REGULAR_CAMPAIGN",
                    "objective": "LANDING_PAGE",
                    "objective_type": "TRAFFIC",
                    "budget": 50.0,
                    "budget_mode": "BUDGET_MODE_DAY",
                    "operation_status": "ENABLE",
                    "secondary_status": "CAMPAIGN_STATUS_ENABLE",
                    "is_smart_performance_campaign": false,
                    "is_new_structure": true,
                    "create_time": "2026-01-13 13:44:30",
                    "modify_time": "2026-03-01 09:12:00"
                },
                {
                    "campaign_id": 1_680_018_437_245_955_i64,
                    "campaign_name": "Lead Gen — App Waitlist",
                    "advertiser_id": 7_106_541_027_904_733_185_i64,
                    "campaign_type": "REGULAR_CAMPAIGN",
                    "objective_type": "LEAD_GENERATION",
                    "budget": 1000.0,
                    "budget_mode": "BUDGET_MODE_TOTAL",
                    "operation_status": "DISABLE",
                    "secondary_status": "CAMPAIGN_STATUS_DISABLE",
                    "create_time": "2026-02-01 08:00:00",
                    "modify_time": "2026-02-20 10:30:00"
                }
            ],
            "page_info": {
                "page": 1,
                "page_size": 10,
                "total_number": 2,
                "total_page": 1
            }
        }
    })
}

/// A successful `POST /campaign/create/` response.
pub fn campaign_create_response() -> serde_json::Value {
    serde_json::json!({
        "code": 0,
        "message": "OK",
        "request_id": "2026061015295501023125104093251",
        "data": { "campaign_id": 1_740_687_531_023_393_i64 }
    })
}

/// A successful `POST /campaign/status/update/` response.
pub fn status_update_response() -> serde_json::Value {
    serde_json::json!({
        "code": 0,
        "message": "OK",
        "request_id": "2026061015295501023125104093252",
        "data": {
            "campaign_ids": ["1680018437245954"],
            "status": "DELETE"
        }
    })
}

/// A successful `GET /report/integrated/get/` response with two daily rows.
///
/// Metric values are strings; dimension IDs are numbers.
pub fn report_response() -> serde_json::Value {
    serde_json::json!({
        "code": 0,
        "message": "OK",
        "request_id": "2026061015295501023125104093253",
        "data": {
            "list": [
                {
                    "dimensions": {
                        "campaign_id": 1_680_018_437_245_954_i64,
                        "stat_time_day": "2026-03-01 00:00:00"
                    },
                    "metrics": {
                        "spend": "234.56",
                        "impressions": "45230",
                        "clicks": "1876",
                        "ctr": "4.15",
                        "cpc": "0.125"
                    }
                },
                {
                    "dimensions": {
                        "campaign_id": 1_680_018_437_245_954_i64,
                        "stat_time_day": "2026-03-02 00:00:00"
                    },
                    "metrics": {
                        "spend": "278.90",
                        "impressions": "52100",
                        "clicks": "2134",
                        "ctr": "4.10",
                        "cpc": "0.131"
                    }
                }
            ],
            "page_info": {
                "page": 1,
                "page_size": 200,
                "total_number": 2,
                "total_page": 1
            }
        }
    })
}

/// A successful `GET /dmp/custom_audience/list/` response.
pub fn audience_list_response() -> serde_json::Value {
    serde_json::json!({
        "code": 0,
        "message": "OK",
        "request_id": "2026061015295501023125104093254",
        "data": {
            "list": [
                {
                    "custom_audience_id": 102_400_000_001_i64,
                    "name": "Website Visitors 30d",
                    "audience_type": "Website Traffic",
                    "cover_num": 152_000,
                    "calculate_type": "MULTIPLE_TYPES",
                    "create_time": "2026-01-10 12:00:00",
                    "is_valid": true,
                    "shared": false
                },
                {
                    "custom_audience_id": 102_400_000_002_i64,
                    "name": "Customer Emails",
                    "audience_type": "Customer File",
                    "cover_num": 48_000,
                    "calculate_type": "EMAIL_SHA256",
                    "create_time": "2026-02-15 16:30:00",
                    "is_valid": true,
                    "shared": false
                }
            ],
            "page_info": {
                "page": 1,
                "page_size": 10,
                "total_number": 2,
                "total_page": 1
            }
        }
    })
}

/// An auth error: HTTP 200 with a non-zero envelope code (40105 = access
/// token incorrect or revoked).
pub fn auth_error_response() -> serde_json::Value {
    serde_json::json!({
        "code": 40105,
        "message": "Access token is incorrect or has been revoked.",
        "request_id": "2026061015295501023125104093255",
        "data": {}
    })
}

/// A rate-limit error: HTTP 200 with envelope code 40100.
pub fn rate_limit_response() -> serde_json::Value {
    serde_json::json!({
        "code": 40100,
        "message": "Due to too many requests, requests from App are temporarily restricted.",
        "request_id": "2026061015295501023125104093256",
        "data": {}
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn campaigns_get_has_envelope_and_list() {
        let v = campaigns_get_response();
        assert_eq!(v["code"], 0);
        assert_eq!(v["data"]["list"].as_array().expect("array").len(), 2);
        assert_eq!(v["data"]["page_info"]["total_number"], 2);
    }

    #[test]
    fn campaign_ids_are_numbers_in_responses() {
        let v = campaigns_get_response();
        assert!(v["data"]["list"][0]["campaign_id"].is_i64());
    }

    #[test]
    fn campaign_create_returns_campaign_id() {
        let v = campaign_create_response();
        assert!(v["data"]["campaign_id"].is_i64());
    }

    #[test]
    fn report_metric_values_are_strings() {
        let v = report_response();
        assert!(v["data"]["list"][0]["metrics"]["spend"].is_string());
        assert!(v["data"]["list"][0]["dimensions"]["campaign_id"].is_i64());
    }

    #[test]
    fn auth_error_keeps_http_semantics_in_code() {
        let v = auth_error_response();
        assert_eq!(v["code"], 40105);
    }

    #[test]
    fn rate_limit_uses_code_40100() {
        let v = rate_limit_response();
        assert_eq!(v["code"], 40100);
    }

    #[test]
    fn audience_list_has_cover_num_sizes() {
        let v = audience_list_response();
        assert!(v["data"]["list"][0]["cover_num"].is_i64());
    }
}
