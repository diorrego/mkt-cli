#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]
//! Wiremock-based integration tests for the LinkedIn OAuth
//! refresh-token exchange.

use secrecy::{ExposeSecret, SecretString};
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const CLIENT_ID: &str = "test-client-id";
const CLIENT_SECRET: &str = "test-client-secret";
const REFRESH_TOKEN: &str = "test-refresh-token";

fn token_url(server: &MockServer) -> String {
    format!("{}/oauth/v2/accessToken", server.uri())
}

/// Verify the refresh exchange posts all four required form params and
/// returns the access token from the response.
#[tokio::test]
async fn test_refresh_exchange_success() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/oauth/v2/accessToken"))
        .and(header("content-type", "application/x-www-form-urlencoded"))
        .and(body_string_contains("grant_type=refresh_token"))
        .and(body_string_contains("refresh_token=test-refresh-token"))
        .and(body_string_contains("client_id=test-client-id"))
        .and(body_string_contains("client_secret=test-client-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "AQXNnd2kXITHmSxB",
            "expires_in": 86_400,
            "refresh_token": "AQWAft_WjYZKwuWXLC5hQlghgTam",
            "refresh_token_expires_in": 439_200,
            "scope": "r_ads,rw_ads"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let token = mkt_linkedin::refresh_access_token(
        CLIENT_ID,
        &SecretString::from(CLIENT_SECRET.to_string()),
        &SecretString::from(REFRESH_TOKEN.to_string()),
        &token_url(&server),
    )
    .await
    .expect("token exchange should succeed");

    assert_eq!(token.expose_secret(), "AQXNnd2kXITHmSxB");
}

/// Verify a 400 from the token endpoint maps to an actionable
/// `AuthError` mentioning MDP approval and the re-authorization
/// fallback.
#[tokio::test]
async fn test_refresh_invalid_grant_maps_to_auth_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/oauth/v2/accessToken"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": "invalid_request",
            "error_description": "The provided authorization grant or refresh token is \
                                  invalid, expired or revoked"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let err = mkt_linkedin::refresh_access_token(
        CLIENT_ID,
        &SecretString::from(CLIENT_SECRET.to_string()),
        &SecretString::from(REFRESH_TOKEN.to_string()),
        &token_url(&server),
    )
    .await
    .expect_err("400 should be an error");

    let msg = err.to_string();
    assert!(msg.contains("linkedin"), "got: {msg}");
    assert!(
        msg.contains("invalid, expired or revoked"),
        "should surface the endpoint detail, got: {msg}"
    );
    assert!(
        msg.contains("MDP") || msg.contains("re-author"),
        "should mention MDP approval or re-authorization, got: {msg}"
    );
    assert_eq!(err.exit_code(), 3, "400 should map to auth exit code");
}

/// Verify a 200 response without an `access_token` field yields a clean
/// error instead of a panic.
#[tokio::test]
async fn test_refresh_response_missing_access_token() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/oauth/v2/accessToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "expires_in": 86_400
        })))
        .expect(1)
        .mount(&server)
        .await;

    let err = mkt_linkedin::refresh_access_token(
        CLIENT_ID,
        &SecretString::from(CLIENT_SECRET.to_string()),
        &SecretString::from(REFRESH_TOKEN.to_string()),
        &token_url(&server),
    )
    .await
    .expect_err("missing access_token should be an error");

    let msg = err.to_string();
    assert!(msg.contains("access_token"), "got: {msg}");
}
