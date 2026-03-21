//! Token resolution logic.

use secrecy::{ExposeSecret, SecretString};

use crate::error::{MktError, Result};

/// Resolves an API token from environment or configuration.
///
/// Resolution order:
/// 1. Environment variable (if `env_var_name` is provided)
/// 2. Config value (if `config_value` is provided)
/// 3. Returns an `AuthError`
pub fn resolve_token(
    provider: &str,
    env_var_name: &str,
    config_value: Option<&str>,
) -> Result<SecretString> {
    // 1. Try environment variable
    if let Ok(token) = std::env::var(env_var_name) {
        if !token.is_empty() {
            return Ok(SecretString::from(token));
        }
    }

    // 2. Try config value
    if let Some(token) = config_value {
        if !token.is_empty() {
            return Ok(SecretString::from(token.to_owned()));
        }
    }

    // 3. No token found
    Err(MktError::auth_error(
        provider,
        &format!("No access token found. Set {env_var_name} or configure it in your profile."),
    ))
}

/// Verify that a secret string is non-empty (without exposing it).
pub fn validate_token(token: &SecretString) -> bool {
    !token.expose_secret().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_from_config_value() {
        // Use a key that definitely does NOT exist
        let key = "MKT_TEST_TOKEN_DEFINITELY_NOT_SET_98765";
        let result = resolve_token("test", key, Some("config-token"));
        let token = result.expect("should resolve");
        assert_eq!(token.expose_secret(), "config-token");
    }

    #[test]
    fn missing_token_returns_auth_error() {
        let key = "MKT_TEST_TOKEN_DEFINITELY_NOT_SET_11111";
        let result = resolve_token("test", key, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No access token"));
    }

    #[test]
    fn empty_config_value_returns_error() {
        let key = "MKT_TEST_TOKEN_DEFINITELY_NOT_SET_22222";
        let result = resolve_token("test", key, Some(""));
        assert!(result.is_err());
    }

    #[test]
    fn env_var_resolution_with_existing_var() {
        // HOME is always set on Linux — use it to test env var path
        let result = resolve_token("test", "HOME", None);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_non_empty_token() {
        let token = SecretString::from("valid".to_owned());
        assert!(validate_token(&token));
    }

    #[test]
    fn validate_empty_token() {
        let token = SecretString::from(String::new());
        assert!(!validate_token(&token));
    }
}
