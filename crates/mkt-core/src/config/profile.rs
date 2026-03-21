//! Profile and configuration types.
//!
//! The config file is a TOML file with a `[defaults]` section and
//! one or more `[profiles.<name>]` sections.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{MktError, Result};

/// Top-level configuration loaded from `config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MktConfig {
    /// Global defaults.
    #[serde(default)]
    pub defaults: Defaults,
    /// Named profiles.
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
}

impl MktConfig {
    /// Load configuration from a TOML file.
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load config from the default location, returning a default config if not found.
    pub fn load() -> Result<Self> {
        let file = super::config_file()?;
        if file.exists() {
            Self::load_from_file(&file)
        } else {
            Ok(Self::default())
        }
    }

    /// Get a profile by name, falling back to the default profile.
    pub fn profile(&self, name: &str) -> Result<&Profile> {
        self.profiles
            .get(name)
            .ok_or_else(|| MktError::ConfigError(format!("Profile '{name}' not found")))
    }
}

impl Default for MktConfig {
    fn default() -> Self {
        Self {
            defaults: Defaults::default(),
            profiles: HashMap::new(),
        }
    }
}

/// Global defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defaults {
    /// Default output format.
    #[serde(default = "default_output")]
    pub output: String,
    /// Default profile name.
    #[serde(default = "default_profile")]
    pub profile: String,
}

fn default_output() -> String {
    "table".into()
}

fn default_profile() -> String {
    "default".into()
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            output: default_output(),
            profile: default_profile(),
        }
    }
}

/// A named profile containing provider-specific configurations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// The default provider for this profile.
    #[serde(default)]
    pub provider: String,
    /// Meta provider config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaConfig>,
    /// Google Ads provider config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google: Option<GoogleConfig>,
    /// TikTok provider config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tiktok: Option<TikTokConfig>,
    /// LinkedIn provider config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkedin: Option<LinkedInConfig>,
}

/// Meta (Facebook/Instagram) provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaConfig {
    /// User access token (overridden by `MKT_META_ACCESS_TOKEN` env var).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    /// Ad account ID (e.g. "act_123456789").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ad_account_id: Option<String>,
    /// Facebook Page ID for organic posts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_id: Option<String>,
    /// Instagram user ID for IG publishing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ig_user_id: Option<String>,
    /// API version override (default: "v24.0").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
}

/// Google Ads provider configuration (stub).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleConfig {
    /// Developer token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub developer_token: Option<String>,
    /// OAuth client ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    /// OAuth client secret.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    /// OAuth refresh token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// Customer ID (e.g. "123-456-7890").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<String>,
}

/// TikTok for Business provider configuration (stub).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TikTokConfig {
    /// Access token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    /// Advertiser ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advertiser_id: Option<String>,
}

/// LinkedIn Marketing provider configuration (stub).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedInConfig {
    /// Access token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    /// Ad account ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ad_account_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_config() {
        let toml_str = r#"
[defaults]
output = "json"
profile = "test"

[profiles.test]
provider = "meta"

[profiles.test.meta]
access_token = "EAAB123"
ad_account_id = "act_123"
"#;
        let config: MktConfig = toml::from_str(toml_str).expect("parse TOML");
        assert_eq!(config.defaults.output, "json");
        assert_eq!(config.defaults.profile, "test");

        let profile = config.profile("test").expect("profile exists");
        assert_eq!(profile.provider, "meta");

        let meta = profile.meta.as_ref().expect("meta config");
        assert_eq!(meta.access_token.as_deref(), Some("EAAB123"));
        assert_eq!(meta.ad_account_id.as_deref(), Some("act_123"));
    }

    #[test]
    fn parse_empty_config() {
        let toml_str = "";
        let config: MktConfig = toml::from_str(toml_str).expect("parse TOML");
        assert_eq!(config.defaults.output, "table");
        assert_eq!(config.defaults.profile, "default");
        assert!(config.profiles.is_empty());
    }

    #[test]
    fn missing_profile_returns_error() {
        let config = MktConfig::default();
        let err = config.profile("nonexistent").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn config_with_multiple_profiles() {
        let toml_str = r#"
[profiles.client-a]
provider = "meta"

[profiles.client-a.meta]
ad_account_id = "act_111"

[profiles.client-b]
provider = "google"

[profiles.client-b.google]
customer_id = "123-456"
"#;
        let config: MktConfig = toml::from_str(toml_str).expect("parse TOML");
        assert_eq!(config.profiles.len(), 2);
        assert!(config.profile("client-a").is_ok());
        assert!(config.profile("client-b").is_ok());
    }

    #[test]
    fn defaults_are_applied() {
        let defaults = Defaults::default();
        assert_eq!(defaults.output, "table");
        assert_eq!(defaults.profile, "default");
    }
}
