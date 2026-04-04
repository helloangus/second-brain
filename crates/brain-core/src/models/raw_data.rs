//! Raw data reference model

use serde::{Deserialize, Serialize};

/// Raw data type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RawDataType {
    Image,
    Audio,
    Video,
    #[default]
    Text,
    Document,
}

impl std::fmt::Display for RawDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RawDataType::Image => write!(f, "image"),
            RawDataType::Audio => write!(f, "audio"),
            RawDataType::Video => write!(f, "video"),
            RawDataType::Text => write!(f, "text"),
            RawDataType::Document => write!(f, "document"),
        }
    }
}

impl RawDataType {
    /// Get Chinese display name
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

/// Reference to raw data file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawDataRef {
    pub data_type: RawDataType,
    /// Relative path from event file, e.g., "../../data/raw/image/photo.jpg"
    pub path: String,
    /// The model used to process this data
    #[serde(default)]
    pub model_used: Option<String>,
    /// Path to the model's output (e.g., transcript, embedding)
    #[serde(default)]
    pub model_output: Option<String>,
}

impl RawDataRef {
    pub fn new(data_type: RawDataType, path: impl Into<String>) -> Self {
        Self {
            data_type,
            path: path.into(),
            model_used: None,
            model_output: None,
        }
    }

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
