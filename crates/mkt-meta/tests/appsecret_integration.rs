#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]
//! Wiremock-based integration tests for optional `appsecret_proof` support.
//!
//! Apps with the "Require App Secret" setting enabled reject Graph API
//! calls that lack an `appsecret_proof` parameter: an HMAC-SHA256 of the
//! access token keyed by the app secret, hex lowercase. When an app
//! secret is configured the proof must travel as a query parameter on
//! GET/DELETE and as a JSON body field on POST; when it is not
//! configured no request may carry it.

use hmac::{Hmac, Mac};
use mkt_core::models::{CampaignFilters, CampaignId, CampaignStatus, CreateCampaignInput};
use mkt_core::provider::MarketingProvider;
use mkt_meta::{MetaClient, MetaProvider};
use secrecy::SecretString;
use sha2::Sha256;
use wiremock::matchers::{body_partial_json, method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── Shared helpers ────────────────────────────────────────────────────────────

const AD_ACCOUNT_ID: &str = "123456789";
const ACCESS_TOKEN: &str = "test_access_token";
const APP_SECRET: &str = "test_app_secret";

/// HMAC-SHA256(key = `APP_SECRET`, message = `ACCESS_TOKEN`) as lowercase
/// hex, computed independently with `openssl dgst -sha256 -hmac`. Pinning
/// the literal guards against the test and the implementation making the
/// same mistake (e.g. swapping key and message).
const EXPECTED_PROOF: &str = "f86b8540509cdb3ad6b1ed956b4c197aff473ad0b9cf075e0b0dca4a8302f6fc";

/// Compute the proof with the same primitives the client uses, then check
/// it against the externally computed constant.
fn expected_proof() -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(APP_SECRET.as_bytes())
        .expect("HMAC accepts keys of any length");
    mac.update(ACCESS_TOKEN.as_bytes());
    let proof = hex::encode(mac.finalize().into_bytes());
    assert_eq!(
        proof, EXPECTED_PROOF,
        "test-side HMAC must match the openssl-computed reference value"
    );
    proof
}

/// Build a [`MetaProvider`] that sends requests to `base_url`, optionally
/// configured with an app secret.
fn make_provider(base_url: &str, app_secret: Option<&str>) -> MetaProvider {
    let mut client = MetaClient::new_with_base_url(
        SecretString::new(ACCESS_TOKEN.to_string().into()),
        AD_ACCOUNT_ID.to_string(),
        // The client joins base_url + path, so we need a trailing slash.
        format!("{base_url}/"),
    )
    .expect("client should build");
    if let Some(secret) = app_secret {
        client = client.with_app_secret(SecretString::new(secret.to_string().into()));
    }
    MetaProvider::new(client, None, None)
}

/// Matcher asserting the JSON request body does NOT contain the given key.
struct BodyLacksKey(&'static str);

impl wiremock::Match for BodyLacksKey {
    fn matches(&self, request: &wiremock::Request) -> bool {
        serde_json::from_slice::<serde_json::Value>(&request.body)
            .is_ok_and(|json| json.get(self.0).is_none())
    }
}

fn empty_list_response() -> serde_json::Value {
    serde_json::json!({
        "data": [],
        "paging": { "cursors": { "before": "", "after": "" } }
    })
}

fn campaign_fixture(id: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "name": "Proofed Campaign",
        "status": "PAUSED",
        "objective": "OUTCOME_SALES",
        "daily_budget": "2000",
        "created_time": "2026-06-01T10:00:00+0000",
        "updated_time": null
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// GET requests must carry `appsecret_proof` as a query parameter with the
/// exact HMAC-SHA256 hex value when an app secret is configured.
#[tokio::test]
async fn test_get_includes_appsecret_proof_query_param() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/act_{AD_ACCOUNT_ID}/campaigns")))
        .and(query_param("appsecret_proof", expected_proof()))
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_list_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri(), Some(APP_SECRET));
    let result = provider.list_campaigns(&CampaignFilters::default()).await;

    assert!(
        result.is_ok(),
        "GET with appsecret_proof should match the mock and succeed: {result:?}"
    );
}

/// POST requests must carry `appsecret_proof` as a JSON body field with
/// the exact HMAC-SHA256 hex value when an app secret is configured. The
/// follow-up GET (create then fetch) must carry it as a query parameter.
#[tokio::test]
async fn test_post_includes_appsecret_proof_in_body() {
    let server = MockServer::start().await;
    let created_id = "120330000000000077";

    Mock::given(method("POST"))
        .and(path(format!("/act_{AD_ACCOUNT_ID}/campaigns")))
        .and(body_partial_json(serde_json::json!({
            "appsecret_proof": expected_proof()
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({ "id": created_id, "success": true })),
        )
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path(format!("/{created_id}")))
        .and(query_param("appsecret_proof", expected_proof()))
        .respond_with(ResponseTemplate::new(200).set_body_json(campaign_fixture(created_id)))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri(), Some(APP_SECRET));
    let input = CreateCampaignInput {
        name: "Proofed Campaign".into(),
        objective: "OUTCOME_SALES".into(),
        status: Some(CampaignStatus::Paused),
        budget: None,
        extra: None,
    };

    let campaign = provider
        .create_campaign(&input)
        .await
        .expect("create_campaign with appsecret_proof should succeed");
    assert_eq!(campaign.id.0, created_id);
}

/// DELETE requests must carry `appsecret_proof` as a query parameter when
/// an app secret is configured.
#[tokio::test]
async fn test_delete_includes_appsecret_proof_query_param() {
    let server = MockServer::start().await;
    let campaign_id = "120330000000000007";

    Mock::given(method("DELETE"))
        .and(path(format!("/{campaign_id}")))
        .and(query_param("appsecret_proof", expected_proof()))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({ "success": true })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri(), Some(APP_SECRET));
    let result = provider
        .delete_campaign(&CampaignId(campaign_id.into()))
        .await;

    assert!(
        result.is_ok(),
        "DELETE with appsecret_proof should match the mock and succeed: {result:?}"
    );
}

/// Without an app secret, GET requests must NOT carry `appsecret_proof`.
#[tokio::test]
async fn test_get_without_app_secret_omits_proof() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/act_{AD_ACCOUNT_ID}/campaigns")))
        .and(query_param_is_missing("appsecret_proof"))
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_list_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri(), None);
    let result = provider.list_campaigns(&CampaignFilters::default()).await;

    assert!(
        result.is_ok(),
        "GET without app secret must not send appsecret_proof: {result:?}"
    );
}

/// Without an app secret, POST bodies must NOT contain `appsecret_proof`.
#[tokio::test]
async fn test_post_without_app_secret_omits_proof() {
    let server = MockServer::start().await;
    let created_id = "120330000000000088";

    Mock::given(method("POST"))
        .and(path(format!("/act_{AD_ACCOUNT_ID}/campaigns")))
        .and(BodyLacksKey("appsecret_proof"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({ "id": created_id, "success": true })),
        )
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path(format!("/{created_id}")))
        .and(query_param_is_missing("appsecret_proof"))
        .respond_with(ResponseTemplate::new(200).set_body_json(campaign_fixture(created_id)))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri(), None);
    let input = CreateCampaignInput {
        name: "Proofed Campaign".into(),
        objective: "OUTCOME_SALES".into(),
        status: Some(CampaignStatus::Paused),
        budget: None,
        extra: None,
    };

    let result = provider.create_campaign(&input).await;
    assert!(
        result.is_ok(),
        "POST without app secret must not send appsecret_proof: {result:?}"
    );
}
