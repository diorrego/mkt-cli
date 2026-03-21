//! Ad set domain model.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::campaign::CampaignId;
use super::common::Budget;

/// Opaque ad set identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AdSetId(pub String);

impl fmt::Display for AdSetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T: Into<String>> From<T> for AdSetId {
    fn from(s: T) -> Self {
        Self(s.into())
    }
}

/// A unified ad set representation across all providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdSet {
    /// Unique identifier.
    pub id: AdSetId,
    /// Which provider this ad set belongs to.
    pub provider: String,
    /// The parent campaign identifier.
    pub campaign_id: CampaignId,
    /// Ad set name.
    pub name: String,
    /// Current status.
    pub status: AdSetStatus,
    /// Targeting configuration, if set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targeting: Option<serde_json::Value>,
    /// Budget configuration, if set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget: Option<Budget>,
    /// When the ad set was created.
    pub created_at: DateTime<Utc>,
    /// When the ad set was last updated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    /// Original API response for debugging and raw access.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<serde_json::Value>,
}

/// The lifecycle status of an ad set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdSetStatus {
    /// Ad set is running.
    Active,
    /// Ad set is paused.
    Paused,
    /// Ad set is archived.
    Archived,
    /// Ad set has been deleted.
    Deleted,
    /// Platform-specific status not mapped to a known variant.
    Other(String),
}

impl fmt::Display for AdSetStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Paused => write!(f, "paused"),
            Self::Archived => write!(f, "archived"),
            Self::Deleted => write!(f, "deleted"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

impl<'de> Deserialize<'de> for AdSetStatus {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "active" => Self::Active,
            "paused" => Self::Paused,
            "archived" => Self::Archived,
            "deleted" => Self::Deleted,
            _ => Self::Other(s),
        })
    }
}

/// Input for creating a new ad set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAdSetInput {
    /// The parent campaign identifier.
    pub campaign_id: CampaignId,
    /// Ad set name.
    pub name: String,
    /// Initial status (defaults to provider default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<AdSetStatus>,
    /// Targeting configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targeting: Option<serde_json::Value>,
    /// Budget configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget: Option<Budget>,
    /// Provider-specific extra fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test]
    fn adset_id_display() {
        let id = AdSetId("adset_123".into());
        assert_eq!(id.to_string(), "adset_123");
    }

    #[test]
    fn adset_id_from_str() {
        let id = AdSetId::from("adset_456");
        assert_eq!(id.0, "adset_456");
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn adset_id_serde_roundtrip() {
        let id = AdSetId("adset_789".into());
        let json = serde_json::to_string(&id).expect("serialize AdSetId");
        let back: AdSetId = serde_json::from_str(&json).expect("deserialize AdSetId");
        assert_eq!(id, back);
    }

    #[test_case(AdSetStatus::Active, "active" ; "active")]
    #[test_case(AdSetStatus::Paused, "paused" ; "paused")]
    #[test_case(AdSetStatus::Archived, "archived" ; "archived")]
    #[test_case(AdSetStatus::Deleted, "deleted" ; "deleted")]
    #[allow(clippy::needless_pass_by_value)]
    fn adset_status_display(status: AdSetStatus, expected: &str) {
        assert_eq!(status.to_string(), expected);
    }

    #[test]
    fn adset_status_other_display() {
        let status = AdSetStatus::Other("pending_review".into());
        assert_eq!(status.to_string(), "pending_review");
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn adset_status_serde_roundtrip() {
        let json = serde_json::to_string(&AdSetStatus::Active).expect("serialize");
        assert_eq!(json, r#""active""#);
        let back: AdSetStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, AdSetStatus::Active);
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn unknown_status_deserializes_as_other() {
        let status: AdSetStatus =
            serde_json::from_str(r#""some_new_status""#).expect("deserialize");
        assert_eq!(status, AdSetStatus::Other("some_new_status".into()));
    }

    #[test]
    fn create_adset_input_construction() {
        let input = CreateAdSetInput {
            campaign_id: CampaignId::from("camp_1"),
            name: String::new(),
            status: None,
            targeting: None,
            budget: None,
            extra: None,
        };
        assert!(input.name.is_empty());
        assert!(input.status.is_none());
        assert!(input.targeting.is_none());
        assert!(input.budget.is_none());
        assert!(input.extra.is_none());
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn adset_optional_fields_skip_serializing_if_none() {
        let now = chrono::Utc::now();
        let adset = AdSet {
            id: AdSetId("adset_1".into()),
            provider: "meta".into(),
            campaign_id: CampaignId::from("camp_1"),
            name: "Test Ad Set".into(),
            status: AdSetStatus::Active,
            targeting: None,
            budget: None,
            created_at: now,
            updated_at: None,
            raw: None,
        };
        let json = serde_json::to_string(&adset).expect("serialize AdSet");
        assert!(!json.contains("targeting"));
        assert!(!json.contains("budget"));
        assert!(!json.contains("updated_at"));
        assert!(!json.contains("raw"));
    }
}
