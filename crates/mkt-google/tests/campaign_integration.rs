#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]
//! Wiremock-based integration tests for the Google Ads campaign API.
//!
//! Each test spins up a local mock server, constructs a [`GoogleProvider`]
//! pointing at it, exercises a provider method, and asserts the response.

use mkt_core::models::{
    Budget, BudgetKind, CampaignFilters, CampaignId, CampaignStatus, CreateCampaignInput,
    InsightsQuery, UpdateCampaignInput,
};
use mkt_core::provider::MarketingProvider;
use mkt_google::{GoogleClient, GoogleProvider};
use mkt_testkit::fixtures::google as fixtures;
use secrecy::{ExposeSecret, SecretString};
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── Shared helpers ────────────────────────────────────────────────────────────

const CUSTOMER_ID: &str = "1234567890";
const DEVELOPER_TOKEN: &str = "dev-token-22chars";
const ACCESS_TOKEN: &str = "ya29.test-access-token";

/// Build a [`GoogleProvider`] that sends requests to `base_url`.
fn make_provider(base_url: &str) -> GoogleProvider {
    let client = GoogleClient::new_with_base_url(
        SecretString::new(ACCESS_TOKEN.to_string().into()),
        DEVELOPER_TOKEN.to_string(),
        CUSTOMER_ID,
        None,
        format!("{base_url}/"),
    )
    .expect("client should build");
    GoogleProvider::new(client)
}

// ── list_campaigns ────────────────────────────────────────────────────────────

/// Verify that `list_campaigns` issues a GAQL search with the required
/// auth headers and maps results into domain campaigns.
#[tokio::test]
async fn test_list_campaigns_sends_gaql_search() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/customers/{CUSTOMER_ID}/googleAds:search")))
        .and(header("authorization", format!("Bearer {ACCESS_TOKEN}")))
        .and(header("developer-token", DEVELOPER_TOKEN))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(fixtures::campaigns_search_response()),
        )
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
    assert_eq!(first.id.0, "111111");
    assert_eq!(first.provider, "google");
    assert_eq!(first.name, "Search — Brand Terms");
    assert_eq!(first.status, CampaignStatus::Active);
    assert_eq!(first.objective, "SEARCH");
    let budget = first.budget.as_ref().expect("budget should map");
    assert!(
        (budget.amount - 5.0).abs() < f64::EPSILON,
        "5000000 micros should be 5.0 units, got {}",
        budget.amount
    );

    let second = &page.data[1];
    assert_eq!(second.status, CampaignStatus::Paused);
    assert_eq!(second.objective, "PERFORMANCE_MAX");

    assert_eq!(page.next_cursor.as_deref(), Some("CJ4Q"));
}

/// Verify that a status filter lands in the GAQL WHERE clause.
#[tokio::test]
async fn test_list_campaigns_status_filter_in_gaql() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/customers/{CUSTOMER_ID}/googleAds:search")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": []
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
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    let query = body["query"].as_str().unwrap();
    assert!(
        query.contains("campaign.status = 'ENABLED'"),
        "GAQL should filter by status, got: {query}"
    );
}

// ── get_campaign ──────────────────────────────────────────────────────────────

/// Verify that `get_campaign` queries by ID and returns a single campaign.
#[tokio::test]
async fn test_get_campaign_queries_by_id() {
    let server = MockServer::start().await;

    let single = serde_json::json!({
        "results": [fixtures::campaigns_search_response()["results"][0]]
    });

    Mock::given(method("POST"))
        .and(path(format!("/customers/{CUSTOMER_ID}/googleAds:search")))
        .respond_with(ResponseTemplate::new(200).set_body_json(&single))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let campaign = provider
        .get_campaign(&CampaignId("111111".into()))
        .await
        .expect("get_campaign should succeed");

    assert_eq!(campaign.id.0, "111111");

    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert!(
        body["query"]
            .as_str()
            .unwrap()
            .contains("campaign.id = 111111"),
        "GAQL should filter by campaign id"
    );
}

/// Verify that an empty result set maps to a not-found error (exit code 4).
#[tokio::test]
async fn test_get_campaign_empty_result_is_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/customers/{CUSTOMER_ID}/googleAds:search")))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({ "results": [] })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let err = provider
        .get_campaign(&CampaignId("404404".into()))
        .await
        .expect_err("missing campaign should be an error");
    assert_eq!(err.exit_code(), 4, "not-found contract, got: {err}");
}

// ── create_campaign ───────────────────────────────────────────────────────────

/// Verify the two-step create: budget mutate, then campaign mutate, then a
/// GAQL fetch of the created campaign.
#[tokio::test]
async fn test_create_campaign_creates_budget_then_campaign() {
    let server = MockServer::start().await;

    // Step 1: budget creation.
    Mock::given(method("POST"))
        .and(path(format!(
            "/customers/{CUSTOMER_ID}/campaignBudgets:mutate"
        )))
        .and(body_partial_json(serde_json::json!({
            "operations": [{ "create": { "amountMicros": "5000000" } }]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::budget_mutate_response()))
        .expect(1)
        .mount(&server)
        .await;

    // Step 2: campaign creation referencing the budget.
    Mock::given(method("POST"))
        .and(path(format!("/customers/{CUSTOMER_ID}/campaigns:mutate")))
        .and(body_partial_json(serde_json::json!({
            "operations": [{
                "create": {
                    "name": "New Search Campaign",
                    "status": "PAUSED",
                    "advertisingChannelType": "SEARCH",
                    "campaignBudget": "customers/1234567890/campaignBudgets/999003",
                    // Mandatory since 2026-04-01: mutates fail without the
                    // EU political advertising declaration (EU-PAR).
                    "containsEuPoliticalAdvertising":
                        "DOES_NOT_CONTAIN_EU_POLITICAL_ADVERTISING",
                }
            }]
        })))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(fixtures::campaign_mutate_response()),
        )
        .expect(1)
        .mount(&server)
        .await;

    // Step 3: GAQL fetch of the created campaign.
    Mock::given(method("POST"))
        .and(path(format!("/customers/{CUSTOMER_ID}/googleAds:search")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{
                "campaign": {
                    "resourceName": "customers/1234567890/campaigns/333333",
                    "id": "333333",
                    "name": "New Search Campaign",
                    "status": "PAUSED",
                    "advertisingChannelType": "SEARCH",
                    "startDate": "2026-06-10"
                },
                "campaignBudget": { "amountMicros": "5000000" }
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let input = CreateCampaignInput {
        name: "New Search Campaign".into(),
        objective: "SEARCH".into(),
        status: Some(CampaignStatus::Paused),
        budget: Some(Budget {
            amount: 5.0,
            currency: "USD".into(),
            kind: BudgetKind::Daily,
        }),
        extra: None,
    };

    let campaign = provider
        .create_campaign(&input)
        .await
        .expect("create_campaign should succeed");

    assert_eq!(campaign.id.0, "333333");
    assert_eq!(campaign.name, "New Search Campaign");
    assert_eq!(campaign.status, CampaignStatus::Paused);
}

/// Verify that creating without a budget is a client-side validation error.
#[tokio::test]
async fn test_create_campaign_without_budget_is_validation_error() {
    let server = MockServer::start().await;
    let provider = make_provider(&server.uri());

    let input = CreateCampaignInput {
        name: "No Budget".into(),
        objective: "SEARCH".into(),
        status: None,
        budget: None,
        extra: None,
    };

    let err = provider
        .create_campaign(&input)
        .await
        .expect_err("missing budget must fail before any HTTP call");
    assert_eq!(err.exit_code(), 2, "validation contract, got: {err}");
}

// ── update / delete ───────────────────────────────────────────────────────────

/// Verify `update_campaign` sends a mutate with an updateMask.
#[tokio::test]
async fn test_update_campaign_sends_update_mask() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/customers/{CUSTOMER_ID}/campaigns:mutate")))
        .and(body_partial_json(serde_json::json!({
            "operations": [{
                "updateMask": "name,status",
                "update": {
                    "resourceName": "customers/1234567890/campaigns/111111",
                    "name": "Renamed",
                    "status": "PAUSED",
                }
            }]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{ "resourceName": "customers/1234567890/campaigns/111111" }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    // Post-update GAQL fetch.
    Mock::given(method("POST"))
        .and(path(format!("/customers/{CUSTOMER_ID}/googleAds:search")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{
                "campaign": {
                    "resourceName": "customers/1234567890/campaigns/111111",
                    "id": "111111",
                    "name": "Renamed",
                    "status": "PAUSED",
                    "advertisingChannelType": "SEARCH",
                    "startDate": "2026-01-15"
                }
            }]
        })))
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

    let campaign = provider
        .update_campaign(&CampaignId("111111".into()), &input)
        .await
        .expect("update_campaign should succeed");
    assert_eq!(campaign.name, "Renamed");
}

/// Verify `delete_campaign` sends a remove operation (Google soft-deletes).
#[tokio::test]
async fn test_delete_campaign_sends_remove_operation() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/customers/{CUSTOMER_ID}/campaigns:mutate")))
        .and(body_partial_json(serde_json::json!({
            "operations": [{ "remove": "customers/1234567890/campaigns/111111" }]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{ "resourceName": "customers/1234567890/campaigns/111111" }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    provider
        .delete_campaign(&CampaignId("111111".into()))
        .await
        .expect("delete_campaign should succeed");
}

// ── insights ──────────────────────────────────────────────────────────────────

/// Verify `get_insights` issues a metrics GAQL query and maps rows with
/// cost converted from micros.
#[tokio::test]
async fn test_get_insights_maps_metrics_rows() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/customers/{CUSTOMER_ID}/googleAds:search")))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(fixtures::insights_search_response()),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let report = provider
        .get_insights(&InsightsQuery::default())
        .await
        .expect("get_insights should succeed");

    assert_eq!(report.provider, "google");
    assert_eq!(report.rows.len(), 2);

    let row = &report.rows[0];
    assert!((row.metrics["impressions"].value - 45230.0).abs() < f64::EPSILON);
    assert!(
        (row.metrics["cost"].value - 234.56).abs() < 1e-9,
        "costMicros should be converted to currency units, got {}",
        row.metrics["cost"].value
    );
    assert_eq!(
        row.dimensions.get("date").map(String::as_str),
        Some("2026-03-01")
    );
    assert_eq!(
        row.dimensions.get("campaign.name").map(String::as_str),
        Some("Search — Brand Terms")
    );
}

/// GAQL requires every query that selects a core date segment to bound it
/// with a finite date range. Without an explicit range the provider must
/// default one instead of emitting an invalid query.
#[tokio::test]
async fn test_get_insights_without_range_defaults_to_last_30_days() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/customers/{CUSTOMER_ID}/googleAds:search")))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(fixtures::insights_search_response()),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    provider
        .get_insights(&InsightsQuery::default())
        .await
        .expect("get_insights should succeed");

    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    let gaql = body["query"].as_str().unwrap();
    assert!(
        gaql.contains("WHERE segments.date DURING LAST_30_DAYS"),
        "queries selecting segments.date must carry a finite range: {gaql}"
    );
}

/// A bidding strategy passed via `extra` must replace the default
/// `manualCpc`, not coexist with it — the campaign bidding strategy is a
/// oneof and the API rejects two members.
#[tokio::test]
async fn test_create_campaign_extra_bidding_strategy_replaces_manual_cpc() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!(
            "/customers/{CUSTOMER_ID}/campaignBudgets:mutate"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::budget_mutate_response()))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path(format!("/customers/{CUSTOMER_ID}/campaigns:mutate")))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(fixtures::campaign_mutate_response()),
        )
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path(format!("/customers/{CUSTOMER_ID}/googleAds:search")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{
                "campaign": {
                    "resourceName": "customers/1234567890/campaigns/333333",
                    "id": "333333",
                    "name": "PMax Campaign",
                    "status": "PAUSED",
                    "advertisingChannelType": "PERFORMANCE_MAX",
                    "startDate": "2026-06-12"
                },
                "campaignBudget": { "amountMicros": "5000000" }
            }]
        })))
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let input = CreateCampaignInput {
        name: "PMax Campaign".into(),
        objective: "PERFORMANCE_MAX".into(),
        status: Some(CampaignStatus::Paused),
        budget: Some(Budget {
            amount: 5.0,
            currency: "USD".into(),
            kind: BudgetKind::Daily,
        }),
        extra: Some(serde_json::json!({ "maximizeConversionValue": {} })),
    };
    provider
        .create_campaign(&input)
        .await
        .expect("create_campaign should succeed");

    let requests = server.received_requests().await.unwrap();
    let campaign_body = requests
        .iter()
        .find(|r| r.url.path().ends_with("campaigns:mutate"))
        .map(|r| serde_json::from_slice::<serde_json::Value>(&r.body).unwrap())
        .expect("campaigns:mutate request should exist");
    let create = &campaign_body["operations"][0]["create"];
    assert!(
        create.get("maximizeConversionValue").is_some(),
        "extra bidding strategy must be forwarded: {create}"
    );
    assert!(
        create.get("manualCpc").is_none(),
        "default manualCpc must yield to the explicit strategy: {create}"
    );
}

// ── errors & health ───────────────────────────────────────────────────────────

/// Verify a Google API error response maps to `MktError` with the HTTP status.
#[tokio::test]
async fn test_api_error_is_mapped() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/customers/{CUSTOMER_ID}/googleAds:search")))
        .respond_with(ResponseTemplate::new(401).set_body_json(fixtures::api_error_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let err = provider
        .list_campaigns(&CampaignFilters::default())
        .await
        .expect_err("401 should be an error");

    let msg = err.to_string();
    assert!(msg.contains("google"), "got: {msg}");
    assert!(msg.contains("401"), "got: {msg}");
    assert_eq!(err.exit_code(), 3, "401 should map to auth exit code");
}

/// Verify `health_check` runs a trivial GAQL query and reports healthy.
#[tokio::test]
async fn test_health_check_reports_healthy() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/customers/{CUSTOMER_ID}/googleAds:search")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{ "customer": { "id": "1234567890" } }]
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
    assert_eq!(health.provider, "google");
}

// ── OAuth refresh ─────────────────────────────────────────────────────────────

/// Verify the refresh-token exchange posts the right form fields and
/// returns the access token.
#[tokio::test]
async fn test_oauth_refresh_exchange() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::oauth_token_response()))
        .expect(1)
        .mount(&server)
        .await;

    let token = mkt_google::fetch_access_token(
        "client-id",
        "client-secret",
        "refresh-token",
        &format!("{}/token", server.uri()),
    )
    .await
    .expect("token exchange should succeed");

    assert_eq!(token.expose_secret(), "ya29.test-access-token");

    let requests = server.received_requests().await.unwrap();
    let body = String::from_utf8_lossy(&requests[0].body);
    assert!(body.contains("grant_type=refresh_token"), "got: {body}");
    assert!(body.contains("refresh_token=refresh-token"), "got: {body}");
}
