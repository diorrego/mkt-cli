//! Audience domain model.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Opaque audience identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AudienceId(pub String);

impl fmt::Display for AudienceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T: Into<String>> From<T> for AudienceId {
    fn from(s: T) -> Self {
        Self(s.into())
    }
}

/// A unified audience representation across all providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Audience {
    /// Unique identifier.
    pub id: AudienceId,
    /// Which provider this audience belongs to.
    pub provider: String,
    /// Audience name.
    pub name: String,
    /// Optional human-readable description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Estimated number of people in the audience.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// The type of audience.
    pub audience_type: AudienceType,
    /// When the audience was created.
    pub created_at: DateTime<Utc>,
    /// Original API response for debugging and raw access.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<serde_json::Value>,
}

/// The classification of an audience.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AudienceType {
    /// Custom audience built from first-party data.
    Custom,
    /// Lookalike audience derived from a seed audience.
    Lookalike,
    /// Saved audience based on demographic/interest criteria.
    SavedAudience,
    /// Platform-specific type not mapped to a known variant.
    Other(String),
}

impl Default for AudienceType {
    fn default() -> Self {
        Self::Custom
    }
}

impl fmt::Display for AudienceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Custom => write!(f, "custom"),
            Self::Lookalike => write!(f, "lookalike"),
            Self::SavedAudience => write!(f, "saved_audience"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

impl<'de> Deserialize<'de> for AudienceType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "custom" => Self::Custom,
            "lookalike" => Self::Lookalike,
            "saved_audience" => Self::SavedAudience,
            _ => Self::Other(s),
        })
    }
}

/// Input for creating a new audience.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateAudienceInput {
    /// Audience name.
    pub name: String,
    /// Optional description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Type of audience to create.
    pub audience_type: AudienceType,
    /// Provider-specific extra fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

/// A single user record for audience membership operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AudienceUser {
    /// Hashed or plain email address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Hashed or plain phone number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    /// First-party external identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
}

/// Result returned after adding or removing users from an audience.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudienceUpdateResult {
    /// The audience that was updated.
    pub audience_id: AudienceId,
    /// Number of user records received.
    pub num_received: u64,
    /// Number of user records that were invalid.
    pub num_invalid: u64,
}

impl crate::output::Formattable for Audience {
    fn headers() -> Vec<String> {
        vec![
            "ID".into(),
            "Name".into(),
            "Type".into(),
            "Size".into(),
            "Provider".into(),
        ]
    }

    fn row(&self) -> Vec<String> {
        vec![
            self.id.to_string(),
            self.name.clone(),
            self.audience_type.to_string(),
            self.size.map_or("-".to_string(), |s| s.to_string()),
            self.provider.clone(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test]
    fn audience_id_display() {
        let id = AudienceId("aud_123".into());
        assert_eq!(id.to_string(), "aud_123");
    }

    #[test]
    fn audience_id_from_str() {
        let id = AudienceId::from("aud_456");
        assert_eq!(id.0, "aud_456");
    }

    #[test]
    fn audience_id_serde_roundtrip() {
        let id = AudienceId("aud_789".into());
        let json = serde_json::to_string(&id).expect("serialize AudienceId");
        let back: AudienceId = serde_json::from_str(&json).expect("deserialize AudienceId");
        assert_eq!(id, back);
    }

    #[test_case(AudienceType::Custom, "custom" ; "custom")]
    #[test_case(AudienceType::Lookalike, "lookalike" ; "lookalike")]
    #[test_case(AudienceType::SavedAudience, "saved_audience" ; "saved_audience")]
    fn audience_type_display(audience_type: AudienceType, expected: &str) {
        assert_eq!(audience_type.to_string(), expected);
    }

    #[test]
    fn audience_type_other_display() {
        let t = AudienceType::Other("combined".into());
        assert_eq!(t.to_string(), "combined");
    }

    #[test]
    fn audience_type_serde_roundtrip() {
        let json = serde_json::to_string(&AudienceType::Lookalike).expect("serialize");
        assert_eq!(json, r#""lookalike""#);
        let back: AudienceType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, AudienceType::Lookalike);
    }

    #[test]
    fn unknown_audience_type_deserializes_as_other() {
        let t: AudienceType =
            serde_json::from_str(r#""website_retargeting""#).expect("deserialize");
        assert_eq!(t, AudienceType::Other("website_retargeting".into()));
    }

    #[test]
    fn create_audience_input_default() {
        let input = CreateAudienceInput::default();
        assert!(input.name.is_empty());
        assert!(input.description.is_none());
        assert_eq!(input.audience_type, AudienceType::Custom);
        assert!(input.extra.is_none());
    }

    #[test]
    fn audience_user_default_all_none() {
        let user = AudienceUser::default();
        assert!(user.email.is_none());
        assert!(user.phone.is_none());
        assert!(user.external_id.is_none());
    }

    #[test]
    fn audience_update_result_serde_roundtrip() {
        let result = AudienceUpdateResult {
            audience_id: AudienceId("aud_1".into()),
            num_received: 100,
            num_invalid: 5,
        };
        let json = serde_json::to_string(&result).expect("serialize AudienceUpdateResult");
        let back: AudienceUpdateResult =
            serde_json::from_str(&json).expect("deserialize AudienceUpdateResult");
        assert_eq!(back.audience_id, result.audience_id);
        assert_eq!(back.num_received, 100);
        assert_eq!(back.num_invalid, 5);
    }

    #[test]
    fn audience_optional_fields_skip_serializing_if_none() {
        let now = chrono::Utc::now();
        let audience = Audience {
            id: AudienceId("aud_1".into()),
            provider: "meta".into(),
            name: "Test Audience".into(),
            description: None,
            size: None,
            audience_type: AudienceType::Custom,
            created_at: now,
            raw: None,
        };
        let json = serde_json::to_string(&audience).expect("serialize Audience");
        assert!(!json.contains("description"));
        assert!(!json.contains("size"));
        assert!(!json.contains("raw"));
    }
}
