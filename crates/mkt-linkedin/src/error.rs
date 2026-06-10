//! Error mapping for LinkedIn Marketing API responses.

use mkt_core::error::MktError;
use serde::Deserialize;

/// The LinkedIn error response shape.
///
/// All fields except `message` are optional in practice; `code` is a
/// constant like `REVOKED_ACCESS_TOKEN` and `serviceErrorCode` a numeric
/// LinkedIn-internal code.
#[derive(Debug, Deserialize)]
pub struct LinkedInErrorResponse {
    /// HTTP-like status replicated in the body.
    #[serde(default)]
    pub status: u16,
    /// Constant error code (e.g. `REVOKED_ACCESS_TOKEN`).
    #[serde(default)]
    pub code: Option<String>,
    /// LinkedIn-internal numeric error code (kept for debugging/forensics).
    #[allow(dead_code)] // part of the documented API error contract
    #[serde(default, rename = "serviceErrorCode")]
    pub service_error_code: Option<i64>,
    /// Human-readable message.
    #[serde(default)]
    pub message: String,
}

impl LinkedInErrorResponse {
    /// Convert into a unified [`MktError`], preferring the HTTP transport
    /// status over the body's `status` when they disagree.
    pub fn into_mkt_error(self, http_status: u16) -> MktError {
        let status = if http_status == 0 {
            self.status
        } else {
            http_status
        };
        let message = match self.code {
            Some(code) => format!("{code} — {}", self.message),
            None => self.message,
        };
        MktError::ApiError {
            provider: "linkedin".into(),
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
    fn deserialize_linkedin_error() {
        let body = serde_json::json!({
            "status": 401,
            "serviceErrorCode": 65600,
            "code": "REVOKED_ACCESS_TOKEN",
            "message": "The token used in the request has been revoked by the user"
        });
        let parsed: LinkedInErrorResponse =
            serde_json::from_value(body).expect("should deserialize");
        assert_eq!(parsed.status, 401);
        assert_eq!(parsed.code.as_deref(), Some("REVOKED_ACCESS_TOKEN"));
        assert_eq!(parsed.service_error_code, Some(65600));
    }

    #[test]
    fn deserialize_minimal_error() {
        let body = serde_json::json!({ "message": "oops" });
        let parsed: LinkedInErrorResponse =
            serde_json::from_value(body).expect("all other fields optional");
        assert!(parsed.code.is_none());
    }

    #[test]
    fn into_mkt_error_includes_code() {
        let resp = LinkedInErrorResponse {
            status: 401,
            code: Some("REVOKED_ACCESS_TOKEN".into()),
            service_error_code: Some(65600),
            message: "revoked".into(),
        };
        let err = resp.into_mkt_error(401);
        let msg = err.to_string();
        assert!(msg.contains("linkedin"));
        assert!(msg.contains("401"));
        assert!(msg.contains("REVOKED_ACCESS_TOKEN"));
    }
}
