//! Raw data reference model
//!
//! Represents references to raw data files (images, audio, video, etc.)
//! attached to events.

use serde::{Deserialize, Serialize};

// ============================================================================
// RawDataType
// ============================================================================

/// Raw data file type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RawDataType {
    /// Image file (photos, screenshots, etc.)
    Image,
    /// Audio file (recordings, voice memos, etc.)
    Audio,
    /// Video file (recordings, clips, etc.)
    Video,
    /// Plain text file
    #[default]
    Text,
    /// Document file (PDF, Word, etc.)
    Document,
}

impl RawDataType {
    fn as_str(&self) -> &'static str {
        match self {
            RawDataType::Image => "image",
            RawDataType::Audio => "audio",
            RawDataType::Video => "video",
            RawDataType::Text => "text",
            RawDataType::Document => "document",
        }
    }

    /// Returns the Chinese display name.
    pub fn display_zh(&self) -> &'static str {
        match self {
            RawDataType::Image => "图片",
            RawDataType::Audio => "音频",
            RawDataType::Video => "视频",
            RawDataType::Text => "文本",
            RawDataType::Document => "文档",
        }
    }
}

impl std::fmt::Display for RawDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// RawDataRef
// ============================================================================

/// Reference to a raw data file, stored relative to an event file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RawDataRef {
    /// Type of the raw data.
    pub data_type: RawDataType,
    /// Relative path from the event file (e.g., `"../../data/raw/image/photo.jpg"`).
    pub path: String,
    /// AI model used to process this data. `None` if not yet processed.
    #[serde(default)]
    pub model_used: Option<String>,
    /// Path to the model's output (e.g., transcript, embedding). `None` if not yet processed.
    #[serde(default)]
    pub model_output: Option<String>,
}

impl RawDataRef {
    /// Creates a new `RawDataRef` without model processing info.
    pub fn new(data_type: RawDataType, path: impl Into<String>) -> Self {
        Self {
            data_type,
            path: path.into(),
            model_used: None,
            model_output: None,
        }
    }

    /// Creates a new `RawDataRef` with model processing info.
    pub fn with_model(
        data_type: RawDataType,
        path: impl Into<String>,
        model_used: impl Into<String>,
        model_output: impl Into<String>,
    ) -> Self {
        Self {
            data_type,
            path: path.into(),
            model_used: Some(model_used.into()),
            model_output: Some(model_output.into()),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------------
    // RawDataType Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_raw_data_type_default() {
        assert_eq!(RawDataType::default(), RawDataType::Text);
    }

    #[test]
    fn test_raw_data_type_display() {
        assert_eq!(RawDataType::Image.to_string(), "image");
        assert_eq!(RawDataType::Audio.to_string(), "audio");
        assert_eq!(RawDataType::Video.to_string(), "video");
        assert_eq!(RawDataType::Text.to_string(), "text");
        assert_eq!(RawDataType::Document.to_string(), "document");
    }

    #[test]
    fn test_raw_data_type_display_zh() {
        assert_eq!(RawDataType::Image.display_zh(), "图片");
        assert_eq!(RawDataType::Audio.display_zh(), "音频");
        assert_eq!(RawDataType::Video.display_zh(), "视频");
        assert_eq!(RawDataType::Text.display_zh(), "文本");
        assert_eq!(RawDataType::Document.display_zh(), "文档");
    }

    #[test]
    fn test_raw_data_type_partial_eq() {
        assert_eq!(RawDataType::Image, RawDataType::Image);
        assert_ne!(RawDataType::Image, RawDataType::Audio);
    }

    #[test]
    fn test_raw_data_type_serde_roundtrip() {
        for data_type in [
            RawDataType::Image,
            RawDataType::Audio,
            RawDataType::Video,
            RawDataType::Text,
            RawDataType::Document,
        ] {
            let json = serde_json::to_string(&data_type).unwrap();
            let parsed: RawDataType = serde_json::from_str(&json).unwrap();
            assert_eq!(data_type, parsed);
        }
    }

    #[test]
    fn test_raw_data_type_yaml_roundtrip() {
        for data_type in [
            RawDataType::Image,
            RawDataType::Audio,
            RawDataType::Video,
            RawDataType::Text,
            RawDataType::Document,
        ] {
            let yaml = serde_yaml::to_string(&data_type).unwrap();
            let parsed: RawDataType = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(data_type, parsed);
        }
    }

    // ------------------------------------------------------------------------
    // RawDataRef Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_raw_data_ref_new() {
        let ref_ = RawDataRef::new(RawDataType::Image, "../../data/photo.jpg");
        assert_eq!(ref_.data_type, RawDataType::Image);
        assert_eq!(ref_.path, "../../data/photo.jpg");
        assert!(ref_.model_used.is_none());
        assert!(ref_.model_output.is_none());
    }

    #[test]
    fn test_raw_data_ref_with_model() {
        let ref_ = RawDataRef::with_model(
            RawDataType::Audio,
            "../../data/recording.mp3",
            "whisper-1",
            "../../data/recording.txt",
        );
        assert_eq!(ref_.data_type, RawDataType::Audio);
        assert_eq!(ref_.path, "../../data/recording.mp3");
        assert_eq!(ref_.model_used, Some("whisper-1".to_string()));
        assert_eq!(
            ref_.model_output,
            Some("../../data/recording.txt".to_string())
        );
    }

    #[test]
    fn test_raw_data_ref_clone() {
        let ref_ = RawDataRef::with_model(
            RawDataType::Video,
            "../../data/video.mp4",
            "gemini-pro",
            "../../data/video_desc.txt",
        );
        let cloned = ref_.clone();
        assert_eq!(ref_, cloned);
    }

    #[test]
    fn test_raw_data_ref_serde_roundtrip() {
        let ref_ = RawDataRef::new(RawDataType::Document, "../../data/report.pdf");
        let json = serde_json::to_string(&ref_).unwrap();
        let parsed: RawDataRef = serde_json::from_str(&json).unwrap();
        assert_eq!(ref_, parsed);
    }

    #[test]
    fn test_raw_data_ref_with_model_serde_roundtrip() {
        let ref_ = RawDataRef::with_model(
            RawDataType::Image,
            "../../data/photo.jpg",
            "gemini-1.5",
            "../../data/photo_desc.txt",
        );
        let json = serde_json::to_string(&ref_).unwrap();
        let parsed: RawDataRef = serde_json::from_str(&json).unwrap();
        assert_eq!(ref_, parsed);
    }

    #[test]
    fn test_raw_data_ref_yaml_deserialization_missing_optional_fields() {
        // Simulates YAML frontmatter where model_used and model_output are omitted
        let yaml = r#"
data_type: image
path: "../../data/photo.jpg"
"#;
        let parsed: RawDataRef = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(parsed.data_type, RawDataType::Image);
        assert_eq!(parsed.path, "../../data/photo.jpg");
        assert!(parsed.model_used.is_none());
        assert!(parsed.model_output.is_none());
    }

    #[test]
    fn test_raw_data_ref_yaml_roundtrip() {
        let ref_ = RawDataRef::with_model(
            RawDataType::Video,
            "../../data/video.mp4",
            "gemini-pro",
            "../../data/video_desc.txt",
        );
        let yaml = serde_yaml::to_string(&ref_).unwrap();
        let parsed: RawDataRef = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(ref_, parsed);
    }
}
