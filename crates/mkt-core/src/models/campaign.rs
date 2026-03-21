//! Campaign domain model.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::common::Budget;

/// Opaque campaign identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CampaignId(pub String);

impl fmt::Display for CampaignId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T: Into<String>> From<T> for CampaignId {
    fn from(s: T) -> Self {
        Self(s.into())
    }
}

/// A unified campaign representation across all providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Campaign {
    /// Unique identifier.
    pub id: CampaignId,
    /// Which provider this campaign belongs to.
    pub provider: String,
    /// Campaign name.
    pub name: String,
    /// Current status.
    pub status: CampaignStatus,
    /// Campaign objective (e.g. `OUTCOME_LEADS`).
    pub objective: String,
    /// Budget configuration, if set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget: Option<Budget>,
    /// When the campaign was created.
    pub created_at: DateTime<Utc>,
    /// When the campaign was last updated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    /// Original API response for debugging and raw access.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<serde_json::Value>,
}

/// The lifecycle status of a campaign.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignStatus {
    /// Campaign is running.
    Active,
    /// Campaign is paused.
    Paused,
    /// Campaign is archived.
    Archived,
    /// Campaign is in draft state.
    Draft,
    /// Campaign has been deleted.
    Deleted,
    /// Platform-specific status not mapped to a known variant.
    Other(String),
}

impl fmt::Display for CampaignStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Paused => write!(f, "paused"),
            Self::Archived => write!(f, "archived"),
            Self::Draft => write!(f, "draft"),
            Self::Deleted => write!(f, "deleted"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

impl<'de> Deserialize<'de> for CampaignStatus {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "active" => Self::Active,
            "paused" => Self::Paused,
            "archived" => Self::Archived,
            "draft" => Self::Draft,
            "deleted" => Self::Deleted,
            _ => Self::Other(s),
        })
    }
}

/// Input for creating a new campaign.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateCampaignInput {
    /// Campaign name.
    pub name: String,
    /// Campaign objective.
    pub objective: String,
    /// Initial status (defaults to provider default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<CampaignStatus>,
    /// Budget configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget: Option<Budget>,
    /// Provider-specific extra fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

/// Input for updating an existing campaign.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateCampaignInput {
    /// New name, if changing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New status, if changing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<CampaignStatus>,
    /// New budget, if changing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget: Option<Budget>,
    /// Provider-specific extra fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

/// Filters for listing campaigns.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CampaignFilters {
    /// Filter by status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<CampaignStatus>,
    /// Filter by name substring.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_contains: Option<String>,
    /// Maximum number of results per page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Pagination cursor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

impl crate::output::Formattable for Campaign {
    fn headers() -> Vec<String> {
        vec![
            "ID".into(),
            "Name".into(),
            "Status".into(),
            "Objective".into(),
            "Provider".into(),
        ]
    }

    fn row(&self) -> Vec<String> {
        vec![
            self.id.to_string(),
            self.name.clone(),
            self.status.to_string(),
            self.objective.clone(),
            self.provider.clone(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test]
    fn campaign_id_display() {
        let id = CampaignId("camp_123".into());
        assert_eq!(id.to_string(), "camp_123");
    }

    #[test]
    fn campaign_id_from_str() {
        let id = CampaignId::from("camp_456");
        assert_eq!(id.0, "camp_456");
    }

    #[test_case(CampaignStatus::Active, "active" ; "active")]
    #[test_case(CampaignStatus::Paused, "paused" ; "paused")]
    #[test_case(CampaignStatus::Archived, "archived" ; "archived")]
    #[test_case(CampaignStatus::Draft, "draft" ; "draft")]
    #[test_case(CampaignStatus::Deleted, "deleted" ; "deleted")]
    #[allow(clippy::needless_pass_by_value)]
    fn campaign_status_display(status: CampaignStatus, expected: &str) {
        assert_eq!(status.to_string(), expected);
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn campaign_status_serde_roundtrip() {
        let json = serde_json::to_string(&CampaignStatus::Active).expect("serialize");
        assert_eq!(json, r#""active""#);
        let back: CampaignStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, CampaignStatus::Active);
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn unknown_status_deserializes_as_other() {
        let status: CampaignStatus =
            serde_json::from_str(r#""something_new""#).expect("deserialize");
        assert_eq!(status, CampaignStatus::Other("something_new".into()));
    }

    #[test]
    fn campaign_filters_default_is_empty() {
        let filters = CampaignFilters::default();
        assert!(filters.status.is_none());
        assert!(filters.name_contains.is_none());
        assert!(filters.limit.is_none());
        assert!(filters.cursor.is_none());
    }

    #[test]
    fn create_campaign_input_default() {
        let input = CreateCampaignInput::default();
        assert!(input.name.is_empty());
        assert!(input.objective.is_empty());
        assert!(input.status.is_none());
    }
}
