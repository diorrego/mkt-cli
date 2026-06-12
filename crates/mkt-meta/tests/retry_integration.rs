#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]
//! End-to-end retry behavior through a real provider client: reads retry
//! rate limits honoring `Retry-After`, writes never repeat a request that
//! may have executed.

use mkt_core::http::RetryPolicy;
use mkt_core::models::CampaignFilters;
use mkt_core::provider::MarketingProvider;
use mkt_meta::{MetaClient, MetaProvider};
use mkt_testkit::fixtures::meta as fixtures;
use secrecy::SecretString;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const AD_ACCOUNT_ID: &str = "123456789";

fn make_retrying_provider(base_url: &str) -> MetaProvider {
    let client = MetaClient::new_with_base_url(
        SecretString::new("tok".to_string().into()),
        AD_ACCOUNT_ID.to_string(),
        format!("{base_url}/"),
    )
    .expect("client should build")
    .with_retry_policy(RetryPolicy::standard());
    MetaProvider::new(client, None, None)
}

/// A 429 with `Retry-After: 0` must be retried and succeed on the second
/// attempt without surfacing an error.
#[tokio::test]
async fn read_retries_a_rate_limit_and_succeeds() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/act_{AD_ACCOUNT_ID}/campaigns")))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "0")
                .set_body_json(serde_json::json!({"error": {"message": "throttled"}})),
        )
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path(format!("/act_{AD_ACCOUNT_ID}/campaigns")))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::campaigns_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_retrying_provider(&server.uri());
    let page = provider
        .list_campaigns(&CampaignFilters::default())
        .await
        .expect("the retried read should succeed");
    assert!(!page.data.is_empty());

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2, "one rate-limited attempt + one retry");
}

/// A 500 on a write is NOT retried: the campaign may have been created and
/// a blind retry would duplicate spend.
#[tokio::test]
async fn write_does_not_retry_server_errors() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/act_{AD_ACCOUNT_ID}/campaigns")))
        .respond_with(
            ResponseTemplate::new(500)
                .set_body_json(serde_json::json!({"error": {"message": "boom"}})),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_retrying_provider(&server.uri());
    let input = mkt_core::models::CreateCampaignInput {
        name: "C".into(),
        objective: "OUTCOME_TRAFFIC".into(),
        ..Default::default()
    };
    provider
        .create_campaign(&input)
        .await
        .expect_err("a 500 on create must fail without retrying");

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1, "writes must not repeat on 5xx");
}

/// Auth errors are terminal: no retry regardless of policy.
#[tokio::test]
async fn read_does_not_retry_auth_errors() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/act_{AD_ACCOUNT_ID}/campaigns")))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": {"message": "Invalid OAuth access token", "type": "OAuthException", "code": 190}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_retrying_provider(&server.uri());
    provider
        .list_campaigns(&CampaignFilters::default())
        .await
        .expect_err("auth failures are terminal");

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
}
