#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]
//! Wiremock-based integration tests for the Meta ad set API.
//!
//! Each test spins up a local mock server, constructs a [`MetaProvider`]
//! pointing at it, exercises a provider method, and asserts the response.

use mkt_core::models::{AdSetStatus, Budget, BudgetKind, CampaignId, CreateAdSetInput};
use mkt_core::provider::MarketingProvider;
use mkt_meta::{MetaClient, MetaProvider};
use mkt_testkit::fixtures::meta as fixtures;
use secrecy::SecretString;
use wiremock::matchers::{body_partial_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── Shared helpers ────────────────────────────────────────────────────────────

const AD_ACCOUNT_ID: &str = "123456789";
const ACCESS_TOKEN: &str = "test_access_token";
const CAMPAIGN_ID: &str = "120330000000000001";

/// Build a [`MetaProvider`] that sends requests to `base_url`.
fn make_provider(base_url: &str) -> MetaProvider {
    let client = MetaClient::new_with_base_url(
        SecretString::new(ACCESS_TOKEN.to_string().into()),
        AD_ACCOUNT_ID.to_string(),
        // The client joins base_url + path, so we need a trailing slash.
        format!("{base_url}/"),
    )
    .expect("client should build");
    MetaProvider::new(client, None, None)
}

// ── list_adsets ───────────────────────────────────────────────────────────────

/// Verify that `list_adsets` sends a GET to `{campaign_id}/adsets`, parses
/// the fixture response, and maps every ad set into the domain model.
#[tokio::test]
async fn test_list_adsets_sends_correct_request() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/{CAMPAIGN_ID}/adsets")))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::adsets_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let campaign_id = CampaignId(CAMPAIGN_ID.into());

    let page = provider
        .list_adsets(&campaign_id)
        .await
        .expect("list_adsets should succeed");

    assert_eq!(page.data.len(), 3, "expected three ad sets from fixture");

    let first = &page.data[0];
    assert_eq!(first.id.0, "23845600000000001");
    assert_eq!(first.provider, "meta");
    assert_eq!(first.campaign_id.0, CAMPAIGN_ID);
    assert_eq!(first.name, "Lookalike — Email List 1%");
    assert_eq!(first.status, AdSetStatus::Active);
    let budget = first
        .budget
        .as_ref()
        .expect("first ad set has daily_budget");
    assert!((budget.amount - 2500.0).abs() < f64::EPSILON);
    assert_eq!(budget.kind, BudgetKind::Daily);

    let third = &page.data[2];
    assert_eq!(third.status, AdSetStatus::Paused);

    assert!(
        page.next_cursor.is_some(),
        "fixture paging cursor should map to next_cursor"
    );
}

/// Verify that `list_adsets` requests the ad set field set we map from.
#[tokio::test]
async fn test_list_adsets_requests_expected_fields() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/{CAMPAIGN_ID}/adsets")))
        .and(query_param(
            "fields",
            "id,campaign_id,name,status,targeting,daily_budget,lifetime_budget,\
             billing_event,optimization_goal,created_time,updated_time",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [],
            "paging": { "cursors": { "before": "", "after": "" } }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let campaign_id = CampaignId(CAMPAIGN_ID.into());

    let page = provider
        .list_adsets(&campaign_id)
        .await
        .expect("list_adsets should succeed");
    assert!(page.data.is_empty());
}

/// Verify that an API error from the adsets endpoint propagates as `MktError`.
#[tokio::test]
async fn test_list_adsets_handles_api_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/{CAMPAIGN_ID}/adsets")))
        .respond_with(ResponseTemplate::new(400).set_body_json(fixtures::api_error_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let campaign_id = CampaignId(CAMPAIGN_ID.into());

    let result = provider.list_adsets(&campaign_id).await;

    let err = result.expect_err("should propagate API error");
    let err_string = err.to_string();
    assert!(
        err_string.contains("meta"),
        "error should mention provider: {err_string}"
    );
    assert!(
        err_string.contains("400"),
        "error should include status code: {err_string}"
    );
}

// ── create_adset ──────────────────────────────────────────────────────────────

/// Verify that `create_adset` POSTs to `act_{id}/adsets` with the mapped body
/// and then GETs the full object, returning correctly mapped domain data.
#[tokio::test]
async fn test_create_adset_sends_post_request() {
    let server = MockServer::start().await;

    let created_id = "23845600000000099";

    // Step 1: POST to create — Meta returns {"id": "..."}.
    Mock::given(method("POST"))
        .and(path(format!("/act_{AD_ACCOUNT_ID}/adsets")))
        .and(body_partial_json(serde_json::json!({
            "campaign_id": CAMPAIGN_ID,
            "name": "Retargeting — Cart Abandoners",
            "status": "PAUSED",
            "daily_budget": "1500",
        })))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({ "id": created_id })),
        )
        .expect(1)
        .mount(&server)
        .await;

    // Step 2: GET the newly created ad set by ID.
    let created_fixture = serde_json::json!({
        "id": created_id,
        "campaign_id": CAMPAIGN_ID,
        "name": "Retargeting — Cart Abandoners",
        "status": "PAUSED",
        "daily_budget": "1500",
        "targeting": { "geo_locations": { "countries": ["US"] } },
        "billing_event": "IMPRESSIONS",
        "optimization_goal": "OFFSITE_CONVERSIONS",
        "created_time": "2026-06-09T10:00:00+0000",
        "updated_time": null
    });

    Mock::given(method("GET"))
        .and(path(format!("/{created_id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(&created_fixture))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let input = CreateAdSetInput {
        campaign_id: CampaignId(CAMPAIGN_ID.into()),
        name: "Retargeting — Cart Abandoners".into(),
        status: Some(AdSetStatus::Paused),
        targeting: Some(serde_json::json!({ "geo_locations": { "countries": ["US"] } })),
        budget: Some(Budget {
            amount: 1500.0,
            currency: "USD".into(),
            kind: BudgetKind::Daily,
        }),
        extra: Some(serde_json::json!({
            "billing_event": "IMPRESSIONS",
            "optimization_goal": "OFFSITE_CONVERSIONS",
        })),
    };

    let adset = provider
        .create_adset(&input)
        .await
        .expect("create_adset should succeed");

    assert_eq!(adset.id.0, created_id);
    assert_eq!(adset.provider, "meta");
    assert_eq!(adset.campaign_id.0, CAMPAIGN_ID);
    assert_eq!(adset.name, "Retargeting — Cart Abandoners");
    assert_eq!(adset.status, AdSetStatus::Paused);
    assert!(adset.targeting.is_some(), "targeting should be mapped");
    let budget = adset.budget.expect("budget should be mapped");
    assert!((budget.amount - 1500.0).abs() < f64::EPSILON);
    assert_eq!(budget.kind, BudgetKind::Daily);
}

/// Verify that a 400 error response from the create endpoint is surfaced.
#[tokio::test]
async fn test_create_adset_handles_api_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/act_{AD_ACCOUNT_ID}/adsets")))
        .respond_with(ResponseTemplate::new(400).set_body_json(fixtures::api_error_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let input = CreateAdSetInput {
        campaign_id: CampaignId(CAMPAIGN_ID.into()),
        name: "Bad Ad Set".into(),
        status: None,
        targeting: None,
        budget: None,
        extra: None,
    };

    let result = provider.create_adset(&input).await;

    let err = result.expect_err("should propagate API error");
    assert!(
        err.to_string().contains("400"),
        "error should include status code: {err}"
    );
}
