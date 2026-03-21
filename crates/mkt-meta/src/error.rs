//! Meta Graph API error types.
//!
//! Provides deserialization for the standard Graph API error envelope and
//! conversion into the workspace-wide [`MktError`].

use mkt_core::error::MktError;
use serde::Deserialize;

/// A single error object returned by the Graph API.
///
/// Fields beyond `message` are retained for diagnostic logging and
/// are populated during deserialization of the Graph API response.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct GraphApiError {
    /// Human-readable error description.
    pub message: String,
    /// Error category (e.g. `"OAuthException"`).
    #[serde(rename = "type")]
    pub error_type: String,
    /// Numeric error code.
    pub code: i32,
    /// Optional sub-code for finer-grained classification.
    pub error_subcode: Option<i32>,
    /// Facebook trace ID for support requests.
    pub fbtrace_id: Option<String>,
}

/// The top-level error envelope returned by the Graph API.
///
/// ```json
/// {"error":{"message":"...","type":"OAuthException","code":190}}
/// ```
#[derive(Debug, Deserialize)]
pub struct GraphApiErrorResponse {
    /// The nested error payload.
    pub error: GraphApiError,
}

impl GraphApiErrorResponse {
    /// Convert this Graph API error into an [`MktError::ApiError`].
    pub fn into_mkt_error(self, status: u16) -> MktError {
        let fbtrace = self.error.fbtrace_id.as_deref().unwrap_or("none");
        MktError::ApiError {
            provider: "meta".into(),
            status,
            message: format!(
                "{} (code={}, fbtrace_id={})",
                self.error.message, self.error.code, fbtrace
            ),
            retry_after: None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_graph_api_error() {
        let json = r#"{
            "error": {
                "message": "Invalid OAuth access token.",
                "type": "OAuthException",
                "code": 190,
                "error_subcode": 463,
                "fbtrace_id": "abc123"
            }
        }"#;
        let resp: GraphApiErrorResponse = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(resp.error.code, 190);
        assert_eq!(resp.error.error_subcode, Some(463));
        assert_eq!(resp.error.fbtrace_id.as_deref(), Some("abc123"));
    }

    #[test]
    fn into_mkt_error_preserves_fields() {
        let resp = GraphApiErrorResponse {
            error: GraphApiError {
                message: "Token expired".into(),
                error_type: "OAuthException".into(),
                code: 190,
                error_subcode: None,
                fbtrace_id: None,
            },
        };
        let err = resp.into_mkt_error(401);
        let msg = err.to_string();
        assert!(msg.contains("meta"));
        assert!(msg.contains("401"));
        assert!(msg.contains("Token expired"));
    }
}
