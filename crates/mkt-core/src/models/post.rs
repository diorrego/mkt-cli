//! Post domain model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

use super::common::Budget;

/// Opaque post identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PostId(pub String);

impl fmt::Display for PostId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T: Into<String>> From<T> for PostId {
    fn from(s: T) -> Self {
        Self(s.into())
    }
}

/// A unified post representation across all providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    /// Unique identifier.
    pub id: PostId,
    /// Which provider this post belongs to.
    pub provider: String,
    /// The social platform (e.g. "facebook", "instagram").
    pub platform: String,
    /// Post message or caption.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Destination link URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    /// Image URL attached to the post.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    /// Public permalink to the post.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permalink: Option<String>,
    /// When the post was created.
    pub created_at: DateTime<Utc>,
    /// Original API response for debugging and raw access.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<serde_json::Value>,
}

/// Input for publishing a new post.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PublishPostInput {
    /// Target social platform.
    pub platform: String,
    /// Post message or caption.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Destination link URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    /// Image URL to attach to the post.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    /// Video URL to attach to the post.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_url: Option<String>,
    /// Provider-specific extra fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

/// Input for promoting an existing post as a paid ad.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotePostInput {
    /// Budget to allocate for the promotion.
    pub budget: Budget,
    /// Number of days to run the promotion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_days: Option<u32>,
    /// Targeting configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targeting: Option<serde_json::Value>,
}

impl crate::output::Formattable for Post {
    fn headers() -> Vec<String> {
        vec![
            "ID".into(),
            "Platform".into(),
            "Message".into(),
            "Permalink".into(),
            "Provider".into(),
        ]
    }

    fn row(&self) -> Vec<String> {
        vec![
            self.id.to_string(),
            self.platform.clone(),
            self.message
                .as_deref()
                .unwrap_or("-")
                .chars()
                .take(60)
                .collect(),
            self.permalink.clone().unwrap_or_else(|| "-".to_string()),
            self.provider.clone(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn post_id_display() {
        let id = PostId("post_123".into());
        assert_eq!(id.to_string(), "post_123");
    }

    #[test]
    fn post_id_from_str() {
        let id = PostId::from("post_456");
        assert_eq!(id.0, "post_456");
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn post_id_serde_roundtrip() {
        let id = PostId("post_789".into());
        let json = serde_json::to_string(&id).expect("serialize PostId");
        let back: PostId = serde_json::from_str(&json).expect("deserialize PostId");
        assert_eq!(id, back);
    }

    #[test]
    fn publish_post_input_default() {
        let input = PublishPostInput::default();
        assert!(input.platform.is_empty());
        assert!(input.message.is_none());
        assert!(input.link.is_none());
        assert!(input.image_url.is_none());
        assert!(input.video_url.is_none());
        assert!(input.extra.is_none());
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn publish_post_input_skips_none_fields() {
        let input = PublishPostInput {
            platform: "facebook".into(),
            message: Some("Hello!".into()),
            link: None,
            image_url: None,
            video_url: None,
            extra: None,
        };
        let json = serde_json::to_string(&input).expect("serialize PublishPostInput");
        assert!(!json.contains("link"));
        assert!(!json.contains("image_url"));
        assert!(!json.contains("video_url"));
        assert!(!json.contains("extra"));
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn promote_post_input_serde_roundtrip() {
        use super::super::common::{Budget, BudgetKind};
        let input = PromotePostInput {
            budget: Budget {
                amount: 5000.0,
                currency: "USD".into(),
                kind: BudgetKind::Daily,
            },
            duration_days: Some(7),
            targeting: None,
        };
        let json = serde_json::to_string(&input).expect("serialize PromotePostInput");
        let back: PromotePostInput =
            serde_json::from_str(&json).expect("deserialize PromotePostInput");
        assert!((back.budget.amount - 5000.0).abs() < f64::EPSILON);
        assert_eq!(back.budget.currency, "USD");
        assert_eq!(back.duration_days, Some(7));
        assert!(back.targeting.is_none());
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn post_optional_fields_skip_serializing_if_none() {
        let now = chrono::Utc::now();
        let post = Post {
            id: PostId("p_1".into()),
            provider: "meta".into(),
            platform: "facebook".into(),
            message: None,
            link: None,
            image_url: None,
            permalink: None,
            created_at: now,
            raw: None,
        };
        let json = serde_json::to_string(&post).expect("serialize Post");
        assert!(!json.contains("message"));
        assert!(!json.contains("link"));
        assert!(!json.contains("image_url"));
        assert!(!json.contains("permalink"));
        assert!(!json.contains("raw"));
    }
}
