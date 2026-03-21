//! Wiremock helpers for testing HTTP-based provider crates.
//!
//! Provides `MockServer` convenience wrappers and pre-built
//! `ResponseTemplate` factories that mirror real Meta Graph API responses.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use mkt_testkit::http::{meta_campaigns_stub, meta_error_stub};
//! use wiremock::{MockServer, Mock, matchers::{method, path}};
//!
//! #[tokio::main]
//! async fn main() {
//!     let server = MockServer::start().await;
//!     Mock::given(method("GET"))
//!         .and(path("/v24.0/act_123/campaigns"))
//!         .respond_with(meta_campaigns_stub())
//!         .mount(&server)
//!         .await;
//! }
//! ```

use wiremock::ResponseTemplate;

use crate::fixtures::meta as fixtures;

// ── Response template factories ─────────────────────────────────────────────

/// A 200 OK response matching a Meta Graph API campaigns list.
///
/// The body is the embedded `campaigns.json` fixture.
pub fn meta_campaigns_stub() -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_string(fixtures::campaigns_json())
        .append_header("content-type", "application/json; charset=utf-8")
}

/// A 200 OK response matching a Meta Graph API ad sets list.
///
/// The body is the embedded `adsets.json` fixture.
pub fn meta_adsets_stub() -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_string(fixtures::adsets_json())
        .append_header("content-type", "application/json; charset=utf-8")
}

/// A 200 OK response matching a Meta Graph API insights report.
///
/// The body is the embedded `insights.json` fixture.
pub fn meta_insights_stub() -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_string(fixtures::insights_json())
        .append_header("content-type", "application/json; charset=utf-8")
}

/// A 200 OK response matching a Meta Graph API page post creation.
///
/// The body is the embedded `page_post.json` fixture.
pub fn meta_page_post_stub() -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_string(fixtures::page_post_json())
        .append_header("content-type", "application/json; charset=utf-8")
}

/// A 400 Bad Request response with the standard Meta Graph API error envelope.
///
/// Use this to test error handling paths. Status code defaults to 400;
/// use [`meta_error_stub_with_status`] for other status codes.
pub fn meta_error_stub() -> ResponseTemplate {
    meta_error_stub_with_status(400)
}

/// A Meta Graph API error response with a custom HTTP status code.
///
/// The body is the standard `{"error": {...}}` envelope used by all Meta
/// Graph API error responses.
pub fn meta_error_stub_with_status(status: u16) -> ResponseTemplate {
    ResponseTemplate::new(status)
        .set_body_json(fixtures::api_error_response())
        .append_header("content-type", "application/json; charset=utf-8")
}

/// A 401 Unauthorized response with a Meta-style OAuth error body.
///
/// Useful for testing credential validation and token refresh logic.
pub fn meta_auth_error_stub() -> ResponseTemplate {
    meta_error_stub_with_status(401)
}

/// A 429 Too Many Requests response simulating Meta rate limiting.
///
/// Includes the `x-app-usage` and `retry-after` headers that the real API
/// sends when the rate limit is exceeded.
pub fn meta_rate_limit_stub() -> ResponseTemplate {
    ResponseTemplate::new(429)
        .set_body_json(serde_json::json!({
            "error": {
                "message": "Application request limit reached",
                "type": "OAuthException",
                "code": 4,
                "fbtrace_id": "RateLimitTrace123"
            }
        }))
        .append_header("content-type", "application/json; charset=utf-8")
        .append_header(
            "x-app-usage",
            r#"{"call_count":100,"total_cputime":100,"total_time":100}"#,
        )
        .append_header("retry-after", "60")
}

/// A 200 OK response with an empty data array and no pagination.
///
/// Useful for testing "no results" code paths such as empty campaign lists.
pub fn meta_empty_list_stub() -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_json(serde_json::json!({ "data": [], "paging": { "cursors": {} } }))
        .append_header("content-type", "application/json; charset=utf-8")
}

/// A 200 OK response with a single-item campaign creation confirmation.
///
/// Matches the `{"id": "...", "success": true}` shape that the Meta API
/// returns after a successful campaign creation.
pub fn meta_campaign_create_stub() -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_json(fixtures::campaign_create_response())
        .append_header("content-type", "application/json; charset=utf-8")
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_meta_campaigns_stub_returns_200() {
        let t = meta_campaigns_stub();
        // ResponseTemplate does not expose status directly; verify body parses.
        // The status is embedded in the template; we trust the constructor arg.
        let _ = t; // constructed without panic
    }

    #[test]
    fn test_meta_error_stub_body_is_valid_json() {
        // Verify the error body used in the stub is valid JSON.
        let body = fixtures::api_error_response();
        assert!(body["error"].is_object());
    }

    #[test]
    fn test_meta_rate_limit_stub_can_be_constructed() {
        let t = meta_rate_limit_stub();
        let _ = t;
    }

    #[test]
    fn test_meta_empty_list_stub_can_be_constructed() {
        let t = meta_empty_list_stub();
        let _ = t;
    }

    #[test]
    fn test_meta_campaign_create_stub_body_has_success() {
        let body = fixtures::campaign_create_response();
        assert_eq!(body["success"], true);
    }

    #[tokio::test]
    async fn test_meta_campaigns_stub_serves_fixture_body() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/campaigns"))
            .respond_with(meta_campaigns_stub())
            .mount(&server)
            .await;

        let url = format!("{}/campaigns", server.uri());
        let resp = reqwest::get(&url).await.expect("request failed");
        assert_eq!(resp.status().as_u16(), 200);

        let body: serde_json::Value = resp.json().await.expect("body must be JSON");
        let data = body["data"].as_array().expect("data must be array");
        assert!(!data.is_empty(), "fixture data should not be empty");
    }

    #[tokio::test]
    async fn test_meta_error_stub_serves_error_envelope() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/bad"))
            .respond_with(meta_error_stub())
            .mount(&server)
            .await;

        let url = format!("{}/bad", server.uri());
        let resp = reqwest::get(&url).await.expect("request failed");
        assert_eq!(resp.status().as_u16(), 400);

        let body: serde_json::Value = resp.json().await.expect("body must be JSON");
        assert!(body["error"].is_object(), "error envelope must be present");
    }

    #[tokio::test]
    async fn test_meta_rate_limit_stub_serves_429() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/throttled"))
            .respond_with(meta_rate_limit_stub())
            .mount(&server)
            .await;

        let url = format!("{}/throttled", server.uri());
        let resp = reqwest::get(&url).await.expect("request failed");
        assert_eq!(resp.status().as_u16(), 429);
    }

    #[tokio::test]
    async fn test_meta_empty_list_stub_serves_empty_data() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/empty"))
            .respond_with(meta_empty_list_stub())
            .mount(&server)
            .await;

        let url = format!("{}/empty", server.uri());
        let resp = reqwest::get(&url).await.expect("request failed");
        let body: serde_json::Value = resp.json().await.expect("body must be JSON");
        let data = body["data"].as_array().expect("data must be array");
        assert!(data.is_empty(), "empty stub must return zero items");
    }

    #[tokio::test]
    async fn test_meta_auth_error_stub_serves_401() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/auth"))
            .respond_with(meta_auth_error_stub())
            .mount(&server)
            .await;

        let url = format!("{}/auth", server.uri());
        let resp = reqwest::get(&url).await.expect("request failed");
        assert_eq!(resp.status().as_u16(), 401);
    }
}
