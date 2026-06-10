//! Error mapping for TikTok Business API v1.3 envelope codes.
//!
//! TikTok returns HTTP 200 even on logical errors; the envelope's `code`
//! field is authoritative (`0` = OK). This module maps non-zero codes to
//! unified [`MktError`] variants.

use mkt_core::error::MktError;

/// Envelope code for rate limiting (HTTP stays 200).
pub const CODE_RATE_LIMITED: i64 = 40100;

/// Default backoff hint in seconds when TikTok rate-limits (the API sends
/// no `Retry-After`; its window is roughly one minute).
const RATE_LIMIT_BACKOFF_SECS: u64 = 60;

/// Map a non-zero TikTok envelope code to a unified [`MktError`].
///
/// Code ranges (per the official error list):
/// - `40100` — app-level rate limit (transient)
/// - `40101..=40114` — authentication / token errors
/// - `40002` — resource not accessible or does not exist
/// - `40001` — invalid parameter or missing permission
/// - `5xxxx` / `60001` — TikTok-internal errors (transient)
pub fn envelope_code_to_error(code: i64, message: &str) -> MktError {
    match code {
        CODE_RATE_LIMITED => MktError::RateLimited {
            provider: "tiktok".into(),
            retry_after_secs: RATE_LIMIT_BACKOFF_SECS,
        },
        40101..=40114 => MktError::auth_error("tiktok", message),
        40002 => MktError::ApiError {
            provider: "tiktok".into(),
            status: 404,
            message: format!("code 40002 — {message}"),
            retry_after: None,
        },
        50000..=59999 | 60001 => MktError::ApiError {
            provider: "tiktok".into(),
            status: 503,
            message: format!("code {code} — {message}"),
            retry_after: None,
        },
        _ => MktError::ApiError {
            provider: "tiktok".into(),
            status: 400,
            message: format!("code {code} — {message}"),
            retry_after: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_code_maps_to_rate_limited() {
        let err = envelope_code_to_error(40100, "too many requests");
        assert_eq!(err.exit_code(), 5);
        assert!(err.is_transient());
    }

    #[test]
    fn auth_codes_map_to_auth_error() {
        let err = envelope_code_to_error(40105, "token revoked");
        assert_eq!(err.exit_code(), 3);
        let err = envelope_code_to_error(40101, "no auth");
        assert_eq!(err.exit_code(), 3);
    }

    #[test]
    fn missing_resource_maps_to_not_found() {
        let err = envelope_code_to_error(40002, "does not exist");
        assert_eq!(err.exit_code(), 4);
    }

    #[test]
    fn internal_errors_are_transient() {
        let err = envelope_code_to_error(60001, "maintenance");
        assert!(err.is_transient());
        let err = envelope_code_to_error(50001, "internal");
        assert!(err.is_transient());
    }

    #[test]
    fn parameter_errors_map_to_api_error() {
        let err = envelope_code_to_error(40001, "bad param");
        assert_eq!(err.exit_code(), 7);
        assert!(err.to_string().contains("40001"));
    }
}
