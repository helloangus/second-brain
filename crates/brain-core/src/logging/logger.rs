//! Logger - convenient wrapper for logging operations
//!
//! Logger writes to a separate log database that can be rotated weekly or monthly.
//! This provides a high-level API for recording structured log entries about:
//! - CRUD operations on events, entities, and tags
//! - AI processing operations with timing and metadata
//! - Pipeline task state transitions
//! - System events (startup, shutdown, etc.)
//!
//! # Log Database Rotation
//!
//! The logger automatically writes to time-based log databases. The `BrainConfig`
//! determines the naming scheme via `log_db_path_for_time()`. Query methods like
//! `get_recent()` aggregate across all historical log databases.

use crate::error::Result;
use crate::logging::log::{
    AiProcessingMetadata, CrudMetadata, CrudOperation, LogEntry, LogLevel, LogSource, LogType,
    PipelineMetadata, TargetType,
};
use crate::logging::repo::LogRepository;
use crate::BrainConfig;
use rusqlite::Connection;

/// Logger - writes to a separate log database with rotation support.
///
/// The logger provides a convenient high-level API for recording structured
/// log entries. It manages database connections and routes entries to the
/// appropriate time-based log database.
pub struct Logger {
    /// Configuration used to determine log database paths
    config: BrainConfig,
    /// Minimum log level to record (currently informational only)
    min_level: LogLevel,
}

impl Logger {
    /// Creates a new logger with the given configuration.
    ///
    /// # Arguments
    /// * `config` - BrainConfig used to determine log database paths
    pub fn new(config: &BrainConfig) -> Self {
        Self {
            config: config.clone(),
            min_level: LogLevel::Info,
        }
    }

    /// Sets the minimum log level (builder pattern).
    ///
    /// # Arguments
    /// * `level` - Minimum log level to record
    ///
    /// Note: Currently this is informational only; all entries are logged regardless.
    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.min_level = level;
        self
    }

    /// Executes a closure with a connection to the current time-based log database.
    ///
    /// This internal helper opens (or creates) the appropriate log database
    /// for the current time period and runs any pending migrations before
    /// passing the connection to the callback.
    ///
    /// # Arguments
    /// * `f` - Closure that receives the SQLite connection and returns a Result
    fn with_current_log_repo<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&Connection) -> Result<R>,
    {
        let log_db_path = self.config.log_db_path_for_time();
        let conn = Connection::open(&log_db_path)?;

        // Run migrations to ensure schema exists
        crate::db::run_migrations(&conn)?;

        f(&conn)
    }

    /// Records a CRUD operation on an event.
    ///
    /// # Arguments
    /// * `op` - The CRUD operation (Create, Read, Update, Delete, Search, List)
    /// * `event_id` - ID of the event being operated on
    /// * `source` - Source information (device, channel, agent)
    /// * `duration_ms` - Operation duration in milliseconds
    pub fn log_event_crud(
        &self,
        op: CrudOperation,
        event_id: &str,
        source: &LogSource,
        duration_ms: u64,
    ) -> Result<()> {
        self.with_current_log_repo(|conn| {
            let repo = LogRepository::new(conn);
            let entry = LogEntry::new(LogType::Crud, op.to_string(), TargetType::Event)
                .with_target_id(event_id)
                .with_source(source.clone())
                .with_duration_ms(duration_ms);
            repo.insert(&entry)
        })
    }

    /// Records a CRUD operation on an entity (person, place, project, etc.).
    ///
    /// # Arguments
    /// * `op` - The CRUD operation
    /// * `entity_id` - ID of the entity being operated on
    /// * `source` - Source information
    /// * `duration_ms` - Operation duration in milliseconds
    pub fn log_entity_crud(
        &self,
        op: CrudOperation,
        entity_id: &str,
        source: &LogSource,
        duration_ms: u64,
    ) -> Result<()> {
        self.with_current_log_repo(|conn| {
            let repo = LogRepository::new(conn);
            let entry = LogEntry::new(LogType::Crud, op.to_string(), TargetType::Entity)
                .with_target_id(entity_id)
                .with_source(source.clone())
                .with_duration_ms(duration_ms);
            repo.insert(&entry)
        })
    }

    /// Records a tag operation (adding/removing tags from an event).
    ///
    /// # Arguments
    /// * `op` - The tag operation
    /// * `event_id` - ID of the event being tagged
    /// * `tag` - The tag name (currently unused in metadata)
    /// * `source` - Source information
    pub fn log_tag_op(
        &self,
        op: CrudOperation,
        event_id: &str,
        _tag: &str,
        source: &LogSource,
    ) -> Result<()> {
        self.with_current_log_repo(|conn| {
            let repo = LogRepository::new(conn);
            let metadata = CrudMetadata {
                before: None,
                after: None,
                query: None,
                result_count: None,
            };
            let entry = LogEntry::new(LogType::Tag, op.to_string(), TargetType::Tag)
                .with_target_id(event_id)
                .with_source(source.clone())
                .with_metadata(metadata);
            repo.insert(&entry)
        })
    }

    /// Records an AI processing operation (e.g., analyzing an image or document).
    ///
    /// # Arguments
    /// * `input_path` - Path to the input file being processed
    /// * `data_type` - Type of data (e.g., "photo", "note", "document")
    /// * `model_name` - AI model used (e.g., "gpt-4", "claude-3")
    /// * `duration_ms` - Processing duration in milliseconds
    /// * `success` - Whether the operation succeeded
    /// * `file_size_bytes` - Size of the input file in bytes
    /// * `error_msg` - Error message if the operation failed
    #[allow(clippy::too_many_arguments)]
    pub fn log_ai_processing(
        &self,
        input_path: &str,
        data_type: &str,
        model_name: &str,
        duration_ms: u64,
        success: bool,
        file_size_bytes: Option<u64>,
        error_msg: Option<&str>,
    ) -> Result<()> {
        self.with_current_log_repo(|conn| {
            let repo = LogRepository::new(conn);

            let metadata = AiProcessingMetadata {
                file_size_bytes,
                data_type: Some(data_type.to_string()),
                model_name: Some(model_name.to_string()),
                input_path: Some(input_path.to_string()),
                task_type: None,
                success,
                error: error_msg.map(|s| s.to_string()),
                output_summary_length: None,
            };

            let mut entry =
                LogEntry::new(LogType::AiProcessing, "analyze", TargetType::PipelineTask)
                    .with_duration_ms(duration_ms)
                    .with_metadata(metadata);

            if !success {
                entry = entry.with_error(error_msg.unwrap_or("Unknown error"));
            }

            repo.insert(&entry)
        })
    }

    /// Records a pipeline task state transition (e.g., pending -> processing -> done).
    ///
    /// # Arguments
    /// * `task_id` - Unique identifier for the task
    /// * `task_type` - Type of task (e.g., "image_analysis", "note_summary")
    /// * `from_state` - Previous state (None for new tasks)
    /// * `to_state` - New state (e.g., "pending", "processing", "done", "failed")
    pub fn log_pipeline_task(
        &self,
        task_id: &str,
        task_type: &str,
        from_state: &str,
        to_state: &str,
    ) -> Result<()> {
        self.with_current_log_repo(|conn| {
            let repo = LogRepository::new(conn);

            let metadata = PipelineMetadata {
                task_id: Some(task_id.to_string()),
                task_type: Some(task_type.to_string()),
                from_state: Some(from_state.to_string()),
                to_state: Some(to_state.to_string()),
            };

            let entry = LogEntry::new(LogType::Pipeline, "transition", TargetType::PipelineTask)
                .with_target_id(task_id)
                .with_metadata(metadata);

            repo.insert(&entry)
        })
    }

    /// Records a file ingest operation (file copied to raw data directory).
    ///
    /// # Arguments
    /// * `source_path` - Original file path
    /// * `dest_path` - Destination path in raw data directory
    /// * `data_type` - Type of data being ingested
    /// * `file_size_bytes` - Size of the file in bytes
    pub fn log_ingest_file(
        &self,
        source_path: &str,
        dest_path: &str,
        data_type: &str,
        file_size_bytes: Option<u64>,
    ) -> Result<()> {
        self.with_current_log_repo(|conn| {
            let repo = LogRepository::new(conn);

            let metadata = crate::logging::log::IngestMetadata {
                source_path: Some(source_path.to_string()),
                dest_path: Some(dest_path.to_string()),
                data_type: Some(data_type.to_string()),
                file_size_bytes,
            };

            let entry = LogEntry::new(LogType::Pipeline, "ingest_file", TargetType::PipelineTask)
                .with_metadata(metadata);

            repo.insert(&entry)
        })
    }

    /// Records a task being added to the processing queue.
    ///
    /// # Arguments
    /// * `task_id` - Unique identifier for the task
    /// * `task_type` - Type of task
    /// * `_input_path` - Path to input file (currently unused)
    pub fn log_queue_add(&self, task_id: &str, task_type: &str, _input_path: &str) -> Result<()> {
        self.with_current_log_repo(|conn| {
            let repo = LogRepository::new(conn);

            let metadata = PipelineMetadata {
                task_id: Some(task_id.to_string()),
                task_type: Some(task_type.to_string()),
                from_state: None,
                to_state: Some("pending".to_string()),
            };

            let entry = LogEntry::new(LogType::Pipeline, "queue_add", TargetType::PipelineTask)
                .with_target_id(task_id)
                .with_metadata(metadata);

            repo.insert(&entry)
        })
    }

    /// Records a system event (startup, shutdown, config changes, etc.).
    ///
    /// # Arguments
    /// * `component` - Name of the component (e.g., "brain-cli", "brain-indexerd")
    /// * `action` - Action performed (e.g., "startup", "shutdown", "config_reload")
    /// * `duration_ms` - Optional duration of the operation
    pub fn log_system(
        &self,
        component: &str,
        action: &str,
        duration_ms: Option<u64>,
    ) -> Result<()> {
        self.with_current_log_repo(|conn| {
            let repo = LogRepository::new(conn);

            let metadata = crate::logging::log::SystemMetadata {
                component: component.to_string(),
                action: action.to_string(),
                version: None,
            };

            let mut entry =
                LogEntry::new(LogType::System, action, TargetType::System).with_metadata(metadata);

            if let Some(ms) = duration_ms {
                entry = entry.with_duration_ms(ms);
            }

            repo.insert(&entry)
        })
    }

    /// Queries logs by type, aggregating from all historical log databases.
    ///
    /// # Arguments
    /// * `log_type` - Type of logs to query
    /// * `limit` - Maximum number of entries to return
    ///
    /// Results are sorted by timestamp descending and limited to the most recent entries.
    pub fn get_by_type(&self, log_type: LogType, limit: usize) -> Result<Vec<LogEntry>> {
        let mut all_entries = Vec::new();

        for db_path in self.config.iter_log_db_paths() {
            if let Ok(conn) = Connection::open(&db_path) {
                let repo = LogRepository::new(&conn);
                if let Ok(entries) = repo.query_by_type(log_type, limit) {
                    all_entries.extend(entries);
                }
            }
        }

        // Sort by timestamp descending and limit
        all_entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all_entries.truncate(limit);

        Ok(all_entries)
    }

    /// Queries logs for a specific target entity, aggregating from all databases.
    ///
    /// # Arguments
    /// * `target_type` - Type of entity to query
    /// * `target_id` - Specific entity ID
    /// * `limit` - Maximum number of entries to return
    pub fn get_for_target(
        &self,
        target_type: TargetType,
        target_id: &str,
        limit: usize,
    ) -> Result<Vec<LogEntry>> {
        let mut all_entries = Vec::new();

        for db_path in self.config.iter_log_db_paths() {
            if let Ok(conn) = Connection::open(&db_path) {
                let repo = LogRepository::new(&conn);
                if let Ok(entries) = repo.query_for_target(target_type, target_id, limit) {
                    all_entries.extend(entries);
                }
            }
        }

        all_entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all_entries.truncate(limit);

        Ok(all_entries)
    }

    /// Gets the most recent logs across all historical log databases.
    ///
    /// # Arguments
    /// * `limit` - Maximum number of entries to return
    ///
    /// This is useful for displaying recent activity in a dashboard.
    pub fn get_recent(&self, limit: usize) -> Result<Vec<LogEntry>> {
        let mut all_entries = Vec::new();

        for db_path in self.config.iter_log_db_paths() {
            if let Ok(conn) = Connection::open(&db_path) {
                let repo = LogRepository::new(&conn);
                if let Ok(entries) = repo.query_recent(limit) {
                    all_entries.extend(entries);
                }
            }
        }

        all_entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all_entries.truncate(limit);

        Ok(all_entries)
    }

    /// Gets aggregated AI processing statistics across all historical log databases.
    ///
    /// # Arguments
    /// * `days` - Number of days to look back
    ///
    /// Returns counts of total, successful, and failed operations,
    /// plus average processing duration in milliseconds.
    pub fn get_ai_stats(&self, days: u32) -> Result<crate::logging::repo::AiProcessingStats> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days as i64);
        let mut total = 0i64;
        let mut successful = 0i64;
        let mut failed = 0i64;
        let mut duration_sum = 0.0;
        let mut count = 0i64;

        for db_path in self.config.iter_log_db_paths() {
            if let Ok(conn) = Connection::open(&db_path) {
                let repo = LogRepository::new(&conn);

                // Simple aggregation query
                if let Ok(entries) = repo.query_by_timerange(cutoff, chrono::Utc::now(), 10000) {
                    for entry in entries {
                        if entry.log_type == LogType::AiProcessing {
                            total += 1;
                            if entry.success {
                                successful += 1;
                            } else {
                                failed += 1;
                            }
                            if let Some(ms) = entry.duration_ms {
                                duration_sum += ms as f64;
                                count += 1;
                            }
                        }
                    }
                }
            }
        }

        let avg_duration = if count > 0 {
            duration_sum / count as f64
        } else {
            0.0
        };

        Ok(crate::logging::repo::AiProcessingStats {
            total_operations: total as u64,
            successful_operations: successful as u64,
            failed_operations: failed as u64,
            avg_duration_ms: avg_duration,
            by_data_type: std::collections::HashMap::new(),
            by_model: std::collections::HashMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BrainConfig;
    use tempfile::TempDir;

    /// Creates a test logger with a temporary log directory.
    fn create_test_logger() -> (Logger, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = BrainConfig {
            log_db_path: temp_dir.path().join("logs"),
            log_rotation: "monthly".to_string(),
            ..Default::default()
        };
        (Logger::new(&config), temp_dir)
    }

    #[test]
    fn test_logger_new() {
        let (logger, _temp) = create_test_logger();
        // Logger should be created without error
        assert_eq!(logger.min_level, LogLevel::Info);
    }

    #[test]
    fn test_logger_with_level() {
        let (logger, _temp) = create_test_logger();
        let logger = logger.with_level(LogLevel::Debug);
        assert_eq!(logger.min_level, LogLevel::Debug);
    }

    #[test]
    fn test_log_event_crud() {
        let (logger, _temp) = create_test_logger();
        let source = LogSource {
            device: Some("test-device".to_string()),
            channel: Some("test".to_string()),
            agent: Some("unit-test".to_string()),
        };

        let result = logger.log_event_crud(CrudOperation::Create, "evt-test-001", &source, 50);

        assert!(result.is_ok());
    }

    #[test]
    fn test_log_entity_crud() {
        let (logger, _temp) = create_test_logger();
        let source = LogSource::default();

        let result = logger.log_entity_crud(CrudOperation::Read, "ent-test-001", &source, 30);

        assert!(result.is_ok());
    }

    #[test]
    fn test_log_ai_processing_success() {
        let (logger, _temp) = create_test_logger();

        let result = logger.log_ai_processing(
            "/path/to/image.jpg",
            "photo",
            "gpt-4-vision",
            1500,
            true,
            Some(102400),
            None,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_log_ai_processing_failure() {
        let (logger, _temp) = create_test_logger();

        let result = logger.log_ai_processing(
            "/path/to/image.jpg",
            "photo",
            "gpt-4-vision",
            5000,
            false,
            Some(102400),
            Some("Connection timeout"),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_log_pipeline_task() {
        let (logger, _temp) = create_test_logger();

        let result =
            logger.log_pipeline_task("task-001", "image_analysis", "pending", "processing");

        assert!(result.is_ok());
    }

    #[test]
    fn test_log_ingest_file() {
        let (logger, _temp) = create_test_logger();

        let result = logger.log_ingest_file(
            "/data/raw/photo_001.jpg",
            "/data/raw/2024/04/photo_001.jpg",
            "photo",
            Some(204800),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_log_queue_add() {
        let (logger, _temp) = create_test_logger();

        let result = logger.log_queue_add("task-002", "note_summary", "/data/notes/test.md");

        assert!(result.is_ok());
    }

    #[test]
    fn test_log_system() {
        let (logger, _temp) = create_test_logger();

        let result = logger.log_system("brain-cli", "startup", Some(100));

        assert!(result.is_ok());
    }

    #[test]
    fn test_log_tag_op() {
        let (logger, _temp) = create_test_logger();
        let source = LogSource::default();

        let result = logger.log_tag_op(CrudOperation::Create, "evt-tag-test", "rust", &source);

        assert!(result.is_ok());
    }

    #[test]
    fn test_get_recent_empty() {
        let (logger, _temp) = create_test_logger();

        let results = logger.get_recent(10);
        assert!(results.is_ok());
        assert!(results.unwrap().is_empty());
    }

    #[test]
    fn test_get_by_type_after_logging() {
        let (logger, _temp) = create_test_logger();

        // Log an event CRUD
        logger
            .log_event_crud(
                CrudOperation::Create,
                "evt-query-test",
                &LogSource::default(),
                50,
            )
            .unwrap();

        let results = logger.get_by_type(LogType::Crud, 10);
        assert!(results.is_ok());
        let entries = results.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].operation, "create");
    }

    #[test]
    fn test_get_ai_stats_empty() {
        let (logger, _temp) = create_test_logger();

        let stats = logger.get_ai_stats(7);
        assert!(stats.is_ok());
        let s = stats.unwrap();
        assert_eq!(s.total_operations, 0);
        assert_eq!(s.successful_operations, 0);
        assert_eq!(s.failed_operations, 0);
    }

    #[test]
    fn test_get_ai_stats_with_data() {
        let (logger, _temp) = create_test_logger();

        // Log successful AI processing
        logger
            .log_ai_processing(
                "/path/to/doc.pdf",
                "document",
                "claude-3",
                2000,
                true,
                Some(51200),
                None,
            )
            .unwrap();

        // Log failed AI processing
        logger
            .log_ai_processing(
                "/path/to/img.jpg",
                "photo",
                "gpt-4-vision",
                3000,
                false,
                Some(102400),
                Some("Rate limit exceeded"),
            )
            .unwrap();

        let stats = logger.get_ai_stats(7).unwrap();
        assert_eq!(stats.total_operations, 2);
        assert_eq!(stats.successful_operations, 1);
        assert_eq!(stats.failed_operations, 1);
    }
}
