//! Logger - convenient wrapper for logging operations
//!
//! Logger writes to a separate log database that can be rotated weekly or monthly.

use crate::error::Result;
use crate::logging::log::{
    AiProcessingMetadata, CrudMetadata, CrudOperation, LogEntry, LogLevel, LogSource, LogType,
    PipelineMetadata, TargetType,
};
use crate::logging::repo::LogRepository;
use crate::BrainConfig;
use rusqlite::Connection;

/// Logger - writes to a separate log database with rotation support
pub struct Logger {
    config: BrainConfig,
    min_level: LogLevel,
}

impl Logger {
    /// Create a new logger with the given config
    pub fn new(config: &BrainConfig) -> Self {
        Self {
            config: config.clone(),
            min_level: LogLevel::Info,
        }
    }

    /// Set minimum log level
    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.min_level = level;
        self
    }

    /// Get the current log database, creating it if necessary
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

    /// Log an event CRUD operation
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

    /// Log an entity CRUD operation
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

    /// Log a tag operation
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

    /// Log AI processing with timing and metadata
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

    /// Log pipeline task state transition
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

    /// Log ingest flow (file copied to raw data)
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

    /// Log queue add (task added to pending queue)
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

    /// Log system event
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

    /// Query logs by type from all log databases
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

    /// Query logs for a specific target from all log databases
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

    /// Get recent logs from all log databases
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

    /// Get AI processing statistics from all log databases
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
