//! Media asset domain model.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Opaque media asset identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MediaAssetId(pub String);

impl fmt::Display for MediaAssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T: Into<String>> From<T> for MediaAssetId {
    fn from(s: T) -> Self {
        Self(s.into())
    }
}

/// A unified media asset representation across all providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaAsset {
    /// Unique identifier.
    pub id: MediaAssetId,
    /// Which provider this asset belongs to.
    pub provider: String,
    /// The type of media.
    pub media_type: MediaType,
    /// CDN or storage URL for the asset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Original filename.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    /// File size in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    /// When the asset was created.
    pub created_at: DateTime<Utc>,
    /// Original API response for debugging and raw access.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<serde_json::Value>,
}

/// The media format of an asset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    /// A static image (JPEG, PNG, GIF, etc.).
    Image,
    /// A video file.
    Video,
    /// Platform-specific media type not mapped to a known variant.
    Other(String),
}

impl fmt::Display for MediaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Image => write!(f, "image"),
            Self::Video => write!(f, "video"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

impl<'de> Deserialize<'de> for MediaType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "image" => Self::Image,
            "video" => Self::Video,
            _ => Self::Other(s),
        })
    }
}

/// Input for uploading an image asset.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UploadImageInput {
    /// Local file path to the image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    /// Remote URL of the image to import.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Optional display name for the asset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Input for uploading a video asset.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UploadVideoInput {
    /// Local file path to the video.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    /// Remote URL of the video to import.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Optional display name for the asset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Video title shown in the player.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Video description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl crate::output::Formattable for MediaAsset {
    fn headers() -> Vec<String> {
        vec!["ID".into(), "Type".into(), "URL".into(), "Provider".into()]
    }

    fn row(&self) -> Vec<String> {
        vec![
            self.id.to_string(),
            self.media_type.to_string(),
            self.url.clone().unwrap_or_default(),
            self.provider.clone(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test]
    fn media_asset_id_display() {
        let id = MediaAssetId("media_123".into());
        assert_eq!(id.to_string(), "media_123");
    }

    #[test]
    fn media_asset_id_from_str() {
        let id = MediaAssetId::from("media_456");
        assert_eq!(id.0, "media_456");
    }

    #[test]
    fn media_asset_id_serde_roundtrip() {
        let id = MediaAssetId("media_789".into());
        let json = serde_json::to_string(&id).expect("serialize MediaAssetId");
        let back: MediaAssetId = serde_json::from_str(&json).expect("deserialize MediaAssetId");
        assert_eq!(id, back);
    }

    #[test_case(MediaType::Image, "image" ; "image")]
    #[test_case(MediaType::Video, "video" ; "video")]
    fn media_type_display(media_type: MediaType, expected: &str) {
        assert_eq!(media_type.to_string(), expected);
    }

    #[test]
    fn media_type_other_display() {
        let t = MediaType::Other("audio".into());
        assert_eq!(t.to_string(), "audio");
    }

    #[test]
    fn media_type_serde_roundtrip() {
        let json = serde_json::to_string(&MediaType::Image).expect("serialize");
        assert_eq!(json, r#""image""#);
        let back: MediaType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, MediaType::Image);
    }

    #[test]
    fn unknown_media_type_deserializes_as_other() {
        let t: MediaType = serde_json::from_str(r#""document""#).expect("deserialize");
        assert_eq!(t, MediaType::Other("document".into()));
    }

    #[test]
    fn upload_image_input_default_all_none() {
        let input = UploadImageInput::default();
        assert!(input.file_path.is_none());
        assert!(input.url.is_none());
        assert!(input.name.is_none());
    }

    #[test]
    fn upload_video_input_default_all_none() {
        let input = UploadVideoInput::default();
        assert!(input.file_path.is_none());
        assert!(input.url.is_none());
        assert!(input.name.is_none());
        assert!(input.title.is_none());
        assert!(input.description.is_none());
    }

    #[test]
    fn upload_image_input_skips_none_fields() {
        let input = UploadImageInput {
            file_path: Some("/tmp/image.png".into()),
            url: None,
            name: None,
        };
        let json = serde_json::to_string(&input).expect("serialize UploadImageInput");
        assert!(!json.contains("\"url\""));
        assert!(!json.contains("\"name\""));
    }

    #[test]
    fn upload_video_input_serde_roundtrip() {
        let input = UploadVideoInput {
            file_path: None,
            url: Some("https://cdn.example.com/video.mp4".into()),
            name: Some("my_video".into()),
            title: Some("My Video".into()),
            description: Some("A great video".into()),
        };
        let json = serde_json::to_string(&input).expect("serialize UploadVideoInput");
        let back: UploadVideoInput =
            serde_json::from_str(&json).expect("deserialize UploadVideoInput");
        assert!(back.file_path.is_none());
        assert_eq!(
            back.url.as_deref(),
            Some("https://cdn.example.com/video.mp4")
        );
        assert_eq!(back.title.as_deref(), Some("My Video"));
        assert_eq!(back.description.as_deref(), Some("A great video"));
    }

    #[test]
    fn media_asset_optional_fields_skip_serializing_if_none() {
        let now = chrono::Utc::now();
        let asset = MediaAsset {
            id: MediaAssetId("m_1".into()),
            provider: "meta".into(),
            media_type: MediaType::Image,
            url: None,
            filename: None,
            size_bytes: None,
            created_at: now,
            raw: None,
        };
        let json = serde_json::to_string(&asset).expect("serialize MediaAsset");
        assert!(!json.contains("url"));
        assert!(!json.contains("filename"));
        assert!(!json.contains("size_bytes"));
        assert!(!json.contains("raw"));
    }
}
