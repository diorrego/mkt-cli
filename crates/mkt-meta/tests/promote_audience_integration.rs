#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]
//! Wiremock-based integration tests for post promotion and audience
//! user uploads against the Meta Marketing API.

use mkt_core::models::{AdStatus, AudienceId, AudienceUser, PostId, PromotePostInput};
use mkt_core::pii;
use mkt_core::provider::MarketingProvider;
use mkt_meta::{MetaClient, MetaProvider};
use mkt_testkit::fixtures::meta as fixtures;
use secrecy::SecretString;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── Shared helpers ────────────────────────────────────────────────────────────

const AD_ACCOUNT_ID: &str = "123456789";
const ACCESS_TOKEN: &str = "test_access_token";

/// Build a [`MetaProvider`] that sends requests to `base_url`.
fn make_provider(base_url: &str) -> MetaProvider {
    let client = MetaClient::new_with_base_url(
        SecretString::new(ACCESS_TOKEN.to_string().into()),
        AD_ACCOUNT_ID.to_string(),
        format!("{base_url}/"),
    )
    .expect("client should build");
    MetaProvider::new(client, None, None)
}

// ── promote_post ──────────────────────────────────────────────────────────────

/// Verify the full boost flow: creative from `object_story_id`, then ad in
/// the target ad set, then a GET of the created ad mapped to the domain.
#[tokio::test]
async fn test_promote_post_creates_creative_then_ad() {
    let server = MockServer::start().await;

    let post_id = "123456789_987654321";
    let adset_id = "23845600000000001";
    let creative_id = "9988776655";
    let ad_id = "120440000000000001";

    // Step 1: creative referencing the organic post.
    Mock::given(method("POST"))
        .and(path(format!("/act_{AD_ACCOUNT_ID}/adcreatives")))
        .and(body_partial_json(serde_json::json!({
            "object_story_id": post_id,
        })))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({ "id": creative_id })),
        )
        .expect(1)
        .mount(&server)
        .await;

    // Step 2: ad inside the existing ad set, created paused for safety.
    Mock::given(method("POST"))
        .and(path(format!("/act_{AD_ACCOUNT_ID}/ads")))
        .and(body_partial_json(serde_json::json!({
            "adset_id": adset_id,
            "creative": { "creative_id": creative_id },
            "status": "PAUSED",
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "id": ad_id })))
        .expect(1)
        .mount(&server)
        .await;

    // Step 3: fetch the created ad.
    Mock::given(method("GET"))
        .and(path(format!("/{ad_id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": ad_id,
            "adset_id": adset_id,
            "name": "Boost — 123456789_987654321",
            "status": "PAUSED",
            "creative": { "id": creative_id },
            "created_time": "2026-06-09T12:00:00+0000",
            "updated_time": null
        })))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let input = PromotePostInput {
        adset_id: adset_id.into(),
        name: None,
        extra: None,
    };

    let ad = provider
        .promote_post(&PostId(post_id.into()), &input)
        .await
        .expect("promote_post should succeed");

    assert_eq!(ad.id.0, ad_id);
    assert_eq!(ad.provider, "meta");
    assert_eq!(ad.adset_id.0, adset_id);
    assert_eq!(ad.status, AdStatus::Paused);
    assert_eq!(
        ad.creative_id.as_ref().map(|c| c.0.as_str()),
        Some(creative_id),
        "creative id should be mapped from the ad's creative field"
    );
}

/// Verify a custom ad name is forwarded to the ads endpoint.
#[tokio::test]
async fn test_promote_post_forwards_custom_name() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/act_{AD_ACCOUNT_ID}/adcreatives")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "id": "c1" })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path(format!("/act_{AD_ACCOUNT_ID}/ads")))
        .and(body_partial_json(serde_json::json!({
            "name": "My Custom Boost",
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "id": "a1" })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/a1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "a1",
            "adset_id": "s1",
            "name": "My Custom Boost",
            "status": "PAUSED",
            "created_time": "2026-06-09T12:00:00+0000"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let input = PromotePostInput {
        adset_id: "s1".into(),
        name: Some("My Custom Boost".into()),
        extra: None,
    };

    let ad = provider
        .promote_post(&PostId("p1".into()), &input)
        .await
        .expect("promote_post should succeed");
    assert_eq!(ad.name, "My Custom Boost");
}

/// Verify an API error during creative creation is propagated.
#[tokio::test]
async fn test_promote_post_propagates_creative_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/act_{AD_ACCOUNT_ID}/adcreatives")))
        .respond_with(ResponseTemplate::new(400).set_body_json(fixtures::api_error_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let input = PromotePostInput {
        adset_id: "s1".into(),
        name: None,
        extra: None,
    };

    let err = provider
        .promote_post(&PostId("p1".into()), &input)
        .await
        .expect_err("should propagate API error");
    assert!(err.to_string().contains("400"), "got: {err}");
}

// ── add_users_to_audience ─────────────────────────────────────────────────────

/// Verify users are normalized, SHA-256 hashed, and posted to
/// `/{audience_id}/users` with a consistent multi-column schema.
#[tokio::test]
async fn test_add_users_hashes_and_uploads() {
    let server = MockServer::start().await;

    let audience_id = "23842000001";
    let expected_email_hash = pii::sha256_hex("john.doe@example.com");
    let expected_phone_hash = pii::sha256_hex("15551234567");

    Mock::given(method("POST"))
        .and(path(format!("/{audience_id}/users")))
        .and(body_partial_json(serde_json::json!({
            "payload": {
                "schema": ["EMAIL", "PHONE"],
                "data": [
                    [expected_email_hash, expected_phone_hash],
                    [pii::sha256_hex("jane@example.com"), ""],
                ],
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "audience_id": audience_id,
            "num_received": 2,
            "num_invalid_entries": 0,
        })))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let users = vec![
        AudienceUser {
            email: Some("  John.Doe@Example.COM ".into()),
            phone: Some("+1 (555) 123-4567".into()),
            external_id: None,
        },
        AudienceUser {
            email: Some("jane@example.com".into()),
            phone: None,
            external_id: None,
        },
    ];

    let result = provider
        .add_users_to_audience(&AudienceId(audience_id.into()), &users)
        .await
        .expect("add_users_to_audience should succeed");

    assert_eq!(result.audience_id.0, audience_id);
    assert_eq!(result.num_received, 2);
    assert_eq!(result.num_invalid, 0);
}

/// Verify an empty user list is rejected client-side without an HTTP call.
#[tokio::test]
async fn test_add_users_empty_list_is_validation_error() {
    let server = MockServer::start().await;
    let provider = make_provider(&server.uri());

    let err = provider
        .add_users_to_audience(&AudienceId("a1".into()), &[])
        .await
        .expect_err("empty list should be a validation error");
    assert!(
        err.to_string().to_lowercase().contains("user"),
        "got: {err}"
    );
}

/// Verify an API error from the users endpoint is propagated.
#[tokio::test]
async fn test_add_users_propagates_api_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/a1/users"))
        .respond_with(ResponseTemplate::new(400).set_body_json(fixtures::api_error_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let users = vec![AudienceUser {
        email: Some("x@example.com".into()),
        phone: None,
        external_id: None,
    }];

    let err = provider
        .add_users_to_audience(&AudienceId("a1".into()), &users)
        .await
        .expect_err("should propagate API error");
    assert!(err.to_string().contains("400"), "got: {err}");
}
