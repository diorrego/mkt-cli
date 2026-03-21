//! Ad domain model.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::adset::AdSetId;
use super::creative::CreativeId;

/// Opaque ad identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AdId(pub String);

impl fmt::Display for AdId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T: Into<String>> From<T> for AdId {
    fn from(s: T) -> Self {
        Self(s.into())
    }
}

/// A unified ad representation across all providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ad {
    /// Unique identifier.
    pub id: AdId,
    /// Which provider this ad belongs to.
    pub provider: String,
    /// The parent ad set identifier.
    pub adset_id: AdSetId,
    /// Ad name.
    pub name: String,
    /// Current status.
    pub status: AdStatus,
    /// Creative associated with this ad, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creative_id: Option<CreativeId>,
    /// When the ad was created.
    pub created_at: DateTime<Utc>,
    /// When the ad was last updated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    /// Original API response for debugging and raw access.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<serde_json::Value>,
}

/// The lifecycle status of an ad.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdStatus {
    /// Ad is running.
    Active,
    /// Ad is paused.
    Paused,
    /// Ad is archived.
    Archived,
    /// Ad has been deleted.
    Deleted,
    /// Platform-specific status not mapped to a known variant.
    Other(String),
}

impl fmt::Display for AdStatus {
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

impl<'de> Deserialize<'de> for AdStatus {
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

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test]
    fn ad_id_display() {
        let id = AdId("ad_123".into());
        assert_eq!(id.to_string(), "ad_123");
    }

    #[test]
    fn ad_id_from_str() {
        let id = AdId::from("ad_456");
        assert_eq!(id.0, "ad_456");
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn ad_id_serde_roundtrip() {
        let id = AdId("ad_789".into());
        let json = serde_json::to_string(&id).expect("serialize AdId");
        let back: AdId = serde_json::from_str(&json).expect("deserialize AdId");
        assert_eq!(id, back);
    }

    #[test_case(AdStatus::Active, "active" ; "active")]
    #[test_case(AdStatus::Paused, "paused" ; "paused")]
    #[test_case(AdStatus::Archived, "archived" ; "archived")]
    #[test_case(AdStatus::Deleted, "deleted" ; "deleted")]
    #[allow(clippy::needless_pass_by_value)]
    fn ad_status_display(status: AdStatus, expected: &str) {
        assert_eq!(status.to_string(), expected);
    }

    #[test]
    fn ad_status_other_display() {
        let status = AdStatus::Other("in_review".into());
        assert_eq!(status.to_string(), "in_review");
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn ad_status_serde_roundtrip() {
        let json = serde_json::to_string(&AdStatus::Paused).expect("serialize");
        assert_eq!(json, r#""paused""#);
        let back: AdStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, AdStatus::Paused);
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn unknown_status_deserializes_as_other() {
        let status: AdStatus = serde_json::from_str(r#""pending_review""#).expect("deserialize");
        assert_eq!(status, AdStatus::Other("pending_review".into()));
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn ad_optional_fields_skip_serializing_if_none() {
        let now = chrono::Utc::now();
        let ad = Ad {
            id: AdId("ad_1".into()),
            provider: "meta".into(),
            adset_id: AdSetId::from("adset_1"),
            name: "Test Ad".into(),
            status: AdStatus::Active,
            creative_id: None,
            created_at: now,
            updated_at: None,
            raw: None,
        };
        let json = serde_json::to_string(&ad).expect("serialize Ad");
        assert!(!json.contains("creative_id"));
        assert!(!json.contains("updated_at"));
        assert!(!json.contains("raw"));
    }
}
