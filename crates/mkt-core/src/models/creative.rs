//! Creative domain model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Opaque creative identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CreativeId(pub String);

impl fmt::Display for CreativeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T: Into<String>> From<T> for CreativeId {
    fn from(s: T) -> Self {
        Self(s.into())
    }
}

/// A unified creative representation across all providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Creative {
    /// Unique identifier.
    pub id: CreativeId,
    /// Which provider this creative belongs to.
    pub provider: String,
    /// Creative name.
    pub name: String,
    /// Ad body text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// URL of the image asset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    /// URL of the video asset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_url: Option<String>,
    /// Destination link URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_url: Option<String>,
    /// Call-to-action label (e.g. "LEARN_MORE").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_to_action: Option<String>,
    /// When the creative was created.
    pub created_at: DateTime<Utc>,
    /// Original API response for debugging and raw access.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<serde_json::Value>,
}

/// Input for creating a new creative.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateCreativeInput {
    /// Creative name.
    pub name: String,
    /// Ad body text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// URL of the image asset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    /// URL of the video asset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_url: Option<String>,
    /// Destination link URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_url: Option<String>,
    /// Call-to-action label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_to_action: Option<String>,
    /// Provider-specific extra fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

/// Input for creating a dark post (unpublished page post).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateDarkPostInput {
    /// The Facebook Page ID to post on behalf of.
    pub page_id: String,
    /// The post message / body text.
    pub message: String,
    /// Optional destination link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    /// Optional image URL to attach.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    /// Call-to-action label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_to_action: Option<String>,
}

impl crate::output::Formattable for Creative {
    fn headers() -> Vec<String> {
        vec![
            "ID".into(),
            "Name".into(),
            "Body".into(),
            "Link URL".into(),
            "Provider".into(),
        ]
    }

    fn row(&self) -> Vec<String> {
        vec![
            self.id.to_string(),
            self.name.clone(),
            self.body
                .as_deref()
                .unwrap_or("-")
                .chars()
                .take(40)
                .collect(),
            self.link_url.clone().unwrap_or_else(|| "-".to_string()),
            self.provider.clone(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creative_id_display() {
        let id = CreativeId("creative_123".into());
        assert_eq!(id.to_string(), "creative_123");
    }

    #[test]
    fn creative_id_from_str() {
        let id = CreativeId::from("creative_456");
        assert_eq!(id.0, "creative_456");
    }

    #[test]
    fn creative_id_serde_roundtrip() {
        let id = CreativeId("creative_789".into());
        let json = serde_json::to_string(&id).expect("serialize CreativeId");
        let back: CreativeId = serde_json::from_str(&json).expect("deserialize CreativeId");
        assert_eq!(id, back);
    }

    #[test]
    fn create_creative_input_default() {
        let input = CreateCreativeInput::default();
        assert!(input.name.is_empty());
        assert!(input.body.is_none());
        assert!(input.image_url.is_none());
        assert!(input.video_url.is_none());
        assert!(input.link_url.is_none());
        assert!(input.call_to_action.is_none());
        assert!(input.extra.is_none());
    }

    #[test]
    fn creative_optional_fields_skip_serializing_if_none() {
        let now = chrono::Utc::now();
        let creative = Creative {
            id: CreativeId("c_1".into()),
            provider: "meta".into(),
            name: "Test Creative".into(),
            body: None,
            image_url: None,
            video_url: None,
            link_url: None,
            call_to_action: None,
            created_at: now,
            raw: None,
        };
        let json = serde_json::to_string(&creative).expect("serialize Creative");
        assert!(!json.contains("body"));
        assert!(!json.contains("image_url"));
        assert!(!json.contains("video_url"));
        assert!(!json.contains("link_url"));
        assert!(!json.contains("call_to_action"));
        assert!(!json.contains("raw"));
    }

    #[test]
    fn create_dark_post_input_serde_roundtrip() {
        let input = CreateDarkPostInput {
            page_id: "page_1".into(),
            message: "Hello world".into(),
            link: Some("https://example.com".into()),
            image_url: None,
            call_to_action: Some("LEARN_MORE".into()),
        };
        let json = serde_json::to_string(&input).expect("serialize CreateDarkPostInput");
        let back: CreateDarkPostInput =
            serde_json::from_str(&json).expect("deserialize CreateDarkPostInput");
        assert_eq!(back.page_id, "page_1");
        assert_eq!(back.message, "Hello world");
        assert_eq!(back.link.as_deref(), Some("https://example.com"));
        assert!(back.image_url.is_none());
        assert_eq!(back.call_to_action.as_deref(), Some("LEARN_MORE"));
    }

    #[test]
    fn create_dark_post_input_skips_none_fields() {
        let input = CreateDarkPostInput {
            page_id: "page_1".into(),
            message: "Hello".into(),
            link: None,
            image_url: None,
            call_to_action: None,
        };
        let json = serde_json::to_string(&input).expect("serialize");
        assert!(!json.contains("link"));
        assert!(!json.contains("image_url"));
        assert!(!json.contains("call_to_action"));
    }
}
