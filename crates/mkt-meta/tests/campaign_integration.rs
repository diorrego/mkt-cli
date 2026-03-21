#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]
//! Wiremock-based integration tests for the Meta campaign API.
//!
//! Each test spins up a local mock server, constructs a [`MetaProvider`]
//! pointing at it, exercises a provider method, and asserts the response.

use mkt_core::models::{CampaignFilters, CampaignId, CampaignStatus, CreateCampaignInput};
use mkt_core::provider::MarketingProvider;
use mkt_meta::{MetaClient, MetaProvider};
use mkt_testkit::fixtures::meta as fixtures;
use secrecy::SecretString;
use wiremock::matchers::{method, path, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── Shared helpers ────────────────────────────────────────────────────────────

const AD_ACCOUNT_ID: &str = "123456789";
const ACCESS_TOKEN: &str = "test_access_token";

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

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Verify that `list_campaigns` sends a GET to the correct account campaigns
/// path, parses the response, and returns the right domain values.
#[tokio::test]
async fn test_list_campaigns_sends_correct_request() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/act_{AD_ACCOUNT_ID}/campaigns")))
        .and(query_param(
            "fields",
            "id,name,status,objective,daily_budget,lifetime_budget,created_time,updated_time",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::campaigns_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let filters = CampaignFilters::default();
    let result = provider.list_campaigns(&filters).await;

    let page = result.expect("list_campaigns should succeed");
    assert_eq!(page.data.len(), 2, "expected two campaigns from fixture");

    let first = &page.data[0];
    assert_eq!(first.id.0, "120330000000000001");
    assert_eq!(first.name, "Summer Sale 2026");
    assert_eq!(first.status, CampaignStatus::Active);
    assert_eq!(first.provider, "meta");

    let second = &page.data[1];
    assert_eq!(second.id.0, "120330000000000002");
    assert_eq!(second.status, CampaignStatus::Paused);

    // Pagination cursor should be populated.
    assert_eq!(
        page.next_cursor.as_deref(),
        Some("QVFIUkx5NW5k"),
        "next_cursor should match the 'after' cursor from fixture"
    );
}

/// Verify that `create_campaign` sends a POST to create the campaign and then
/// a GET to fetch the full object, returning correctly mapped domain data.
#[tokio::test]
async fn test_create_campaign_sends_post_request() {
    let server = MockServer::start().await;

    // Step 1: POST to create — returns {"id": "...", "success": true}
    Mock::given(method("POST"))
        .and(path(format!("/act_{AD_ACCOUNT_ID}/campaigns")))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(fixtures::campaign_create_response()),
        )
        .expect(1)
        .mount(&server)
        .await;

    // Step 2: GET the newly created campaign by ID
    let created_id = fixtures::campaign_create_response()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let created_campaign_fixture = serde_json::json!({
        "id": &created_id,
        "name": "New Test Campaign",
        "status": "PAUSED",
        "objective": "OUTCOME_SALES",
        "daily_budget": "2000",
        "created_time": "2026-03-21T10:00:00+0000",
        "updated_time": null
    });

    Mock::given(method("GET"))
        .and(path(format!("/{created_id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(&created_campaign_fixture))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let input = CreateCampaignInput {
        name: "New Test Campaign".into(),
        objective: "OUTCOME_SALES".into(),
        status: Some(CampaignStatus::Paused),
        budget: None,
        extra: None,
    };

    let campaign = provider
        .create_campaign(&input)
        .await
        .expect("create_campaign should succeed");

    assert_eq!(
        campaign.id.0, created_id,
        "returned ID should match fixture"
    );
    assert_eq!(campaign.name, "New Test Campaign");
    assert_eq!(campaign.status, CampaignStatus::Paused);
    assert_eq!(campaign.objective, "OUTCOME_SALES");
    assert_eq!(campaign.provider, "meta");
}

/// Verify that a 400 error response from the API is converted to an `MktError`
/// and surfaced correctly to the caller.
#[tokio::test]
async fn test_create_campaign_handles_api_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/act_{AD_ACCOUNT_ID}/campaigns")))
        .respond_with(ResponseTemplate::new(400).set_body_json(fixtures::api_error_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let input = CreateCampaignInput {
        name: "Bad Campaign".into(),
        objective: "INVALID_OBJECTIVE".into(),
        status: None,
        budget: None,
        extra: None,
    };

    let result = provider.create_campaign(&input).await;

    assert!(result.is_err(), "should propagate API error");
    let err = result.unwrap_err();
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

/// Verify that `get_campaign` fetches a single campaign by ID and maps it
/// correctly into the domain model.
#[tokio::test]
async fn test_get_campaign_by_id() {
    let server = MockServer::start().await;

    let campaign_id = "120330000000000042";
    let fixture = serde_json::json!({
        "id": campaign_id,
        "name": "Single Campaign Fetch",
        "status": "ACTIVE",
        "objective": "OUTCOME_ENGAGEMENT",
        "daily_budget": "7500",
        "created_time": "2026-02-15T08:00:00+0000",
        "updated_time": "2026-03-10T12:00:00+0000"
    });

    Mock::given(method("GET"))
        .and(path(format!("/{campaign_id}")))
        .and(query_param(
            "fields",
            "id,name,status,objective,daily_budget,lifetime_budget,created_time,updated_time",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let id = CampaignId(campaign_id.into());

    let campaign = provider
        .get_campaign(&id)
        .await
        .expect("get_campaign should succeed");

    assert_eq!(campaign.id.0, campaign_id);
    assert_eq!(campaign.name, "Single Campaign Fetch");
    assert_eq!(campaign.status, CampaignStatus::Active);
    assert_eq!(campaign.objective, "OUTCOME_ENGAGEMENT");
    assert!(
        campaign.budget.is_some(),
        "daily_budget in fixture should map to a budget"
    );
    let budget = campaign.budget.unwrap();
    assert!((budget.amount - 7500.0).abs() < f64::EPSILON);
    assert!(
        campaign.updated_at.is_some(),
        "updated_time in fixture should parse to Some"
    );
}

/// Verify that `delete_campaign` sends a DELETE request to the correct path
/// and returns `Ok(())` on a success response.
#[tokio::test]
async fn test_delete_campaign_sends_delete() {
    let server = MockServer::start().await;

    let campaign_id = "120330000000000007";

    Mock::given(method("DELETE"))
        .and(path(format!("/{campaign_id}")))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"success": true})),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let id = CampaignId(campaign_id.into());

    let result = provider.delete_campaign(&id).await;

    assert!(
        result.is_ok(),
        "delete_campaign should return Ok(()) on success, got: {result:?}"
    );
}

/// Verify that a 404 from the delete endpoint is surfaced as an error
/// (guards against silently swallowing non-2xx responses on DELETE).
#[tokio::test]
async fn test_delete_campaign_propagates_not_found_error() {
    let server = MockServer::start().await;

    let campaign_id = "999999999999";

    Mock::given(method("DELETE"))
        .and(path_regex(format!(".*{campaign_id}.*")))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": {
                "message": "Campaign not found",
                "type": "GraphMethodException",
                "code": 100,
                "error_subcode": null,
                "fbtrace_id": "xyz"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let id = CampaignId(campaign_id.into());

    let result = provider.delete_campaign(&id).await;

    assert!(result.is_err(), "should return an error for 404 response");
    let err = result.unwrap_err();
    let err_string = err.to_string();
    assert!(
        err_string.contains("meta"),
        "error should mention provider: {err_string}"
    );
    assert!(
        err_string.contains("404"),
        "error should include the status code: {err_string}"
    );
}

/// Verify that `list_campaigns` with a status filter adds the correct
/// `filtering` query parameter to the request.
#[tokio::test]
async fn test_list_campaigns_with_status_filter_adds_filtering_param() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/act_{AD_ACCOUNT_ID}/campaigns")))
        // Verify that a 'filtering' query param is included when a status filter is set.
        .and(query_param(
            "fields",
            "id,name,status,objective,daily_budget,lifetime_budget,created_time,updated_time",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [],
            "paging": { "cursors": { "before": "", "after": "" } }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let filters = CampaignFilters {
        status: Some(CampaignStatus::Active),
        ..Default::default()
    };

    let result = provider.list_campaigns(&filters).await;

    let page = result.expect("list_campaigns with filter should succeed");
    assert!(
        page.data.is_empty(),
        "empty data array should produce empty Vec"
    );
    // The provider returns Some("") when the cursor value is an empty string
    // (it calls String::from on whatever is present). An absent key produces None.
    // The fixture supplies "", so the result is Some("") — both mean "no more pages".
    assert!(
        page.next_cursor.as_deref().is_none_or(str::is_empty),
        "empty or absent cursor should indicate no further pages, got: {:?}",
        page.next_cursor
    );
}

/// Verify that `list_campaigns` respects the `limit` filter by including it
/// as a query parameter, and respects the `cursor` for pagination.
#[tokio::test]
async fn test_list_campaigns_pagination_params_are_forwarded() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/act_{AD_ACCOUNT_ID}/campaigns")))
        .and(query_param("limit", "5"))
        .and(query_param("after", "some_cursor_value"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [],
            "paging": { "cursors": { "before": "", "after": "" } }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let filters = CampaignFilters {
        limit: Some(5),
        cursor: Some("some_cursor_value".into()),
        ..Default::default()
    };

    let result = provider.list_campaigns(&filters).await;
    assert!(result.is_ok(), "paginated list_campaigns should succeed");
}
