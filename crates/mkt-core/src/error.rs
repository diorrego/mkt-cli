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
}
