#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]
//! Wiremock-based integration tests for the LinkedIn Marketing API
//! campaign and analytics endpoints.

use mkt_core::models::{
    Budget, BudgetKind, CampaignFilters, CampaignId, CampaignStatus, CreateCampaignInput,
    InsightsQuery, UpdateCampaignInput,
};
use mkt_core::provider::MarketingProvider;
use mkt_linkedin::{LinkedInClient, LinkedInProvider};
use mkt_testkit::fixtures::linkedin as fixtures;
use secrecy::SecretString;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── Shared helpers ────────────────────────────────────────────────────────────

const AD_ACCOUNT_ID: &str = "506333826";
const ACCESS_TOKEN: &str = "li-test-token";

/// Build a [`LinkedInProvider`] that sends requests to `base_url`.
fn make_provider(base_url: &str) -> LinkedInProvider {
    let client = LinkedInClient::new_with_base_url(
        SecretString::new(ACCESS_TOKEN.to_string().into()),
        AD_ACCOUNT_ID.to_string(),
        format!("{base_url}/"),
    )
    .expect("client should build");
    LinkedInProvider::new(client)
}

// ── list_campaigns ────────────────────────────────────────────────────────────

/// Verify that `list_campaigns` sends the versioned Rest.li finder request
/// and maps elements into domain campaigns.
#[tokio::test]
async fn test_list_campaigns_sends_versioned_finder() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/adAccounts/{AD_ACCOUNT_ID}/adCampaigns")))
        .and(header("Linkedin-Version", "202605"))
        .and(header("X-Restli-Protocol-Version", "2.0.0"))
        .and(header("authorization", format!("Bearer {ACCESS_TOKEN}")))
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
    assert_eq!(first.id.0, "145282384");
    assert_eq!(first.provider, "linkedin");
    assert_eq!(first.name, "Lead Gen — DACH Q2");
    assert_eq!(first.status, CampaignStatus::Active);
    assert_eq!(first.objective, "LEAD_GENERATION");
    let budget = first.budget.as_ref().expect("dailyBudget should map");
    assert!((budget.amount - 18.0).abs() < f64::EPSILON);
    assert_eq!(budget.currency, "USD");
    assert_eq!(budget.kind, BudgetKind::Daily);
    assert!(
        first.updated_at.is_some(),
        "lastModified should map to updated_at"
    );

    let second = &page.data[1];
    assert_eq!(second.status, CampaignStatus::Paused);

    assert_eq!(page.next_cursor.as_deref(), Some("DgGerr1iVQreCJVjZDOW"));
}

/// Verify a status filter produces the Rest.li search expression with
/// URN-safe encoding (structural characters unencoded).
#[tokio::test]
async fn test_list_campaigns_status_filter_uses_restli_syntax() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/adAccounts/{AD_ACCOUNT_ID}/adCampaigns")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "elements": [],
            "metadata": {}
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
    let raw_query = requests[0].url.query().unwrap_or("");
    assert!(
        raw_query.contains("q=search"),
        "finder param missing: {raw_query}"
    );
    assert!(
        raw_query.contains("search=(status:(values:List(ACTIVE)))"),
        "Rest.li search expression wrong: {raw_query}"
    );
}

// ── get_campaign ──────────────────────────────────────────────────────────────

/// Verify `get_campaign` fetches a single campaign and maps audit stamps.
#[tokio::test]
async fn test_get_campaign_by_id() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!(
            "/adAccounts/{AD_ACCOUNT_ID}/adCampaigns/145282384"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::campaign_get_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let campaign = provider
        .get_campaign(&CampaignId("145282384".into()))
        .await
        .expect("get_campaign should succeed");

    assert_eq!(campaign.id.0, "145282384");
    assert_eq!(campaign.objective, "LEAD_GENERATION");
    // created.time 1767225600000 ms = 2026-01-01T00:00:00Z
    assert_eq!(
        campaign.created_at.format("%Y-%m-%d").to_string(),
        "2026-01-01"
    );
}

// ── create_campaign ───────────────────────────────────────────────────────────

/// Verify `create_campaign` POSTs the campaign body, reads the new ID from
/// the `x-restli-id` response header, and fetches the full object.
#[tokio::test]
async fn test_create_campaign_reads_restli_id_header() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/adAccounts/{AD_ACCOUNT_ID}/adCampaigns")))
        .respond_with(
            ResponseTemplate::new(201)
                .insert_header("x-restli-id", "145282384")
                .set_body_json(serde_json::json!({})),
        )
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path(format!(
            "/adAccounts/{AD_ACCOUNT_ID}/adCampaigns/145282384"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::campaign_get_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let input = CreateCampaignInput {
        name: "Lead Gen — DACH Q2".into(),
        objective: "LEAD_GENERATION".into(),
        status: Some(CampaignStatus::Paused),
        budget: Some(Budget {
            amount: 18.0,
            currency: "USD".into(),
            kind: BudgetKind::Daily,
        }),
        extra: Some(serde_json::json!({
            "campaignGroup": "urn:li:sponsoredCampaignGroup:603030884",
        })),
    };

    let campaign = provider
        .create_campaign(&input)
        .await
        .expect("create_campaign should succeed");
    assert_eq!(campaign.id.0, "145282384");

    // Inspect the POST body for required LinkedIn fields.
    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(
        body["account"],
        format!("urn:li:sponsoredAccount:{AD_ACCOUNT_ID}")
    );
    assert_eq!(
        body["campaignGroup"],
        "urn:li:sponsoredCampaignGroup:603030884"
    );
    assert_eq!(body["objectiveType"], "LEAD_GENERATION");
    assert_eq!(body["status"], "PAUSED");
    assert_eq!(
        body["dailyBudget"]["amount"], "18",
        "money must be a string"
    );
    assert!(
        body["runSchedule"]["start"].is_i64(),
        "runSchedule.start must be epoch ms"
    );
}

/// Verify create without the campaignGroup URN fails fast with validation.
#[tokio::test]
async fn test_create_campaign_requires_campaign_group() {
    let server = MockServer::start().await;
    let provider = make_provider(&server.uri());

    let input = CreateCampaignInput {
        name: "X".into(),
        objective: "WEBSITE_VISIT".into(),
        status: None,
        budget: Some(Budget {
            amount: 10.0,
            currency: "USD".into(),
            kind: BudgetKind::Daily,
        }),
        extra: None,
    };

    let err = provider
        .create_campaign(&input)
        .await
        .expect_err("missing campaignGroup must fail before any HTTP call");
    assert_eq!(err.exit_code(), 2, "validation contract, got: {err}");
}

// ── update / delete ───────────────────────────────────────────────────────────

/// Verify `update_campaign` sends a Rest.li `PARTIAL_UPDATE` patch.
#[tokio::test]
async fn test_update_campaign_sends_partial_update_patch() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!(
            "/adAccounts/{AD_ACCOUNT_ID}/adCampaigns/145282384"
        )))
        .and(header("X-RestLi-Method", "PARTIAL_UPDATE"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path(format!(
            "/adAccounts/{AD_ACCOUNT_ID}/adCampaigns/145282384"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::campaign_get_response()))
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
        .update_campaign(&CampaignId("145282384".into()), &input)
        .await
        .expect("update_campaign should succeed");

    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["patch"]["$set"]["name"], "Renamed");
    assert_eq!(body["patch"]["$set"]["status"], "PAUSED");
}

/// Verify `delete_campaign` soft-deletes a non-draft campaign via
/// `PARTIAL_UPDATE` to `PENDING_DELETION` (hard DELETE only works for
/// drafts, so the provider checks the status first).
#[tokio::test]
async fn test_delete_campaign_sets_pending_deletion() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!(
            "/adAccounts/{AD_ACCOUNT_ID}/adCampaigns/145282384"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::campaign_get_response()))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path(format!(
            "/adAccounts/{AD_ACCOUNT_ID}/adCampaigns/145282384"
        )))
        .and(header("X-RestLi-Method", "PARTIAL_UPDATE"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    provider
        .delete_campaign(&CampaignId("145282384".into()))
        .await
        .expect("delete_campaign should succeed");

    let requests = server.received_requests().await.unwrap();
    let patch_request = requests
        .iter()
        .find(|r| r.method == wiremock::http::Method::POST)
        .expect("PARTIAL_UPDATE request should exist");
    let body: serde_json::Value = serde_json::from_slice(&patch_request.body).unwrap();
    assert_eq!(body["patch"]["$set"]["status"], "PENDING_DELETION");
}

/// LinkedIn documents hard DELETE as the deletion method for `DRAFT`
/// campaigns; `PENDING_DELETION` is not a documented transition for them.
#[tokio::test]
async fn test_delete_campaign_draft_uses_hard_delete() {
    let server = MockServer::start().await;

    let mut draft = fixtures::campaign_get_response();
    draft["status"] = serde_json::json!("DRAFT");

    Mock::given(method("GET"))
        .and(path(format!(
            "/adAccounts/{AD_ACCOUNT_ID}/adCampaigns/145282384"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(draft))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("DELETE"))
        .and(path(format!(
            "/adAccounts/{AD_ACCOUNT_ID}/adCampaigns/145282384"
        )))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    provider
        .delete_campaign(&CampaignId("145282384".into()))
        .await
        .expect("deleting a draft should hard-delete");
}

// ── insights ──────────────────────────────────────────────────────────────────

/// Verify `get_insights` queries adAnalytics and maps elements with the
/// string-typed cost converted to a numeric `cost` metric.
#[tokio::test]
async fn test_get_insights_maps_analytics_elements() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/adAnalytics"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::analytics_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let report = provider
        .get_insights(&InsightsQuery::default())
        .await
        .expect("get_insights should succeed");

    assert_eq!(report.provider, "linkedin");
    assert_eq!(report.rows.len(), 2);

    let row = &report.rows[0];
    assert!((row.metrics["impressions"].value - 165.0).abs() < f64::EPSILON);
    assert!((row.metrics["clicks"].value - 11.0).abs() < f64::EPSILON);
    assert!(
        (row.metrics["cost"].value - 19.91833).abs() < 1e-9,
        "costInLocalCurrency string should convert to numeric cost"
    );
    assert_eq!(
        row.dimensions.get("date").map(String::as_str),
        Some("2026-03-01")
    );
    assert_eq!(
        row.dimensions.get("pivot").map(String::as_str),
        Some("urn:li:sponsoredCampaign:145282384")
    );

    // The request must carry the Rest.li dateRange/accounts syntax unencoded.
    let requests = server.received_requests().await.unwrap();
    let raw_query = requests[0].url.query().unwrap_or("");
    assert!(raw_query.contains("q=analytics"), "got: {raw_query}");
    assert!(raw_query.contains("pivot=CAMPAIGN"), "got: {raw_query}");
    assert!(
        raw_query.contains(&format!(
            "accounts=List(urn%3Ali%3AsponsoredAccount%3A{AD_ACCOUNT_ID})"
        )),
        "URN colons must be %3A-encoded inside List(): {raw_query}"
    );
    // dateRange.start is a required adAnalytics parameter: queries without
    // an explicit range must still send a finite default.
    assert!(
        raw_query.contains("dateRange=(start:(year:"),
        "adAnalytics requires dateRange; a default must be sent: {raw_query}"
    );
}

/// adAnalytics accepts at most 20 metrics per request; reject locally with
/// a validation error instead of a late server-side 400.
#[tokio::test]
async fn test_get_insights_rejects_more_than_20_metrics() {
    let provider = make_provider("http://127.0.0.1:9"); // never reached

    let query = InsightsQuery {
        metrics: (0..21).map(|i| format!("metric{i}")).collect(),
        ..Default::default()
    };
    let err = provider
        .get_insights(&query)
        .await
        .expect_err("21 metrics must be rejected");
    assert!(
        matches!(err, mkt_core::error::MktError::ValidationError { .. }),
        "expected ValidationError, got: {err:?}"
    );
}

// ── errors & health ───────────────────────────────────────────────────────────

/// Verify a LinkedIn error response maps to `MktError` with status + code.
#[tokio::test]
async fn test_api_error_is_mapped() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/adAccounts/{AD_ACCOUNT_ID}/adCampaigns")))
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
    assert!(msg.contains("linkedin"), "got: {msg}");
    assert!(msg.contains("401"), "got: {msg}");
    assert!(
        msg.contains("REVOKED_ACCESS_TOKEN"),
        "should surface the LinkedIn error code: {msg}"
    );
    assert_eq!(err.exit_code(), 3);
}

/// Verify `health_check` runs a minimal finder and reports healthy.
#[tokio::test]
async fn test_health_check_reports_healthy() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/adAccounts/{AD_ACCOUNT_ID}/adCampaigns")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "elements": [],
            "metadata": {}
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
    assert_eq!(health.provider, "linkedin");
}
