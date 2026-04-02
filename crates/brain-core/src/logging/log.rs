//! Log entry data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Log severity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "trace"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
        }
    }
}

/// Categories of logs
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LogType {
    /// CRUD operations on entities
    Crud,
    /// AI processing timing and results
    AiProcessing,
    /// Pipeline queue operations
    Pipeline,
    /// System events (startup, shutdown, config changes)
    System,
    /// Tag operations
    Tag,
    /// Future: proactive cognition logging
    Cognition,
    /// Future: system evaluation/metrics
    Evaluation,
    /// Catch-all for custom logging
    Custom,
}

impl std::fmt::Display for LogType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogType::Crud => write!(f, "crud"),
            LogType::AiProcessing => write!(f, "ai_processing"),
            LogType::Pipeline => write!(f, "pipeline"),
            LogType::System => write!(f, "system"),
            LogType::Tag => write!(f, "tag"),
            LogType::Cognition => write!(f, "cognition"),
            LogType::Evaluation => write!(f, "evaluation"),
            LogType::Custom => write!(f, "custom"),
        }
    }
}

/// CRUD operations
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CrudOperation {
    Create,
    Read,
    Update,
    Delete,
    Search,
    List,
}

impl std::fmt::Display for CrudOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CrudOperation::Create => write!(f, "create"),
            CrudOperation::Read => write!(f, "read"),
            CrudOperation::Update => write!(f, "update"),
            CrudOperation::Delete => write!(f, "delete"),
            CrudOperation::Search => write!(f, "search"),
            CrudOperation::List => write!(f, "list"),
        }
    }
}

/// Target entity types for logging
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetType {
    Event,
    Entity,
    Tag,
    PipelineTask,
    Config,
    System,
}

impl std::fmt::Display for TargetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetType::Event => write!(f, "event"),
            TargetType::Entity => write!(f, "entity"),
            TargetType::Tag => write!(f, "tag"),
            TargetType::PipelineTask => write!(f, "pipeline_task"),
            TargetType::Config => write!(f, "config"),
            TargetType::System => write!(f, "system"),
        }
    }
}

/// Context about the source of an operation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LogSource {
    pub device: Option<String>,
    pub channel: Option<String>,
    pub agent: Option<String>,
}

/// Metadata for AI processing logs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AiProcessingMetadata {
    pub file_size_bytes: Option<u64>,
    pub data_type: Option<String>,
    pub model_name: Option<String>,
    pub input_path: Option<String>,
    pub task_type: Option<String>,
    pub success: bool,
    pub error: Option<String>,
    #[serde(default)]
    pub output_summary_length: Option<usize>,
}

/// Metadata for CRUD operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CrudMetadata {
    pub before: Option<JsonValue>,
    pub after: Option<JsonValue>,
    pub query: Option<String>,
    pub result_count: Option<usize>,
}

/// Metadata for pipeline operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PipelineMetadata {
    pub task_id: Option<String>,
    pub task_type: Option<String>,
    pub from_state: Option<String>,
    pub to_state: Option<String>,
}

/// Metadata for ingest operations (file copied to raw data)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct IngestMetadata {
    pub source_path: Option<String>,
    pub dest_path: Option<String>,
    pub data_type: Option<String>,
    pub file_size_bytes: Option<u64>,
}

/// Metadata for system events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SystemMetadata {
    pub component: String,
    pub action: String,
    pub version: Option<String>,
}

/// Unified log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub log_type: LogType,
    pub operation: String,
    pub target_type: TargetType,
    pub target_id: Option<String>,
    pub source: LogSource,
    pub success: bool,
    pub error_message: Option<String>,
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub metadata: Option<JsonValue>,
}

impl LogEntry {
    pub fn new(log_type: LogType, operation: impl Into<String>, target_type: TargetType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            level: LogLevel::Info,
            log_type,
            operation: operation.into(),
            target_type,
            target_id: None,
            source: LogSource::default(),
            success: true,
            error_message: None,
            duration_ms: None,
            metadata: None,
        }
    }

    pub fn with_target_id(mut self, id: impl Into<String>) -> Self {
        self.target_id = Some(id.into());
        self
    }

    pub fn with_source(mut self, source: LogSource) -> Self {
        self.source = source;
        self
    }

    pub fn with_duration_ms(mut self, ms: u64) -> Self {
        self.duration_ms = Some(ms);
        self
    }

    pub fn with_metadata<T: Serialize>(mut self, metadata: T) -> Self {
        self.metadata = serde_json::to_value(metadata).ok();
        self
    }

    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.success = false;
        self.error_message = Some(error.into());
        self
    }

    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_entry_builder() {
        let entry = LogEntry::new(
            LogType::Crud,
            CrudOperation::Create.to_string(),
            TargetType::Event,
        )
        .with_target_id("evt-123")
        .with_duration_ms(45)
        .with_source(LogSource {
            device: Some("PC".to_string()),
            channel: Some("CLI".to_string()),
            agent: Some("manual_entry".to_string()),
        });

        assert_eq!(entry.log_type, LogType::Crud);
        assert_eq!(entry.operation, "create");
        assert_eq!(entry.target_type, TargetType::Event);
        assert_eq!(entry.target_id, Some("evt-123".to_string()));
        assert_eq!(entry.duration_ms, Some(45));
        assert!(entry.success);
        assert!(entry.error_message.is_none());
    }

    #[test]
    fn test_log_entry_error() {
        let entry = LogEntry::new(LogType::AiProcessing, "analyze", TargetType::PipelineTask)
            .with_error("Connection refused");

        assert!(!entry.success);
        assert_eq!(entry.error_message, Some("Connection refused".to_string()));
    }

    #[test]
    fn test_log_types_display() {
        assert_eq!(LogType::AiProcessing.to_string(), "ai_processing");
        assert_eq!(CrudOperation::Update.to_string(), "update");
        assert_eq!(TargetType::Entity.to_string(), "entity");
    }
}
