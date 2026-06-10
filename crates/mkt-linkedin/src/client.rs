//! Low-level HTTP wrapper for the LinkedIn Marketing API.
//!
//! [`LinkedInClient`] handles the versioned REST headers
//! (`Linkedin-Version`, `X-Restli-Protocol-Version`), Rest.li 2.0 query
//! strings (which must NOT be fully URL-encoded — structural characters
//! like `(`, `)`, `,` and `:` stay literal while URN colons are `%3A`),
//! JSON parsing, error mapping, and rate limiting.

use mkt_core::error::{MktError, Result};
use mkt_core::http::RateLimiter;
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use tracing::instrument;

use crate::error::LinkedInErrorResponse;

/// The LinkedIn API version sent in the `Linkedin-Version` header
/// (YYYYMM; versions are supported for at least one year).
const LINKEDIN_VERSION: &str = "202605";

/// Maximum concurrent requests (semaphore permits).
const MAX_CONCURRENT: usize = 100;

/// A successful write response: LinkedIn creates return the new entity ID
/// in the `x-restli-id` response header, with an empty body.
#[derive(Debug)]
pub struct WriteResponse {
    /// Parsed JSON body (empty object for 201/204 responses without one).
    pub body: serde_json::Value,
    /// Value of the `x-restli-id` header, if present.
    pub restli_id: Option<String>,
}

/// Low-level client for the LinkedIn Marketing REST API.
#[derive(Debug)]
pub struct LinkedInClient {
    http: Client,
    base_url: String,
    access_token: SecretString,
    ad_account_id: String,
    rate_limiter: RateLimiter,
}

impl LinkedInClient {
    /// Create a new client for the given ad account.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying HTTP client cannot be built.
    pub fn new(access_token: SecretString, ad_account_id: String) -> Result<Self> {
        Self::new_with_base_url(
            access_token,
            ad_account_id,
            "https://api.linkedin.com/rest/".to_string(),
        )
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

    /// The ad account ID (numeric, no URN prefix).
    pub fn ad_account_id(&self) -> &str {
        &self.ad_account_id
    }

    /// Perform a GET request with a raw, pre-built Rest.li query string.
    ///
    /// The query string is appended verbatim — callers are responsible for
    /// Rest.li 2.0 encoding (URN colons as `%3A`, structural characters
    /// literal). Pass an empty string for no query.
    ///
    /// # Errors
    ///
    /// Returns [`MktError::ApiError`] for non-2xx responses and
    /// [`MktError::Http`] for transport failures.
    #[instrument(skip(self), fields(provider = "linkedin"))]
    pub async fn get_raw(&self, path: &str, raw_query: &str) -> Result<serde_json::Value> {
        self.rate_limiter.acquire(1).await?;
        let url = if raw_query.is_empty() {
            format!("{}{path}", self.base_url)
        } else {
            format!("{}{path}?{raw_query}", self.base_url)
        };

        let response = self
            .http
            .get(&url)
            .bearer_auth(self.access_token.expose_secret())
            .header("Linkedin-Version", LINKEDIN_VERSION)
            .header("X-Restli-Protocol-Version", "2.0.0")
            .send()
            .await?;

        Ok(Self::parse_response(response).await?.body)
    }

    /// Perform a POST request with a JSON body.
    ///
    /// `restli_method` adds an `X-RestLi-Method` header (e.g.
    /// `PARTIAL_UPDATE` for Rest.li patches).
    ///
    /// # Errors
    ///
    /// Returns [`MktError::ApiError`] for non-2xx responses and
    /// [`MktError::Http`] for transport failures.
    #[instrument(skip(self, body), fields(provider = "linkedin"))]
    pub async fn post(
        &self,
        path: &str,
        body: &serde_json::Value,
        restli_method: Option<&str>,
    ) -> Result<WriteResponse> {
        self.rate_limiter.acquire(3).await?;
        let url = format!("{}{path}", self.base_url);

        let mut request = self
            .http
            .post(&url)
            .bearer_auth(self.access_token.expose_secret())
            .header("Linkedin-Version", LINKEDIN_VERSION)
            .header("X-Restli-Protocol-Version", "2.0.0")
            .json(body);

        if let Some(method) = restli_method {
            request = request.header("X-RestLi-Method", method);
        }

        let response = request.send().await?;
        Self::parse_response(response).await
    }

    /// Parse a response into a [`WriteResponse`] or a mapped error.
    async fn parse_response(response: reqwest::Response) -> Result<WriteResponse> {
        let status = response.status().as_u16();
        let restli_id = response
            .headers()
            .get("x-restli-id")
            .and_then(|v| v.to_str().ok())
            .map(String::from);
        let body_text = response.text().await?;

        if (200..300).contains(&status) {
            let body = if body_text.trim().is_empty() {
                serde_json::Value::Object(serde_json::Map::new())
            } else {
                serde_json::from_str(&body_text)?
            };
            return Ok(WriteResponse { body, restli_id });
        }

        if let Ok(api_err) = serde_json::from_str::<LinkedInErrorResponse>(&body_text) {
            return Err(api_err.into_mkt_error(status));
        }

        Err(MktError::ApiError {
            provider: "linkedin".into(),
            status,
            message: body_text,
            retry_after: None,
        })
    }
}
