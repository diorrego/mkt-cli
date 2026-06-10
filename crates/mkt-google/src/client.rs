//! Low-level HTTP wrapper for the Google Ads API REST interface.
//!
//! [`GoogleClient`] handles authentication headers (`Bearer` token +
//! `developer-token` + optional `login-customer-id`), JSON parsing, error
//! mapping, and rate limiting. Higher-level logic lives in
//! [`crate::provider`].

use mkt_core::error::{MktError, Result};
use mkt_core::http::RateLimiter;
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use tracing::instrument;

use crate::error::GoogleApiErrorResponse;

/// Google Ads API version used in the REST base URL.
const API_VERSION: &str = "v24";

/// Maximum concurrent requests (semaphore permits).
const MAX_CONCURRENT: usize = 100;

/// Low-level client for the Google Ads REST API.
#[derive(Debug)]
pub struct GoogleClient {
    http: Client,
    base_url: String,
    access_token: SecretString,
    developer_token: String,
    customer_id: String,
    login_customer_id: Option<String>,
    rate_limiter: RateLimiter,
}

impl GoogleClient {
    /// Create a new client for the given customer account.
    ///
    /// `customer_id` and `login_customer_id` accept dashed form
    /// (`123-456-7890`); dashes are stripped.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying HTTP client cannot be built.
    pub fn new(
        access_token: SecretString,
        developer_token: String,
        customer_id: &str,
        login_customer_id: Option<&str>,
    ) -> Result<Self> {
        let base_url = format!("https://googleads.googleapis.com/{API_VERSION}/");
        Self::new_with_base_url(
            access_token,
            developer_token,
            customer_id,
            login_customer_id,
            base_url,
        )
    }

    /// Create a new client with a custom base URL (e.g. for wiremock tests).
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying HTTP client cannot be built.
    pub fn new_with_base_url(
        access_token: SecretString,
        developer_token: String,
        customer_id: &str,
        login_customer_id: Option<&str>,
        base_url: String,
    ) -> Result<Self> {
        let http = mkt_core::http::build_http_client(None)?;
        Ok(Self {
            http,
            base_url,
            access_token,
            developer_token,
            customer_id: customer_id.replace('-', ""),
            login_customer_id: login_customer_id.map(|id| id.replace('-', "")),
            rate_limiter: RateLimiter::new(MAX_CONCURRENT),
        })
    }

    /// The customer ID (dashes stripped).
    pub fn customer_id(&self) -> &str {
        &self.customer_id
    }

    /// Run a GAQL query through `googleAds:search`.
    ///
    /// # Errors
    ///
    /// Returns [`MktError::ApiError`] for non-2xx responses and
    /// [`MktError::Http`] for transport failures.
    #[instrument(skip(self, query), fields(provider = "google"))]
    pub async fn search(&self, query: &str) -> Result<serde_json::Value> {
        let path = format!("customers/{}/googleAds:search", self.customer_id);
        let body = serde_json::json!({ "query": query });
        self.post(&path, &body).await
    }

    /// Send mutate operations to a resource endpoint, e.g.
    /// `mutate("campaigns", ops)` posts to `customers/{cid}/campaigns:mutate`.
    ///
    /// # Errors
    ///
    /// Returns [`MktError::ApiError`] for non-2xx responses and
    /// [`MktError::Http`] for transport failures.
    #[instrument(skip(self, operations), fields(provider = "google"))]
    pub async fn mutate(
        &self,
        resource: &str,
        operations: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let path = format!("customers/{}/{resource}:mutate", self.customer_id);
        let body = serde_json::json!({ "operations": operations });
        self.post(&path, &body).await
    }

    /// Perform an authenticated POST request with a JSON body.
    async fn post(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        self.rate_limiter.acquire(1).await?;
        let url = format!("{}{path}", self.base_url);

        let mut request = self
            .http
            .post(&url)
            .bearer_auth(self.access_token.expose_secret())
            .header("developer-token", &self.developer_token)
            .json(body);

        if let Some(login_id) = &self.login_customer_id {
            request = request.header("login-customer-id", login_id);
        }

        let response = request.send().await?;
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

        if let Ok(api_err) = serde_json::from_str::<GoogleApiErrorResponse>(&body) {
            return Err(api_err.into_mkt_error(status));
        }

        Err(MktError::ApiError {
            provider: "google".into(),
            status,
            message: body,
            retry_after: None,
        })
    }
}
