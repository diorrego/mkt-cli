//! Low-level HTTP wrapper for the TikTok Business API v1.3.
//!
//! [`TikTokClient`] handles the `Access-Token` header (TikTok does not use
//! `Authorization: Bearer`), the `{"code","message","request_id","data"}`
//! envelope (HTTP stays 200 on logical errors — `code != 0` is the error
//! signal), JSON-encoded query parameters, and rate limiting. Endpoint
//! paths require trailing slashes.

use mkt_core::error::{MktError, Result};
use mkt_core::http::{OpKind, RateLimiter, RetryPolicy, retry, retry_after_secs};
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use tracing::instrument;

use crate::error::envelope_code_to_error;

/// Client-side request budget in cells per second (reads cost 1,
/// writes 3). TikTok standard tier is ~10 QPS per app; 8 rps leaves headroom.
const REQUESTS_PER_SECOND: u32 = 8;

/// Low-level client for the TikTok Business API.
#[derive(Debug)]
pub struct TikTokClient {
    http: Client,
    base_url: String,
    access_token: SecretString,
    advertiser_id: String,
    rate_limiter: RateLimiter,
    retry: RetryPolicy,
}

impl TikTokClient {
    /// Create a new client for the given advertiser account.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying HTTP client cannot be built.
    pub fn new(access_token: SecretString, advertiser_id: String) -> Result<Self> {
        Ok(Self::new_with_base_url(
            access_token,
            advertiser_id,
            "https://business-api.tiktok.com/open_api/v1.3/".to_string(),
        )?
        .with_retry_policy(RetryPolicy::standard()))
    }

    /// Create a new client with a custom base URL (e.g. for wiremock tests).
    /// Starts with no retries so tests stay deterministic; production
    /// constructors opt in via [`Self::with_retry_policy`].
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying HTTP client cannot be built.
    pub fn new_with_base_url(
        access_token: SecretString,
        advertiser_id: String,
        base_url: String,
    ) -> Result<Self> {
        let http = mkt_core::http::build_http_client(None)?;
        Ok(Self {
            http,
            base_url,
            access_token,
            advertiser_id,
            rate_limiter: RateLimiter::new(REQUESTS_PER_SECOND),
            retry: RetryPolicy::none(),
        })
    }

    /// Replace the retry policy (reads retry transient failures, writes
    /// only rate limits and connection failures).
    #[must_use]
    pub const fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry = policy;
        self
    }

    /// The advertiser ID (numeric string).
    pub fn advertiser_id(&self) -> &str {
        &self.advertiser_id
    }

    /// Perform a GET request. `params` values that are arrays/objects must
    /// already be JSON-encoded strings (the v1.3 convention).
    ///
    /// Returns the envelope's `data` value.
    ///
    /// # Errors
    ///
    /// Returns [`MktError::ApiError`] / [`MktError::RateLimited`] /
    /// [`MktError::AuthError`] for envelope errors and [`MktError::Http`]
    /// for transport failures.
    #[instrument(skip(self), fields(provider = "tiktok"))]
    pub async fn get(&self, path: &str, params: &[(&str, &str)]) -> Result<serde_json::Value> {
        self.rate_limiter.acquire(1).await?;
        let url = format!("{}{path}", self.base_url);

        retry(&self.retry, OpKind::Read, || async {
            let response = self
                .http
                .get(&url)
                .header("Access-Token", self.access_token.expose_secret())
                .query(params)
                .send()
                .await?;
            Self::parse_response(response).await
        })
        .await
    }

    /// Perform a POST request with a JSON body. Returns the envelope's
    /// `data` value.
    ///
    /// # Errors
    ///
    /// Returns [`MktError::ApiError`] / [`MktError::RateLimited`] /
    /// [`MktError::AuthError`] for envelope errors and [`MktError::Http`]
    /// for transport failures.
    #[instrument(skip(self, body), fields(provider = "tiktok"))]
    pub async fn post(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        self.rate_limiter.acquire(3).await?;
        let url = format!("{}{path}", self.base_url);

        retry(&self.retry, OpKind::Write, || async {
            let response = self
                .http
                .post(&url)
                .header("Access-Token", self.access_token.expose_secret())
                .json(body)
                .send()
                .await?;
            Self::parse_response(response).await
        })
        .await
    }

    /// Parse a response: check the HTTP status, then the envelope `code`.
    async fn parse_response(response: reqwest::Response) -> Result<serde_json::Value> {
        let status = response.status().as_u16();
        let retry_after = retry_after_secs(response.headers());
        let body = response.text().await?;

        if status == 429 {
            return Err(MktError::RateLimited {
                provider: "tiktok".into(),
                retry_after_secs: retry_after.unwrap_or(60),
            });
        }

        if !(200..300).contains(&status) {
            return Err(MktError::ApiError {
                provider: "tiktok".into(),
                status,
                message: body,
                retry_after,
            });
        }

        let envelope: serde_json::Value = serde_json::from_str(&body)?;
        let code = envelope["code"].as_i64().unwrap_or(-1);
        if code != 0 {
            let message = envelope["message"].as_str().unwrap_or("unknown error");
            return Err(envelope_code_to_error(code, message));
        }

        Ok(envelope["data"].clone())
    }
}
