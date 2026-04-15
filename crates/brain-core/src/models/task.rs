//! Task models for AI processing pipeline
//!
//! Data flows through the pipeline as [`PipelineTask`] entities: input is submitted,
//! AI adapter processes it, output is stored in [`PipelineOutput`].

use super::raw_data::RawDataType;
use serde::{Deserialize, Serialize};

/// Task type — what kind of AI processing to perform on the input data.
///
/// Each variant maps to a specific AI capability:
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    /// Analyze image content and generate a natural language description.
    ImageCaption,
    /// Detect and locate faces within an image.
    FaceDetection,
    /// Extract printed or handwritten text from an image (Optical Character Recognition).
    Ocr,
    /// Transcribe speech in an audio file to text (Automatic Speech Recognition).
    Asr,
    /// Identify which speaker is talking at which time in an audio recording.
    SpeakerDiarization,
    /// Generate a vector embedding for semantic search or similarity comparison.
    Embedding,
    #[default]
    /// Perform multi-step reasoning over text input.
    Reasoning,
    /// Condense long text into a short summary.
    Summarize,
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
            TaskType::Summarize => write!(f, "summarize"),
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
            "summarize" => Some(TaskType::Summarize),
            _ => None,
        }
    }

    /// Which [`RawDataType`] this task expects as input.
    pub fn data_type(&self) -> RawDataType {
        match self {
            TaskType::ImageCaption => RawDataType::Image,
            TaskType::FaceDetection => RawDataType::Image,
            TaskType::Ocr => RawDataType::Image,
            TaskType::Asr => RawDataType::Audio,
            TaskType::SpeakerDiarization => RawDataType::Audio,
            TaskType::Embedding => RawDataType::Text,
            TaskType::Reasoning => RawDataType::Text,
            TaskType::Summarize => RawDataType::Text,
        }
    }
}

/// Maps a [`TaskType`] to a list of recommended AI models that can handle it.
///
/// Used by the pipeline to select which model adapter to invoke.
/// TODO: wire this up when selecting models for new TaskType variants.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelRecommendation {
    pub task_type: TaskType,
    pub data_type: RawDataType,
    pub recommended_models: Vec<String>,
}

impl ModelRecommendation {
    /// Default model recommendations covering all currently defined [`TaskType`] variants.
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
            ModelRecommendation {
                task_type: TaskType::Summarize,
                data_type: RawDataType::Text,
                recommended_models: vec!["Qwen2.5".to_string()],
            },
        ]
    }
}

/// A unit of work in the AI pipeline: input data to be processed, along with its current status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

/// Lifecycle state of a pipeline task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    #[default]
    Pending,
    Processing,
    Done,
    Failed,
}

/// Describes the raw input data submitted to the pipeline for AI processing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PipelineInput {
    /// Path to the input file, relative to the configured raw data root.
    /// Includes the data-type prefix directory (e.g. `image/photo_001.jpg`).
    pub path: String,
    /// Input channel through which data entered the system (e.g. CLI, API, Web).
    #[serde(default)]
    pub channel: Option<String>,
    /// Device that created the original data (e.g. PC, iPhone, Server).
    #[serde(default)]
    pub device: Option<String>,
    /// Mechanism that captured the data (e.g. `manual_entry`, `pipeline`).
    #[serde(default)]
    pub capture_agent: Option<String>,
    /// Type of the raw input data.
    pub data_type: RawDataType,
    /// Arbitrary key-value metadata associated with this input (e.g. MIME type, file size).
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

/// Structured output produced by an AI adapter after processing a [`PipelineTask`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PipelineOutput {
    /// One-sentence summary of the processed input.
    #[serde(default)]
    pub summary: Option<String>,
    /// Extended text content — used when a summary is too short (e.g. full transcript,
    /// full OCR result, full generated description).
    #[serde(default)]
    pub extended: Option<String>,
    /// Classified event type derived from the input (e.g. `note`, `task`, `photo`).
    #[serde(default)]
    pub type_: Option<String>,
    /// Sub-type identifying the AI task that produced this output
    /// (e.g. `summarize`, `reasoning`, `image_caption`).
    #[serde(default)]
    pub subtype: Option<String>,
    /// Simple keyword tags generated by the AI (e.g. `["meeting", "quarterly", "budget"]`).
    #[serde(default)]
    pub tags: Vec<String>,
    /// High-level topics or themes (e.g. `["finance", "strategy"]`).
    #[serde(default)]
    pub topics: Vec<String>,
    /// Named entities extracted from the input (e.g. person names, organization names).
    #[serde(default)]
    pub entities: Vec<String>,
    /// Confidence score of the AI's overall assessment, in the range [0.0, 1.0].
    #[serde(default)]
    pub confidence: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------------------
    // TaskType tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_task_type_display_roundtrip() {
        for variant in [
            TaskType::ImageCaption,
            TaskType::FaceDetection,
            TaskType::Ocr,
            TaskType::Asr,
            TaskType::SpeakerDiarization,
            TaskType::Embedding,
            TaskType::Reasoning,
            TaskType::Summarize,
        ] {
            let s = variant.to_string();
            assert_eq!(
                TaskType::from_str(&s),
                Some(variant.clone()),
                "from_str(display({:?})) should roundtrip",
                variant
            );
        }
    }

    #[test]
    fn test_task_type_from_str_aliases() {
        assert_eq!(
            TaskType::from_str("image.analyze"),
            Some(TaskType::ImageCaption)
        );
        assert_eq!(
            TaskType::from_str("face.detect"),
            Some(TaskType::FaceDetection)
        );
        assert_eq!(TaskType::from_str("audio.transcribe"), Some(TaskType::Asr));
        assert_eq!(
            TaskType::from_str("speaker.diarize"),
            Some(TaskType::SpeakerDiarization)
        );
    }

    #[test]
    fn test_task_type_from_str_case_insensitive() {
        assert_eq!(TaskType::from_str("ASR"), Some(TaskType::Asr));
        assert_eq!(TaskType::from_str("Ocr"), Some(TaskType::Ocr));
        assert_eq!(TaskType::from_str("SUMMARIZE"), Some(TaskType::Summarize));
    }

    #[test]
    fn test_task_type_from_str_unknown() {
        assert_eq!(TaskType::from_str("unknown_task"), None);
        assert_eq!(TaskType::from_str(""), None);
    }

    #[test]
    fn test_task_type_data_type() {
        assert_eq!(TaskType::ImageCaption.data_type(), RawDataType::Image);
        assert_eq!(TaskType::FaceDetection.data_type(), RawDataType::Image);
        assert_eq!(TaskType::Ocr.data_type(), RawDataType::Image);
        assert_eq!(TaskType::Asr.data_type(), RawDataType::Audio);
        assert_eq!(TaskType::SpeakerDiarization.data_type(), RawDataType::Audio);
        assert_eq!(TaskType::Embedding.data_type(), RawDataType::Text);
        assert_eq!(TaskType::Reasoning.data_type(), RawDataType::Text);
        assert_eq!(TaskType::Summarize.data_type(), RawDataType::Text);
    }

    #[test]
    fn test_task_type_default() {
        assert_eq!(TaskType::default(), TaskType::Reasoning);
    }

    // ---------------------------------------------------------------------------
    // TaskStatus tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_task_status_default() {
        assert_eq!(TaskStatus::default(), TaskStatus::Pending);
    }

    #[test]
    fn test_task_status_serde_roundtrip() {
        for status in [
            TaskStatus::Pending,
            TaskStatus::Processing,
            TaskStatus::Done,
            TaskStatus::Failed,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let parsed: TaskStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, status, "serde roundtrip for {:?}", status);
        }
    }

    // ---------------------------------------------------------------------------
    // PipelineInput tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_pipeline_input_serde_roundtrip() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("mime_type".to_string(), "image/jpeg".to_string());

        let input = PipelineInput {
            path: "image/photo_001.jpg".to_string(),
            channel: Some("CLI".to_string()),
            device: Some("iPhone".to_string()),
            capture_agent: Some("pipeline".to_string()),
            data_type: RawDataType::Image,
            metadata,
        };

        let json = serde_json::to_string(&input).unwrap();
        let parsed: PipelineInput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, input);
    }

    #[test]
    fn test_pipeline_input_missing_optional_fields() {
        // Optional fields should default to None/empty when missing from JSON.
        let json = r#"{"path":"text/note.md","data_type":"text"}"#;
        let parsed: PipelineInput = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.path, "text/note.md");
        assert_eq!(parsed.channel, None);
        assert_eq!(parsed.device, None);
        assert_eq!(parsed.capture_agent, None);
        assert_eq!(parsed.metadata, std::collections::HashMap::new());
    }

    // ---------------------------------------------------------------------------
    // PipelineOutput tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_pipeline_output_serde_roundtrip() {
        let output = PipelineOutput {
            summary: Some("Meeting about Q1 budget".to_string()),
            extended: None,
            type_: Some("note".to_string()),
            subtype: Some("summarize".to_string()),
            tags: vec!["meeting".to_string(), "budget".to_string()],
            topics: vec!["finance".to_string()],
            entities: vec!["Alice".to_string(), "Bob".to_string()],
            confidence: Some(0.92),
        };

        let json = serde_json::to_string(&output).unwrap();
        let parsed: PipelineOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, output);
    }

    #[test]
    fn test_pipeline_output_missing_fields_default_to_empty() {
        // Collections default to empty Vec; summary/extended/type/subtype/confidence default to None.
        let json = r#"{}"#;
        let parsed: PipelineOutput = serde_json::from_str(json).unwrap();
        assert!(parsed.summary.is_none());
        assert!(parsed.extended.is_none());
        assert!(parsed.type_.is_none());
        assert!(parsed.subtype.is_none());
        assert!(parsed.tags.is_empty());
        assert!(parsed.topics.is_empty());
        assert!(parsed.entities.is_empty());
        assert!(parsed.confidence.is_none());
    }

    // ---------------------------------------------------------------------------
    // PipelineTask tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_pipeline_task_data_type() {
        let task = PipelineTask {
            id: "task-001".to_string(),
            task: TaskType::Asr,
            input: PipelineInput {
                path: "audio/meeting.mp3".to_string(),
                channel: None,
                device: None,
                capture_agent: None,
                data_type: RawDataType::Audio,
                metadata: std::collections::HashMap::new(),
            },
            output: None,
            status: TaskStatus::Pending,
        };
        assert_eq!(task.data_type(), RawDataType::Audio);
    }

    #[test]
    fn test_pipeline_task_with_output_roundtrip() {
        let task = PipelineTask {
            id: "task-002".to_string(),
            task: TaskType::Summarize,
            input: PipelineInput {
                path: "text/article.md".to_string(),
                channel: Some("web".to_string()),
                device: None,
                capture_agent: None,
                data_type: RawDataType::Text,
                metadata: std::collections::HashMap::new(),
            },
            output: Some(PipelineOutput {
                summary: Some("Article about Rust async".to_string()),
                extended: None,
                type_: Some("note".to_string()),
                subtype: Some("summarize".to_string()),
                tags: vec!["rust".to_string(), "async".to_string()],
                topics: vec![],
                entities: vec![],
                confidence: Some(0.88),
            }),
            status: TaskStatus::Done,
        };

        let json = serde_json::to_string(&task).unwrap();
        let parsed: PipelineTask = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, task);
    }

    // ---------------------------------------------------------------------------
    // ModelRecommendation tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_model_recommendation_get_defaults_count() {
        let defaults = ModelRecommendation::get_defaults();
        // One entry per TaskType variant
        assert_eq!(defaults.len(), 8);
    }

    #[test]
    fn test_model_recommendation_every_task_type_has_recommendation() {
        let defaults = ModelRecommendation::get_defaults();
        for variant in [
            TaskType::ImageCaption,
            TaskType::FaceDetection,
            TaskType::Ocr,
            TaskType::Asr,
            TaskType::SpeakerDiarization,
            TaskType::Embedding,
            TaskType::Reasoning,
            TaskType::Summarize,
        ] {
            assert!(
                defaults.iter().any(|r| r.task_type == variant),
                "TaskType::{:?} should have a ModelRecommendation",
                variant
            );
        }
    }

    #[test]
    fn test_model_recommendation_data_type_matches_task() {
        for rec in ModelRecommendation::get_defaults() {
            assert_eq!(
                rec.data_type,
                rec.task_type.data_type(),
                "ModelRecommendation data_type should match task_type.data_type() for {:?}",
                rec.task_type
            );
        }
    }
}
