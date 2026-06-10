//! Unified error types for the `mkt` workspace.
//!
//! All provider crates convert their specific errors into [`MktError`]
//! variants. The CLI boundary uses `anyhow` for final error reporting.

use thiserror::Error;

/// The unified error type for the `mkt` workspace.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum MktError {
    /// The requested provider is not registered or not available.
    #[error("Provider '{provider}' not found. Available: {available}")]
    ProviderNotFound {
        /// The provider name that was requested.
        provider: String,
        /// Comma-separated list of available provider names.
        available: String,
    },

    /// An API request returned an error response.
    #[error("API error from {provider}: {status} — {message}")]
    ApiError {
        /// Which provider returned the error.
        provider: String,
        /// HTTP status code.
        status: u16,
        /// Human-readable error message from the API.
        message: String,
        /// Optional retry-after hint in seconds.
        retry_after: Option<u64>,
    },

    /// Authentication failed for a provider.
    #[error("Authentication failed for {provider}: {reason}")]
    AuthError {
        /// Which provider failed authentication.
        provider: String,
        /// Human-readable reason for the failure.
        reason: String,
    },

    /// A configuration error occurred.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// The provider's rate limit has been exceeded.
    #[error("Rate limit exceeded for {provider}. Retry after {retry_after_secs}s")]
    RateLimited {
        /// Which provider hit the rate limit.
        provider: String,
        /// How many seconds to wait before retrying.
        retry_after_secs: u64,
    },

    /// A validation error on user input.
    #[error("Validation error: {field} — {message}")]
    ValidationError {
        /// The field that failed validation.
        field: String,
        /// What went wrong.
        message: String,
    },

    /// The provider does not support the requested feature.
    #[error("{provider} does not support '{feature}'")]
    NotSupported {
        /// Which provider lacks the feature.
        provider: String,
        /// The feature that is not supported.
        feature: String,
    },

    /// An HTTP transport error from `reqwest`.
    #[error(transparent)]
    Http(#[from] reqwest::Error),

    /// A filesystem I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// A JSON serialization/deserialization error.
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    /// A TOML parsing error.
    #[error(transparent)]
    Toml(#[from] toml::de::Error),

    /// A CSV writing error.
    #[error(transparent)]
    Csv(#[from] csv::Error),
}

impl MktError {
    /// Convenience constructor for `NotSupported` errors.
    pub fn not_supported(provider: &str, feature: &str) -> Self {
        Self::NotSupported {
            provider: provider.to_string(),
            feature: feature.to_string(),
        }
    }

    /// Convenience constructor for `AuthError`.
    pub fn auth_error(provider: &str, reason: &str) -> Self {
        Self::AuthError {
            provider: provider.to_string(),
            reason: reason.to_string(),
        }
    }

    /// Process exit code for this error.
    ///
    /// The contract is stable and documented in `AGENTS.md` so scripts and
    /// coding agents can branch on it:
    ///
    /// | Code | Meaning                                  |
    /// |------|------------------------------------------|
    /// | 0    | success                                  |
    /// | 1    | unexpected error (I/O, transport, bug)   |
    /// | 2    | invalid input or configuration           |
    /// | 3    | authentication failed                    |
    /// | 4    | resource or provider not found           |
    /// | 5    | rate limited (transient — retry)         |
    /// | 6    | feature not supported by the provider    |
    /// | 7    | provider API rejected the request        |
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::ValidationError { .. } | Self::ConfigError(_) => 2,
            Self::AuthError { .. } => 3,
            Self::ProviderNotFound { .. } => 4,
            Self::RateLimited { .. } => 5,
            Self::NotSupported { .. } => 6,
            Self::ApiError { status, .. } => match status {
                401 | 403 => 3,
                404 => 4,
                429 => 5,
                _ => 7,
            },
            _ => 1,
        }
    }

    /// Stable machine-readable error identifier (`snake_case`).
    ///
    /// Emitted in structured error output so agents can match on the kind
    /// of failure without parsing human-readable messages.
    #[must_use]
    pub const fn error_type(&self) -> &'static str {
        match self {
            Self::ProviderNotFound { .. } => "provider_not_found",
            Self::ApiError { .. } => "api_error",
            Self::AuthError { .. } => "auth_error",
            Self::ConfigError(_) => "config_error",
            Self::RateLimited { .. } => "rate_limited",
            Self::ValidationError { .. } => "validation_error",
            Self::NotSupported { .. } => "not_supported",
            Self::Http(_) => "http_error",
            Self::Io(_) => "io_error",
            Self::SerdeJson(_) => "serde_error",
            Self::Toml(_) => "toml_error",
            Self::Csv(_) => "csv_error",
        }
    }

    /// Whether retrying the same request later may succeed.
    #[must_use]
    pub const fn is_transient(&self) -> bool {
        match self {
            Self::RateLimited { .. } | Self::Http(_) => true,
            Self::ApiError { status, .. } => *status == 429 || *status >= 500,
            _ => false,
        }
    }

    /// A recovery hint for the user or agent, when one exists.
    #[must_use]
    pub fn suggestion(&self) -> Option<String> {
        match self {
            Self::AuthError { provider, .. } => Some(format!(
                "Run 'mkt doctor' and check the MKT_{}_ACCESS_TOKEN environment \
                 variable or the profile config.",
                provider.to_uppercase()
            )),
            Self::ApiError { status, .. } if *status == 401 || *status == 403 => {
                Some("Run 'mkt doctor' to verify credentials and permissions.".to_string())
            }
            Self::NotSupported { .. } => {
                Some("Run 'mkt providers' to see each provider's capabilities.".to_string())
            }
            Self::RateLimited {
                retry_after_secs, ..
            } => Some(format!("Retry after {retry_after_secs} seconds.")),
            Self::ApiError {
                retry_after: Some(secs),
                ..
            } => Some(format!("Retry after {secs} seconds.")),
            Self::ProviderNotFound { available, .. } => {
                Some(format!("Available providers: {available}."))
            }
            Self::ConfigError(_) => {
                Some("Run 'mkt doctor' to validate the configuration.".to_string())
            }
            _ => None,
        }
    }
}

/// A `Result` type alias that uses [`MktError`].
pub type Result<T> = std::result::Result<T, MktError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_provider_not_found() {
        let err = MktError::ProviderNotFound {
            provider: "twitter".into(),
            available: "meta, google".into(),
        };
        assert_eq!(
            err.to_string(),
            "Provider 'twitter' not found. Available: meta, google"
        );
    }

    #[test]
    fn display_api_error() {
        let err = MktError::ApiError {
            provider: "meta".into(),
            status: 400,
            message: "Invalid objective".into(),
            retry_after: None,
        };
        assert_eq!(
            err.to_string(),
            "API error from meta: 400 — Invalid objective"
        );
    }

    #[test]
    fn display_not_supported() {
        let err = MktError::not_supported("tiktok", "dark_posts");
        assert_eq!(err.to_string(), "tiktok does not support 'dark_posts'");
    }

    #[test]
    fn display_rate_limited() {
        let err = MktError::RateLimited {
            provider: "meta".into(),
            retry_after_secs: 30,
        };
        assert_eq!(
            err.to_string(),
            "Rate limit exceeded for meta. Retry after 30s"
        );
    }

    #[test]
    fn display_auth_error() {
        let err = MktError::auth_error("google", "token expired");
        assert_eq!(
            err.to_string(),
            "Authentication failed for google: token expired"
        );
    }

    #[test]
    fn from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let mkt_err: MktError = io_err.into();
        assert!(matches!(mkt_err, MktError::Io(_)));
    }

    #[test]
    #[allow(clippy::panic)]
    fn from_serde_json_error() {
        let Err(json_err) = serde_json::from_str::<serde_json::Value>("not json") else {
            panic!("expected JSON parse error");
        };
        let mkt_err: MktError = json_err.into();
        assert!(matches!(mkt_err, MktError::SerdeJson(_)));
    }

    #[test]
    fn display_config_error() {
        let err = MktError::ConfigError("missing profile".into());
        assert_eq!(err.to_string(), "Configuration error: missing profile");
    }

    #[test]
    fn display_validation_error() {
        let err = MktError::ValidationError {
            field: "budget".into(),
            message: "must be positive".into(),
        };
        assert_eq!(
            err.to_string(),
            "Validation error: budget — must be positive"
        );
    }

    // ── Exit code contract (stable, documented in AGENTS.md) ──

    #[test]
    fn exit_codes_follow_documented_contract() {
        assert_eq!(
            MktError::ValidationError {
                field: "x".into(),
                message: "y".into()
            }
            .exit_code(),
            2
        );
        assert_eq!(MktError::ConfigError("bad".into()).exit_code(), 2);
        assert_eq!(MktError::auth_error("meta", "expired").exit_code(), 3);
        assert_eq!(
            MktError::ProviderNotFound {
                provider: "x".into(),
                available: "meta".into()
            }
            .exit_code(),
            4
        );
        assert_eq!(
            MktError::RateLimited {
                provider: "meta".into(),
                retry_after_secs: 10
            }
            .exit_code(),
            5
        );
        assert_eq!(MktError::not_supported("meta", "x").exit_code(), 6);
    }

    #[test]
    fn api_error_exit_code_depends_on_status() {
        let not_found = MktError::ApiError {
            provider: "meta".into(),
            status: 404,
            message: "missing".into(),
            retry_after: None,
        };
        assert_eq!(not_found.exit_code(), 4);

        let rate_limited = MktError::ApiError {
            provider: "meta".into(),
            status: 429,
            message: "slow down".into(),
            retry_after: Some(30),
        };
        assert_eq!(rate_limited.exit_code(), 5);

        let auth = MktError::ApiError {
            provider: "meta".into(),
            status: 401,
            message: "bad token".into(),
            retry_after: None,
        };
        assert_eq!(auth.exit_code(), 3);

        let other = MktError::ApiError {
            provider: "meta".into(),
            status: 400,
            message: "bad request".into(),
            retry_after: None,
        };
        assert_eq!(other.exit_code(), 7);
    }

    #[test]
    fn generic_errors_exit_code_is_one() {
        let io_err: MktError =
            std::io::Error::new(std::io::ErrorKind::NotFound, "file missing").into();
        assert_eq!(io_err.exit_code(), 1);
    }

    #[test]
    fn error_type_is_stable_snake_case() {
        assert_eq!(
            MktError::ValidationError {
                field: "x".into(),
                message: "y".into()
            }
            .error_type(),
            "validation_error"
        );
        assert_eq!(MktError::auth_error("m", "r").error_type(), "auth_error");
        assert_eq!(
            MktError::RateLimited {
                provider: "m".into(),
                retry_after_secs: 1
            }
            .error_type(),
            "rate_limited"
        );
        assert_eq!(
            MktError::not_supported("m", "f").error_type(),
            "not_supported"
        );
    }

    #[test]
    fn transient_errors_are_flagged_for_retry() {
        assert!(
            MktError::RateLimited {
                provider: "m".into(),
                retry_after_secs: 1
            }
            .is_transient()
        );
        let server_err = MktError::ApiError {
            provider: "m".into(),
            status: 503,
            message: "unavailable".into(),
            retry_after: None,
        };
        assert!(server_err.is_transient());
        assert!(
            !MktError::ValidationError {
                field: "x".into(),
                message: "y".into()
            }
            .is_transient()
        );
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn suggestions_guide_recovery() {
        let auth = MktError::auth_error("meta", "expired");
        let suggestion = auth.suggestion().expect("auth errors carry a suggestion");
        assert!(suggestion.contains("doctor"), "got: {suggestion}");

        let unsupported = MktError::not_supported("meta", "x");
        assert!(
            unsupported
                .suggestion()
                .expect("not_supported carries a suggestion")
                .contains("providers")
        );
    }
}
