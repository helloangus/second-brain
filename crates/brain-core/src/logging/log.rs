//! Log entry data structures.
//!
//! Central module for Second Brain's structured logging system. Provides unified
//! `LogEntry` format supporting CRUD, AI processing, pipeline, and system events
//! with type-specific metadata stored as JSON.
//!
//! Organized as: log.rs (data), repo.rs (persistence), logger.rs (high-level API).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Log severity levels (trace/debug/info/warn/error). Default is Info.
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

impl LogLevel {
    /// Chinese display name.
    pub fn display_zh(&self) -> &'static str {
        match self {
            LogLevel::Trace => "追踪",
            LogLevel::Debug => "调试",
            LogLevel::Info => "信息",
            LogLevel::Warn => "警告",
            LogLevel::Error => "错误",
        }
    }
}

/// Categories of logs by functional area.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LogType {
    /// CRUD operations on events, entities, tags
    Crud,
    /// AI model inference (images, documents, etc.) with timing and results
    AiProcessing,
    /// Pipeline task state transitions (pending/processing/done)
    Pipeline,
    /// Application lifecycle (startup, shutdown, config changes)
    System,
    /// Tag attachment/detachment
    Tag,
    /// Future: proactive cognition
    Cognition,
    /// Future: evaluation/metrics
    Evaluation,
    /// Catch-all for custom events
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

impl LogType {
    /// Chinese display name.
    pub fn display_zh(&self) -> &'static str {
        match self {
            LogType::Crud => "增删改查",
            LogType::AiProcessing => "AI处理",
            LogType::Pipeline => "流水线",
            LogType::System => "系统",
            LogType::Tag => "标签",
            LogType::Cognition => "认知",
            LogType::Evaluation => "评估",
            LogType::Custom => "自定义",
        }
    }
}

/// CRUD operation types (create/read/update/delete/search/list).
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

impl CrudOperation {
    /// Chinese display name.
    pub fn display_zh(&self) -> &'static str {
        match self {
            CrudOperation::Create => "创建",
            CrudOperation::Read => "读取",
            CrudOperation::Update => "更新",
            CrudOperation::Delete => "删除",
            CrudOperation::Search => "搜索",
            CrudOperation::List => "列表",
        }
    }
}

/// Target entity types for log entries.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetType {
    /// Time-based record (photo, note, task)
    Event,
    /// Long-lived entity (person, place, project)
    Entity,
    /// Tag attached to events
    Tag,
    /// AI processing task in pipeline
    PipelineTask,
    /// System configuration
    Config,
    /// System-level events
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

impl TargetType {
    /// Chinese display name.
    pub fn display_zh(&self) -> &'static str {
        match self {
            TargetType::Event => "事件",
            TargetType::Entity => "实体",
            TargetType::Tag => "标签",
            TargetType::PipelineTask => "任务",
            TargetType::Config => "配置",
            TargetType::System => "系统",
        }
    }
}

/// Source context (device/channel/agent) for tracing operation origins.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LogSource {
    pub device: Option<String>,
    pub channel: Option<String>,
    pub agent: Option<String>,
}

/// Metadata for AI processing: input file info, model used, duration, success.
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

/// Metadata for CRUD: before/after states, query, result count.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CrudMetadata {
    pub before: Option<JsonValue>,
    pub after: Option<JsonValue>,
    pub query: Option<String>,
    pub result_count: Option<usize>,
}

/// Metadata for pipeline: task ID, type, state transition (from → to).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PipelineMetadata {
    pub task_id: Option<String>,
    pub task_type: Option<String>,
    pub from_state: Option<String>,
    pub to_state: Option<String>,
}

/// Metadata for file ingest: source path, dest path, file size.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct IngestMetadata {
    pub source_path: Option<String>,
    pub dest_path: Option<String>,
    pub data_type: Option<String>,
    pub file_size_bytes: Option<u64>,
}

/// Metadata for system events: component, action, version.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SystemMetadata {
    pub component: String,
    pub action: String,
    pub version: Option<String>,
}

/// Unified log entry with builder pattern.
///
/// # Builder Example
/// ```ignore
/// use crate::logging::log::{LogEntry, LogType, TargetType, LogLevel, LogSource};
///
/// let entry = LogEntry::new(LogType::Crud, "create", TargetType::Event)
///     .with_target_id("evt-001")
///     .with_duration_ms(45)
///     .with_level(LogLevel::Debug);
/// ```
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
    /// Creates entry with defaults: UUID, Utc::now(), level=Info, success=true.
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

    /// Sets target entity ID.
    pub fn with_target_id(mut self, id: impl Into<String>) -> Self {
        self.target_id = Some(id.into());
        self
    }

    /// Sets source context (device/channel/agent).
    pub fn with_source(mut self, source: LogSource) -> Self {
        self.source = source;
        self
    }

    /// Sets operation duration in milliseconds.
    pub fn with_duration_ms(mut self, ms: u64) -> Self {
        self.duration_ms = Some(ms);
        self
    }

    /// Sets type-specific metadata (serialized to JSON).
    pub fn with_metadata<T: Serialize>(mut self, metadata: T) -> Self {
        self.metadata = serde_json::to_value(metadata).ok();
        self
    }

    /// Sets success=false and records error message.
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.success = false;
        self.error_message = Some(error.into());
        self
    }

    /// Sets log level (default is Info).
    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // LogEntry Builder Tests
    // ========================================================================

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

    // ========================================================================
    // Enum Display and Chinese Name Tests
    // ========================================================================

    #[test]
    fn test_log_level_display() {
        assert_eq!(LogLevel::Trace.to_string(), "trace");
        assert_eq!(LogLevel::Debug.to_string(), "debug");
        assert_eq!(LogLevel::Info.to_string(), "info");
        assert_eq!(LogLevel::Warn.to_string(), "warn");
        assert_eq!(LogLevel::Error.to_string(), "error");
    }

    #[test]
    fn test_log_level_display_zh() {
        assert_eq!(LogLevel::Trace.display_zh(), "追踪");
        assert_eq!(LogLevel::Debug.display_zh(), "调试");
        assert_eq!(LogLevel::Info.display_zh(), "信息");
        assert_eq!(LogLevel::Warn.display_zh(), "警告");
        assert_eq!(LogLevel::Error.display_zh(), "错误");
    }

    #[test]
    fn test_log_type_display() {
        assert_eq!(LogType::Crud.to_string(), "crud");
        assert_eq!(LogType::AiProcessing.to_string(), "ai_processing");
        assert_eq!(LogType::Pipeline.to_string(), "pipeline");
        assert_eq!(LogType::System.to_string(), "system");
        assert_eq!(LogType::Tag.to_string(), "tag");
        assert_eq!(LogType::Cognition.to_string(), "cognition");
        assert_eq!(LogType::Evaluation.to_string(), "evaluation");
        assert_eq!(LogType::Custom.to_string(), "custom");
    }

    #[test]
    fn test_log_type_display_zh() {
        assert_eq!(LogType::Crud.display_zh(), "增删改查");
        assert_eq!(LogType::AiProcessing.display_zh(), "AI处理");
        assert_eq!(LogType::Pipeline.display_zh(), "流水线");
        assert_eq!(LogType::System.display_zh(), "系统");
        assert_eq!(LogType::Tag.display_zh(), "标签");
    }

    #[test]
    fn test_crud_operation_display() {
        assert_eq!(CrudOperation::Create.to_string(), "create");
        assert_eq!(CrudOperation::Read.to_string(), "read");
        assert_eq!(CrudOperation::Update.to_string(), "update");
        assert_eq!(CrudOperation::Delete.to_string(), "delete");
        assert_eq!(CrudOperation::Search.to_string(), "search");
        assert_eq!(CrudOperation::List.to_string(), "list");
    }

    #[test]
    fn test_crud_operation_display_zh() {
        assert_eq!(CrudOperation::Create.display_zh(), "创建");
        assert_eq!(CrudOperation::Read.display_zh(), "读取");
        assert_eq!(CrudOperation::Update.display_zh(), "更新");
        assert_eq!(CrudOperation::Delete.display_zh(), "删除");
        assert_eq!(CrudOperation::Search.display_zh(), "搜索");
        assert_eq!(CrudOperation::List.display_zh(), "列表");
    }

    #[test]
    fn test_target_type_display() {
        assert_eq!(TargetType::Event.to_string(), "event");
        assert_eq!(TargetType::Entity.to_string(), "entity");
        assert_eq!(TargetType::Tag.to_string(), "tag");
        assert_eq!(TargetType::PipelineTask.to_string(), "pipeline_task");
        assert_eq!(TargetType::Config.to_string(), "config");
        assert_eq!(TargetType::System.to_string(), "system");
    }

    #[test]
    fn test_target_type_display_zh() {
        assert_eq!(TargetType::Event.display_zh(), "事件");
        assert_eq!(TargetType::Entity.display_zh(), "实体");
        assert_eq!(TargetType::Tag.display_zh(), "标签");
        assert_eq!(TargetType::PipelineTask.display_zh(), "任务");
        assert_eq!(TargetType::Config.display_zh(), "配置");
        assert_eq!(TargetType::System.display_zh(), "系统");
    }

    // ========================================================================
    // LogEntry Builder Pattern Tests
    // ========================================================================

    #[test]
    fn test_log_entry_with_metadata() {
        let metadata = serde_json::json!({
            "file_size_bytes": 1024,
            "model": "gpt-4"
        });
        let entry = LogEntry::new(LogType::AiProcessing, "analyze", TargetType::PipelineTask)
            .with_metadata(metadata.clone());

        assert!(entry.metadata.is_some());
        let stored = entry.metadata.unwrap();
        assert_eq!(stored["file_size_bytes"], 1024);
        assert_eq!(stored["model"], "gpt-4");
    }

    #[test]
    fn test_log_entry_default_level_is_info() {
        let entry = LogEntry::new(LogType::System, "startup", TargetType::System);
        assert_eq!(entry.level, LogLevel::Info);
    }

    #[test]
    fn test_log_entry_with_level() {
        let entry =
            LogEntry::new(LogType::System, "debug", TargetType::System).with_level(LogLevel::Debug);
        assert_eq!(entry.level, LogLevel::Debug);
    }

    #[test]
    fn test_log_entry_default_success_is_true() {
        let entry = LogEntry::new(LogType::Crud, "create", TargetType::Event);
        assert!(entry.success);
        assert!(entry.error_message.is_none());
    }

    #[test]
    fn test_log_entry_with_error_sets_success_false() {
        let entry = LogEntry::new(LogType::AiProcessing, "analyze", TargetType::PipelineTask)
            .with_error("Connection refused");
        assert!(!entry.success);
        assert_eq!(entry.error_message, Some("Connection refused".to_string()));
    }

    #[test]
    fn test_log_entry_source_defaults_to_empty() {
        let entry = LogEntry::new(LogType::Crud, "create", TargetType::Event);
        assert!(entry.source.device.is_none());
        assert!(entry.source.channel.is_none());
        assert!(entry.source.agent.is_none());
    }

    #[test]
    fn test_log_entry_target_id_is_optional() {
        let entry = LogEntry::new(LogType::System, "startup", TargetType::System);
        assert!(entry.target_id.is_none());

        let entry_with_id = entry.with_target_id("sys-001");
        assert_eq!(entry_with_id.target_id, Some("sys-001".to_string()));
    }

    #[test]
    fn test_log_entry_duration_is_optional() {
        let entry = LogEntry::new(LogType::Crud, "create", TargetType::Event);
        assert!(entry.duration_ms.is_none());

        let entry_with_duration = entry.with_duration_ms(100);
        assert_eq!(entry_with_duration.duration_ms, Some(100));
    }

    #[test]
    fn test_log_entry_uuid_is_unique() {
        let entry1 = LogEntry::new(LogType::Crud, "create", TargetType::Event);
        let entry2 = LogEntry::new(LogType::Crud, "create", TargetType::Event);
        assert_ne!(entry1.id, entry2.id);
    }

    // ========================================================================
    // Metadata Struct Tests
    // ========================================================================

    #[test]
    fn test_ai_processing_metadata_serialization() {
        let metadata = AiProcessingMetadata {
            file_size_bytes: Some(1024),
            data_type: Some("photo".to_string()),
            model_name: Some("gpt-4-vision".to_string()),
            input_path: Some("/path/to/image.jpg".to_string()),
            task_type: None,
            success: true,
            error: None,
            output_summary_length: Some(150),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: AiProcessingMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.file_size_bytes, Some(1024));
        assert_eq!(deserialized.model_name, Some("gpt-4-vision".to_string()));
        assert!(deserialized.success);
    }

    #[test]
    fn test_crud_metadata_serialization() {
        let metadata = CrudMetadata {
            before: Some(serde_json::json!({"name": "old"})),
            after: Some(serde_json::json!({"name": "new"})),
            query: Some("name = 'test'".to_string()),
            result_count: Some(5),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: CrudMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.result_count, Some(5));
    }

    #[test]
    fn test_pipeline_metadata_serialization() {
        let metadata = PipelineMetadata {
            task_id: Some("task-001".to_string()),
            task_type: Some("analysis".to_string()),
            from_state: Some("pending".to_string()),
            to_state: Some("processing".to_string()),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: PipelineMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.from_state, Some("pending".to_string()));
        assert_eq!(deserialized.to_state, Some("processing".to_string()));
    }

    #[test]
    fn test_ingest_metadata_serialization() {
        let metadata = IngestMetadata {
            source_path: Some("/data/raw/input.jpg".to_string()),
            dest_path: Some("/data/raw/2024/04/input.jpg".to_string()),
            data_type: Some("photo".to_string()),
            file_size_bytes: Some(204800),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: IngestMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.file_size_bytes, Some(204800));
    }

    #[test]
    fn test_system_metadata_serialization() {
        let metadata = SystemMetadata {
            component: "brain-cli".to_string(),
            action: "startup".to_string(),
            version: Some("1.0.0".to_string()),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: SystemMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.component, "brain-cli");
        assert_eq!(deserialized.version, Some("1.0.0".to_string()));
    }

    // ========================================================================
    // LogSource Tests
    // ========================================================================

    #[test]
    fn test_log_source_default() {
        let source = LogSource::default();
        assert!(source.device.is_none());
        assert!(source.channel.is_none());
        assert!(source.agent.is_none());
    }

    #[test]
    fn test_log_source_with_values() {
        let source = LogSource {
            device: Some("desktop".to_string()),
            channel: Some("cli".to_string()),
            agent: Some("brain".to_string()),
        };
        assert_eq!(source.device.as_deref(), Some("desktop"));
        assert_eq!(source.channel.as_deref(), Some("cli"));
        assert_eq!(source.agent.as_deref(), Some("brain"));
    }

    // ========================================================================
    // Default Attribute Tests
    // ========================================================================

    #[test]
    fn test_log_level_default_is_info() {
        let default_level = LogLevel::default();
        assert_eq!(default_level, LogLevel::Info);
    }
}
