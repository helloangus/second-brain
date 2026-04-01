//! Task models for AI pipeline

use super::raw_data::RawDataType;
use serde::{Deserialize, Serialize};

/// Task type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    ImageCaption,
    FaceDetection,
    Ocr,
    Asr,
    SpeakerDiarization,
    Embedding,
    #[default]
    Reasoning,
    Routing,
    Summarize,
    Tagging,
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskType::ImageCaption => write!(f, "image_caption"),
            TaskType::FaceDetection => write!(f, "face_detection"),
            TaskType::Ocr => write!(f, "ocr"),
            TaskType::Asr => write!(f, "asr"),
            TaskType::SpeakerDiarization => write!(f, "speaker_diarization"),
            TaskType::Embedding => write!(f, "embedding"),
            TaskType::Reasoning => write!(f, "reasoning"),
            TaskType::Routing => write!(f, "routing"),
            TaskType::Summarize => write!(f, "summarize"),
            TaskType::Tagging => write!(f, "tagging"),
        }
    }
}

impl TaskType {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "image_caption" | "image.analyze" => Some(TaskType::ImageCaption),
            "face_detection" | "face.detect" => Some(TaskType::FaceDetection),
            "ocr" => Some(TaskType::Ocr),
            "asr" | "audio.transcribe" => Some(TaskType::Asr),
            "speaker_diarization" | "speaker.diarize" => Some(TaskType::SpeakerDiarization),
            "embedding" => Some(TaskType::Embedding),
            "reasoning" => Some(TaskType::Reasoning),
            "routing" => Some(TaskType::Routing),
            "summarize" => Some(TaskType::Summarize),
            "tagging" => Some(TaskType::Tagging),
            _ => None,
        }
    }

    pub fn data_type(&self) -> RawDataType {
        match self {
            TaskType::ImageCaption => RawDataType::Image,
            TaskType::FaceDetection => RawDataType::Image,
            TaskType::Ocr => RawDataType::Image,
            TaskType::Asr => RawDataType::Audio,
            TaskType::SpeakerDiarization => RawDataType::Audio,
            TaskType::Embedding => RawDataType::Text,
            TaskType::Reasoning => RawDataType::Text,
            TaskType::Routing => RawDataType::Text,
            TaskType::Summarize => RawDataType::Text,
            TaskType::Tagging => RawDataType::Text,
        }
    }
}

/// Recommended models for each data/task type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRecommendation {
    pub task_type: TaskType,
    pub data_type: RawDataType,
    pub recommended_models: Vec<String>,
}

impl ModelRecommendation {
    /// Get recommended models for common use cases
    pub fn get_defaults() -> Vec<Self> {
        vec![
            ModelRecommendation {
                task_type: TaskType::ImageCaption,
                data_type: RawDataType::Image,
                recommended_models: vec!["Qwen2-VL".to_string(), "LLaVA".to_string()],
            },
            ModelRecommendation {
                task_type: TaskType::FaceDetection,
                data_type: RawDataType::Image,
                recommended_models: vec!["InsightFace".to_string()],
            },
            ModelRecommendation {
                task_type: TaskType::Ocr,
                data_type: RawDataType::Image,
                recommended_models: vec!["PaddleOCR".to_string()],
            },
            ModelRecommendation {
                task_type: TaskType::Asr,
                data_type: RawDataType::Audio,
                recommended_models: vec!["Whisper.cpp".to_string()],
            },
            ModelRecommendation {
                task_type: TaskType::SpeakerDiarization,
                data_type: RawDataType::Audio,
                recommended_models: vec!["pyannote".to_string()],
            },
            ModelRecommendation {
                task_type: TaskType::Embedding,
                data_type: RawDataType::Text,
                recommended_models: vec!["bge-small-en".to_string()],
            },
            ModelRecommendation {
                task_type: TaskType::Reasoning,
                data_type: RawDataType::Text,
                recommended_models: vec!["Qwen2.5".to_string(), "Llama3".to_string()],
            },
        ]
    }
}

/// Pipeline task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineTask {
    pub id: String,
    pub task: TaskType,
    pub input: PipelineInput,
    pub output: Option<PipelineOutput>,
    pub status: TaskStatus,
}

impl PipelineTask {
    pub fn data_type(&self) -> RawDataType {
        self.task.data_type()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    #[default]
    Pending,
    Processing,
    Done,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineInput {
    /// Path to the input file (relative to raw_data_path, includes data_type prefix)
    pub path: String,
    /// Input channel (e.g., CLI, API, Web)
    pub channel: Option<String>,
    /// Device that created this data (e.g., PC, iPhone, Server)
    pub device: Option<String>,
    /// How the data was captured (e.g., manual_entry, pipeline)
    pub capture_agent: Option<String>,
    /// Data type (text, image, audio, video, document)
    pub data_type: RawDataType,
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PipelineOutput {
    pub summary: Option<String>,
    /// Event type (e.g., note, task, research, photo)
    #[serde(default)]
    pub type_: Option<String>,
    /// Event subtype (e.g., summarize, reasoning, image_caption)
    #[serde(default)]
    pub subtype: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub entities: Vec<String>,
    pub confidence: Option<f64>,
}
