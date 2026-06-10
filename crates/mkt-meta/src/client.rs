//! Low-level HTTP wrapper for the Meta Graph API.
//!
//! [`MetaClient`] handles authentication, JSON parsing, error mapping,
//! and rate limiting.  Higher-level logic lives in [`crate::provider`].

use mkt_core::error::{MktError, Result};
use mkt_core::http::RateLimiter;
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use tracing::instrument;

use crate::error::GraphApiErrorResponse;

/// Default Graph API version.
const DEFAULT_API_VERSION: &str = "v25.0";

/// Maximum concurrent requests (semaphore permits).
const MAX_CONCURRENT: usize = 200;

/// Low-level client for the Meta Graph API.
///
/// Wraps `reqwest::Client` with Bearer token auth, JSON error parsing,
/// and basic rate limiting.
#[derive(Debug)]
pub struct MetaClient {
    http: Client,
    base_url: String,
    access_token: SecretString,
    ad_account_id: String,
    rate_limiter: RateLimiter,
}

impl MetaClient {
    /// Create a new client for the given ad account.
    ///
    /// Uses the default base URL `https://graph.facebook.com/{version}/`.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying HTTP client cannot be built.
    pub fn new(
        access_token: SecretString,
        ad_account_id: String,
        api_version: Option<&str>,
    ) -> Result<Self> {
        let version = api_version.unwrap_or(DEFAULT_API_VERSION);
        let base_url = format!("https://graph.facebook.com/{version}/");
        Self::new_with_base_url(access_token, ad_account_id, base_url)
    }

    /// Create a new client with a custom base URL (e.g. for wiremock tests).
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying HTTP client cannot be built.
    pub fn new_with_base_url(
        access_token: SecretString,
        ad_account_id: String,
        base_url: String,
    ) -> Result<Self> {
        let http = mkt_core::http::build_http_client(None)?;
        Ok(Self {
            http,
            base_url,
            access_token,
            ad_account_id,
            rate_limiter: RateLimiter::new(MAX_CONCURRENT),
        })
    }

    /// The ad account ID (without the `act_` prefix).
    pub fn ad_account_id(&self) -> &str {
        &self.ad_account_id
    }

    /// Perform an authenticated GET request.
    ///
    /// # Errors
    ///
    /// Returns [`MktError::ApiError`] for non-2xx responses and
    /// [`MktError::Http`] for transport failures.
    #[instrument(skip(self), fields(provider = "meta"))]
    pub async fn get(&self, path: &str, params: &[(&str, &str)]) -> Result<serde_json::Value> {
        self.rate_limiter.acquire(1).await?;
        let url = format!("{}{path}", self.base_url);

        let response = self
            .http
            .get(&url)
            .bearer_auth(self.access_token.expose_secret())
            .query(params)
            .send()
            .await?;

        Self::parse_response(response).await
    }

    /// Perform an authenticated POST request with a JSON body.
    ///
    /// # Errors
    ///
    /// Returns [`MktError::ApiError`] for non-2xx responses and
    /// [`MktError::Http`] for transport failures.
    #[instrument(skip(self, body), fields(provider = "meta"))]
    pub async fn post(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        self.rate_limiter.acquire(3).await?;
        let url = format!("{}{path}", self.base_url);

        let response = self
            .http
            .post(&url)
            .bearer_auth(self.access_token.expose_secret())
            .json(body)
            .send()
            .await?;

        Self::parse_response(response).await
    }

    /// Perform an authenticated DELETE request.
    ///
    /// # Errors
    ///
    /// Returns [`MktError::ApiError`] for non-2xx responses and
    /// [`MktError::Http`] for transport failures.
    #[instrument(skip(self), fields(provider = "meta"))]
    pub async fn delete(&self, path: &str) -> Result<serde_json::Value> {
        self.rate_limiter.acquire(3).await?;
        let url = format!("{}{path}", self.base_url);

        let response = self
            .http
            .delete(&url)
            .bearer_auth(self.access_token.expose_secret())
            .send()
            .await?;

        Self::parse_response(response).await
    }

    /// Parse a response, returning either the JSON body or a mapped error.
    async fn parse_response(response: reqwest::Response) -> Result<serde_json::Value> {
        let status = response.status().as_u16();
        let body = response.text().await?;

        if (200..300).contains(&status) {
            let value: serde_json::Value = serde_json::from_str(&body)?;
            return Ok(value);
        }

        // Try to parse as a Graph API error envelope.
        if let Ok(api_err) = serde_json::from_str::<GraphApiErrorResponse>(&body) {
            return Err(api_err.into_mkt_error(status));
        }

        // Fall back to a generic API error.
        Err(MktError::ApiError {
            provider: "meta".into(),
            status,
            message: body,
            retry_after: None,
        })
    }
}
