#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]
//! Wiremock-based integration tests for Meta audience listing, image
//! upload, and creative validation against the v25.0 Graph API contract.

use mkt_core::models::{CreateCreativeInput, UploadImageInput};
use mkt_core::provider::MarketingProvider;
use mkt_meta::{MetaClient, MetaProvider};
use secrecy::SecretString;
use wiremock::matchers::{method, path, query_param_contains};
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

// ── audiences ─────────────────────────────────────────────────────────────────

/// v25.0 removed `approximate_count`: audience reads must request the
/// `approximate_count_lower_bound`/`_upper_bound` fields and map the size
/// from them.
#[tokio::test]
async fn test_list_audiences_uses_count_bounds_fields() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/act_{AD_ACCOUNT_ID}/customaudiences")))
        .and(query_param_contains(
            "fields",
            "approximate_count_lower_bound",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "23842000001",
                "name": "Lookalike — Purchasers",
                "subtype": "LOOKALIKE",
                "approximate_count_lower_bound": 15000,
                "approximate_count_upper_bound": 18000
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let audiences = provider
        .list_audiences()
        .await
        .expect("list_audiences should succeed");

    assert_eq!(audiences.len(), 1);
    assert_eq!(
        audiences[0].size,
        Some(15000),
        "size should map from approximate_count_lower_bound"
    );

    // The removed field must not be requested: the API answers error #100
    // for nonexistent fields.
    let requests = server.received_requests().await.unwrap();
    let fields = requests[0]
        .url
        .query_pairs()
        .find(|(k, _)| k == "fields")
        .map(|(_, v)| v.into_owned())
        .unwrap_or_default();
    assert!(
        !fields.split(',').any(|field| field == "approximate_count"),
        "approximate_count no longer exists in v25.0: {fields}"
    );
}

// ── upload_image ──────────────────────────────────────────────────────────────

/// The v25.0 adimages edge accepts `bytes` (Base64) or `copy_from` — not
/// `url`. Importing from a URL therefore downloads the image and uploads
/// its bytes.
#[tokio::test]
async fn test_upload_image_downloads_url_and_sends_bytes() {
    let server = MockServer::start().await;
    let image_bytes: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    Mock::given(method("GET"))
        .and(path("/assets/pixel.png"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(image_bytes)
                .insert_header("content-type", "image/png"),
        )
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path(format!("/act_{AD_ACCOUNT_ID}/adimages")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "images": { "pixel.png": { "hash": "abc123hash", "url": "https://cdn.example/pixel.png" } }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let provider = make_provider(&server.uri());
    let input = UploadImageInput {
        file_path: None,
        url: Some(format!("{}/assets/pixel.png", server.uri())),
        name: Some("pixel".into()),
    };
    let asset = provider
        .upload_image(&input)
        .await
        .expect("upload_image should succeed");
    assert_eq!(asset.id.0, "abc123hash");

    let requests = server.received_requests().await.unwrap();
    let upload = requests
        .iter()
        .find(|r| r.url.path().ends_with("/adimages"))
        .expect("adimages request should exist");
    let body: serde_json::Value = serde_json::from_slice(&upload.body).unwrap();
    assert!(
        body.get("url").is_none(),
        "the adimages edge has no url parameter: {body}"
    );
    use base64::Engine as _;
    assert_eq!(
        body["bytes"].as_str().unwrap_or_default(),
        base64::engine::general_purpose::STANDARD.encode(image_bytes),
        "image bytes must upload Base64-encoded"
    );
}

// ── create_creative ───────────────────────────────────────────────────────────

/// `object_story_spec.page_id` must be a real Page ID; the provider must
/// fail fast when none is configured instead of sending the invalid
/// placeholder "me".
#[tokio::test]
async fn test_create_creative_without_page_id_is_validation_error() {
    let provider = make_provider("http://127.0.0.1:9"); // never reached

    let input = CreateCreativeInput {
        name: "Creative".into(),
        body: Some("Hello".into()),
        ..Default::default()
    };
    let err = provider
        .create_creative(&input)
        .await
        .expect_err("creating a creative without a page_id must fail");
    assert!(
        matches!(err, mkt_core::error::MktError::ValidationError { .. }),
        "expected ValidationError, got: {err:?}"
    );
}
