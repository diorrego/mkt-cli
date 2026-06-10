//! Error mapping for Google Ads API REST responses.

use mkt_core::error::MktError;
use serde::Deserialize;

/// The standard Google API error envelope: `{"error": {...}}`.
#[derive(Debug, Deserialize)]
pub struct GoogleApiErrorResponse {
    /// The inner error object.
    pub error: GoogleApiError,
}

/// The inner error object of a Google API error response.
#[derive(Debug, Deserialize)]
pub struct GoogleApiError {
    /// Numeric HTTP-like code (e.g. 401).
    #[serde(default)]
    pub code: u16,
    /// Human-readable message.
    #[serde(default)]
    pub message: String,
    /// Canonical status string (e.g. `UNAUTHENTICATED`).
    #[serde(default)]
    pub status: String,
}

impl GoogleApiErrorResponse {
    /// Convert into a unified [`MktError`], preferring the HTTP transport
    /// status over the body's `code` when they disagree.
    pub fn into_mkt_error(self, http_status: u16) -> MktError {
        let status = if http_status == 0 {
            self.error.code
        } else {
            http_status
        };
        let message = if self.error.status.is_empty() {
            self.error.message
        } else {
            format!("{} — {}", self.error.status, self.error.message)
        };
        MktError::ApiError {
            provider: "google".into(),
            status,
            message,
            retry_after: None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_google_api_error() {
        let body = serde_json::json!({
            "error": {
                "code": 401,
                "message": "Request had invalid authentication credentials.",
                "status": "UNAUTHENTICATED"
            }
        });
        let parsed: GoogleApiErrorResponse =
            serde_json::from_value(body).expect("should deserialize");
        assert_eq!(parsed.error.code, 401);
        assert_eq!(parsed.error.status, "UNAUTHENTICATED");
    }

    #[test]
    fn into_mkt_error_preserves_status_and_message() {
        let resp = GoogleApiErrorResponse {
            error: GoogleApiError {
                code: 401,
                message: "bad token".into(),
                status: "UNAUTHENTICATED".into(),
            },
        };
        let err = resp.into_mkt_error(401);
        let msg = err.to_string();
        assert!(msg.contains("google"));
        assert!(msg.contains("401"));
        assert!(msg.contains("UNAUTHENTICATED"));
    }
}
