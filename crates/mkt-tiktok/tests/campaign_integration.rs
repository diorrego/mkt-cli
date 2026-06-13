#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]
//! Wiremock-based integration tests for the TikTok Business API v1.3
//! campaign, reporting, and audience endpoints.

use mkt_core::models::{
    Budget, BudgetKind, CampaignFilters, CampaignId, CampaignStatus, CreateCampaignInput,
    InsightsQuery, UpdateCampaignInput,
};
use mkt_core::provider::MarketingProvider;
use mkt_testkit::fixtures::tiktok as fixtures;
use mkt_tiktok::{TikTokClient, TikTokProvider};
use secrecy::SecretString;
use wiremock::matchers::{body_partial_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── Shared helpers ────────────────────────────────────────────────────────────

const ADVERTISER_ID: &str = "7106541027904733185";
const ACCESS_TOKEN: &str = "tt-test-token";

/// Build a [`TikTokProvider`] that sends requests to `base_url`.
fn make_provider(base_url: &str) -> TikTokProvider {
    let client = TikTokClient::new_with_base_url(
        SecretString::new(ACCESS_TOKEN.to_string().into()),
        ADVERTISER_ID.to_string(),
        format!("{base_url}/"),
    )
    .expect("client should build");
    TikTokProvider::new(client)
}

// ── list_campaigns ────────────────────────────────────────────────────────────

/// Verify that `list_campaigns` sends the Access-Token header (not Bearer)
/// to `/campaign/get/` and maps the envelope's `data.list`.
#[tokio::test]
async fn test_list_campaigns_uses_access_token_header() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/campaign/get/"))
        .and(header("Access-Token", ACCESS_TOKEN))
        .and(query_param("advertiser_id", ADVERTISER_ID))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::campaigns_get_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let page = provider
        .list_campaigns(&CampaignFilters::default())
        .await
        .expect("list_campaigns should succeed");

    assert_eq!(page.data.len(), 2);

    let first = &page.data[0];
    assert_eq!(first.id.0, "1680018437245954");
    assert_eq!(first.provider, "tiktok");
    assert_eq!(first.name, "Spark Ads — Spring Drop");
    assert_eq!(first.status, CampaignStatus::Active);
    assert_eq!(first.objective, "TRAFFIC");
    let budget = first.budget.as_ref().expect("budget should map");
    assert!((budget.amount - 50.0).abs() < f64::EPSILON);
    assert_eq!(budget.kind, BudgetKind::Daily);
    assert_eq!(
        first.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        "2026-01-13 13:44:30"
    );

    let second = &page.data[1];
    assert_eq!(second.status, CampaignStatus::Paused);
    let second_budget = second.budget.as_ref().expect("budget should map");
    assert_eq!(second_budget.kind, BudgetKind::Lifetime);
}

/// Verify a status filter is sent as a JSON-encoded `filtering` param.
#[tokio::test]
async fn test_list_campaigns_status_filter_is_json_encoded() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/campaign/get/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": 0, "message": "OK", "request_id": "x",
            "data": { "list": [], "page_info": { "page": 1, "page_size": 10, "total_number": 0, "total_page": 0 } }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let filters = CampaignFilters {
        status: Some(CampaignStatus::Active),
        ..Default::default()
    };
    provider
        .list_campaigns(&filters)
        .await
        .expect("filtered list should succeed");

    let requests = server.received_requests().await.unwrap();
    let url = &requests[0].url;
    let filtering = url
        .query_pairs()
        .find(|(k, _)| k == "filtering")
        .map(|(_, v)| v.into_owned())
        .expect("filtering param should be present");
    let parsed: serde_json::Value =
        serde_json::from_str(&filtering).expect("filtering must be valid JSON");
    assert_eq!(parsed["secondary_status"], "CAMPAIGN_STATUS_ENABLE");
}

/// Verify the advertiser account currency from `GET /advertiser/info/`
/// flows into campaign budgets, and that the lookup happens only once per
/// provider instance (the `expect(1)` fails if a second list re-queries).
#[tokio::test]
async fn test_list_campaigns_uses_account_currency() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/advertiser/info/"))
        .and(header("Access-Token", ACCESS_TOKEN))
        .and(query_param(
            "advertiser_ids",
            format!("[\"{ADVERTISER_ID}\"]"),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": 0, "message": "OK", "request_id": "x",
            "data": {
                "list": [{
                    "advertiser_id": ADVERTISER_ID,
                    "name": "Acme EU",
                    "currency": "EUR",
                    "timezone": "Europe/Madrid",
                    "status": "STATUS_ENABLE"
                }]
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/campaign/get/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::campaigns_get_response()))
        .expect(2)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let page = provider
        .list_campaigns(&CampaignFilters::default())
        .await
        .expect("list_campaigns should succeed");
    let budget = page.data[0].budget.as_ref().expect("budget should map");
    assert_eq!(
        budget.currency, "EUR",
        "budget must use the account currency"
    );

    // Second call on the same provider must reuse the cached currency
    // (advertiser/info/ is expect(1) above).
    let page2 = provider
        .list_campaigns(&CampaignFilters::default())
        .await
        .expect("second list_campaigns should succeed");
    let budget2 = page2.data[0].budget.as_ref().expect("budget should map");
    assert_eq!(budget2.currency, "EUR");
}

/// If `GET /advertiser/info/` fails, campaign listings must still work,
/// falling back to USD instead of propagating the metadata error.
#[tokio::test]
async fn test_list_campaigns_currency_falls_back_to_usd_on_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/advertiser/info/"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/campaign/get/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::campaigns_get_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let page = provider
        .list_campaigns(&CampaignFilters::default())
        .await
        .expect("list_campaigns must survive an advertiser/info/ failure");
    let budget = page.data[0].budget.as_ref().expect("budget should map");
    assert_eq!(budget.currency, "USD", "currency must fall back to USD");
}

/// `filters.limit` above TikTok's documented maximum must be clamped to
/// `page_size=1000`, otherwise `/campaign/get/` rejects the request.
#[tokio::test]
async fn test_list_campaigns_clamps_page_size_to_max() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/campaign/get/"))
        .and(query_param("page_size", "1000"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": 0, "message": "OK", "request_id": "x",
            "data": { "list": [], "page_info": { "page": 1, "page_size": 1000, "total_number": 0, "total_page": 0 } }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let filters = CampaignFilters {
        limit: Some(5000),
        ..Default::default()
    };
    provider
        .list_campaigns(&filters)
        .await
        .expect("clamped list should succeed");
}

// ── get_campaign ──────────────────────────────────────────────────────────────

/// Verify `get_campaign` filters by campaign ID and returns the first match.
#[tokio::test]
async fn test_get_campaign_filters_by_id() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/campaign/get/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::campaigns_get_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let campaign = provider
        .get_campaign(&CampaignId("1680018437245954".into()))
        .await
        .expect("get_campaign should succeed");

    assert_eq!(campaign.id.0, "1680018437245954");

    let requests = server.received_requests().await.unwrap();
    let filtering = requests[0]
        .url
        .query_pairs()
        .find(|(k, _)| k == "filtering")
        .map(|(_, v)| v.into_owned())
        .expect("filtering param");
    let parsed: serde_json::Value = serde_json::from_str(&filtering).unwrap();
    assert_eq!(parsed["campaign_ids"][0], "1680018437245954");
}

/// Verify an empty list for a get-by-id maps to not-found (exit code 4).
#[tokio::test]
async fn test_get_campaign_empty_is_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/campaign/get/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": 0, "message": "OK", "request_id": "x",
            "data": { "list": [], "page_info": { "page": 1, "page_size": 10, "total_number": 0, "total_page": 0 } }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let err = provider
        .get_campaign(&CampaignId("404".into()))
        .await
        .expect_err("missing campaign should error");
    assert_eq!(err.exit_code(), 4, "got: {err}");
}

// ── create_campaign ───────────────────────────────────────────────────────────

/// Verify `create_campaign` POSTs the body and re-fetches the campaign.
/// New campaigns are created DISABLE (paused) unless told otherwise.
#[tokio::test]
async fn test_create_campaign_posts_then_fetches() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/campaign/create/"))
        .and(body_partial_json(serde_json::json!({
            "advertiser_id": ADVERTISER_ID,
            "campaign_name": "Spark Ads — Spring Drop",
            "objective_type": "TRAFFIC",
            "budget_mode": "BUDGET_MODE_DAY",
            "budget": 50.0,
            "operation_status": "DISABLE",
        })))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(fixtures::campaign_create_response()),
        )
        .expect(1)
        .mount(&server)
        .await;

    // Re-fetch of the created campaign.
    Mock::given(method("GET"))
        .and(path("/campaign/get/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": 0, "message": "OK", "request_id": "x",
            "data": {
                "list": [{
                    "campaign_id": 1_740_687_531_023_393_i64,
                    "campaign_name": "Spark Ads — Spring Drop",
                    "objective_type": "TRAFFIC",
                    "budget": 50.0,
                    "budget_mode": "BUDGET_MODE_DAY",
                    "operation_status": "DISABLE",
                    "create_time": "2026-06-10 12:00:00",
                    "modify_time": "2026-06-10 12:00:00"
                }],
                "page_info": { "page": 1, "page_size": 10, "total_number": 1, "total_page": 1 }
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let input = CreateCampaignInput {
        name: "Spark Ads — Spring Drop".into(),
        objective: "TRAFFIC".into(),
        status: None,
        budget: Some(Budget {
            amount: 50.0,
            currency: "USD".into(),
            kind: BudgetKind::Daily,
        }),
        extra: None,
    };

    let campaign = provider
        .create_campaign(&input)
        .await
        .expect("create_campaign should succeed");
    assert_eq!(campaign.id.0, "1740687531023393");

    // The create body must carry a request_id so network-level retries
    // deduplicate instead of creating a second campaign.
    let requests = server.received_requests().await.unwrap();
    let create_body = requests
        .iter()
        .find(|r| r.url.path().ends_with("/campaign/create/"))
        .map(|r| serde_json::from_slice::<serde_json::Value>(&r.body).unwrap())
        .expect("create request should exist");
    let request_id = create_body["request_id"].as_str().unwrap_or_default();
    assert!(
        !request_id.is_empty(),
        "create must send an idempotency request_id: {create_body}"
    );
    assert_eq!(campaign.status, CampaignStatus::Paused);
}

// ── update / delete ───────────────────────────────────────────────────────────

/// Verify `update_campaign` posts name/budget changes to /campaign/update/
/// and a status change to /campaign/status/update/.
#[tokio::test]
async fn test_update_campaign_routes_status_separately() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/campaign/update/"))
        .and(body_partial_json(serde_json::json!({
            "advertiser_id": ADVERTISER_ID,
            "campaign_id": "1680018437245954",
            "campaign_name": "Renamed",
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": 0, "message": "OK", "request_id": "x",
            "data": { "campaign_id": 1_680_018_437_245_954_i64 }
        })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/campaign/status/update/"))
        .and(body_partial_json(serde_json::json!({
            "advertiser_id": ADVERTISER_ID,
            "campaign_ids": ["1680018437245954"],
            "operation_status": "DISABLE",
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::status_update_response()))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/campaign/get/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::campaigns_get_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let input = UpdateCampaignInput {
        name: Some("Renamed".into()),
        status: Some(CampaignStatus::Paused),
        budget: None,
        extra: None,
    };

    provider
        .update_campaign(&CampaignId("1680018437245954".into()), &input)
        .await
        .expect("update_campaign should succeed");
}

/// Verify `delete_campaign` uses `operation_status` DELETE.
#[tokio::test]
async fn test_delete_campaign_sends_delete_status() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/campaign/status/update/"))
        .and(body_partial_json(serde_json::json!({
            "advertiser_id": ADVERTISER_ID,
            "campaign_ids": ["1680018437245954"],
            "operation_status": "DELETE",
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::status_update_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    provider
        .delete_campaign(&CampaignId("1680018437245954".into()))
        .await
        .expect("delete_campaign should succeed");
}

// ── insights ──────────────────────────────────────────────────────────────────

/// Verify `get_insights` queries the integrated report with JSON-encoded
/// dimensions/metrics and maps string metric values to numbers.
#[tokio::test]
async fn test_get_insights_maps_report_rows() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/report/integrated/get/"))
        .and(query_param("report_type", "BASIC"))
        .and(query_param("data_level", "AUCTION_CAMPAIGN"))
        .and(query_param("start_date", "2026-03-01"))
        .and(query_param("end_date", "2026-03-02"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::report_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let query = InsightsQuery {
        date_range: Some(mkt_core::models::DateRange {
            start: chrono::DateTime::parse_from_rfc3339("2026-03-01T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            end: chrono::DateTime::parse_from_rfc3339("2026-03-02T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        }),
        ..Default::default()
    };
    let report = provider
        .get_insights(&query)
        .await
        .expect("get_insights should succeed");

    assert_eq!(report.provider, "tiktok");
    assert_eq!(report.rows.len(), 2);

    let row = &report.rows[0];
    assert!((row.metrics["spend"].value - 234.56).abs() < 1e-9);
    assert!((row.metrics["impressions"].value - 45230.0).abs() < f64::EPSILON);
    assert_eq!(
        row.dimensions.get("date").map(String::as_str),
        Some("2026-03-01"),
        "stat_time_day should map to a date dimension"
    );
    assert_eq!(
        row.dimensions.get("campaign_id").map(String::as_str),
        Some("1680018437245954")
    );

    // dimensions/metrics params must be JSON arrays.
    let requests = server.received_requests().await.unwrap();
    let dims = requests[0]
        .url
        .query_pairs()
        .find(|(k, _)| k == "dimensions")
        .map(|(_, v)| v.into_owned())
        .expect("dimensions param");
    let parsed: serde_json::Value = serde_json::from_str(&dims).unwrap();
    assert!(
        parsed
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d == "campaign_id")
    );
}

/// Lifetime metrics do not support time dimensions: a query without a
/// date range must send `query_lifetime=true` WITHOUT `stat_time_day`,
/// or the API rejects the combination.
#[tokio::test]
async fn test_get_insights_lifetime_omits_time_dimension() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/report/integrated/get/"))
        .and(query_param("query_lifetime", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": 0, "message": "OK", "request_id": "x",
            "data": {
                "list": [{
                    "dimensions": { "campaign_id": "1680018437245954" },
                    "metrics": { "spend": "1234.50", "impressions": "98765" }
                }],
                "page_info": { "page": 1, "page_size": 200, "total_number": 1, "total_page": 1 }
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let report = provider
        .get_insights(&InsightsQuery::default())
        .await
        .expect("lifetime insights should succeed");
    assert_eq!(report.rows.len(), 1);

    let requests = server.received_requests().await.unwrap();
    let dims = requests[0]
        .url
        .query_pairs()
        .find(|(k, _)| k == "dimensions")
        .map(|(_, v)| v.into_owned())
        .expect("dimensions param");
    let parsed: serde_json::Value = serde_json::from_str(&dims).unwrap();
    let dims_array = parsed.as_array().unwrap();
    assert!(
        !dims_array.iter().any(|d| d == "stat_time_day"),
        "lifetime queries must not request time dimensions: {parsed}"
    );
    assert!(dims_array.iter().any(|d| d == "campaign_id"));
}

/// Reports above one page must aggregate all pages (`total_page` from
/// `page_info`) instead of silently truncating.
#[tokio::test]
async fn test_get_insights_aggregates_report_pages() {
    let server = MockServer::start().await;

    let row = |id: &str| {
        serde_json::json!({
            "dimensions": { "campaign_id": id },
            "metrics": { "spend": "10.0", "impressions": "100" }
        })
    };
    let page = |rows: serde_json::Value, page: u32| {
        serde_json::json!({
            "code": 0, "message": "OK", "request_id": "x",
            "data": {
                "list": rows,
                "page_info": { "page": page, "page_size": 1000, "total_number": 2, "total_page": 2 }
            }
        })
    };

    Mock::given(method("GET"))
        .and(path("/report/integrated/get/"))
        .and(query_param("page", "1"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(page(serde_json::json!([row("1")]), 1)),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/report/integrated/get/"))
        .and(query_param("page", "2"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(page(serde_json::json!([row("2")]), 2)),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let report = provider
        .get_insights(&InsightsQuery::default())
        .await
        .expect("paged report should succeed");
    assert_eq!(report.rows.len(), 2, "both report pages must aggregate");
}

/// Accounts with more than 100 audiences (the per-page maximum) must see
/// every audience, not just the first page.
#[tokio::test]
async fn test_list_audiences_aggregates_pages() {
    let server = MockServer::start().await;

    let aud = |id: u64, name: &str| serde_json::json!({ "custom_audience_id": id, "name": name, "cover_num": 10 });
    Mock::given(method("GET"))
        .and(path("/dmp/custom_audience/list/"))
        .and(query_param("page", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": 0, "message": "OK", "request_id": "x",
            "data": {
                "list": [aud(1, "A")],
                "page_info": { "page": 1, "page_size": 100, "total_number": 2, "total_page": 2 }
            }
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/dmp/custom_audience/list/"))
        .and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": 0, "message": "OK", "request_id": "x",
            "data": {
                "list": [aud(2, "B")],
                "page_info": { "page": 2, "page_size": 100, "total_number": 2, "total_page": 2 }
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let audiences = provider
        .list_audiences()
        .await
        .expect("paged audiences should succeed");
    assert_eq!(audiences.len(), 2, "both audience pages must aggregate");
}

// ── audiences ─────────────────────────────────────────────────────────────────

/// Verify `list_audiences` maps DMP custom audiences.
#[tokio::test]
async fn test_list_audiences_maps_dmp_list() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/dmp/custom_audience/list/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::audience_list_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let audiences = provider
        .list_audiences()
        .await
        .expect("list_audiences should succeed");

    assert_eq!(audiences.len(), 2);
    assert_eq!(audiences[0].id.0, "102400000001");
    assert_eq!(audiences[0].name, "Website Visitors 30d");
    assert_eq!(audiences[0].size, Some(152_000));
    assert_eq!(audiences[0].provider, "tiktok");
}

// ── error envelope ────────────────────────────────────────────────────────────

/// Verify HTTP 200 + code 40105 maps to an auth error (exit code 3).
#[tokio::test]
async fn test_envelope_auth_error_maps_to_auth() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/campaign/get/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::auth_error_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let err = provider
        .list_campaigns(&CampaignFilters::default())
        .await
        .expect_err("code 40105 should be an error despite HTTP 200");
    assert_eq!(err.exit_code(), 3, "got: {err}");
}

/// Verify HTTP 200 + code 40100 maps to a rate-limit error (exit code 5,
/// transient).
#[tokio::test]
async fn test_envelope_rate_limit_maps_to_transient() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/campaign/get/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::rate_limit_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let err = provider
        .list_campaigns(&CampaignFilters::default())
        .await
        .expect_err("code 40100 should be an error despite HTTP 200");
    assert_eq!(err.exit_code(), 5, "got: {err}");
    assert!(err.is_transient(), "rate limits must be flagged transient");
}

/// Verify `health_check` reports healthy on a working endpoint.
#[tokio::test]
async fn test_health_check_reports_healthy() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/campaign/get/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": 0, "message": "OK", "request_id": "x",
            "data": { "list": [], "page_info": { "page": 1, "page_size": 1, "total_number": 0, "total_page": 0 } }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let health = provider
        .health_check()
        .await
        .expect("health_check should succeed");
    assert!(health.healthy);
    assert_eq!(health.provider, "tiktok");
}
