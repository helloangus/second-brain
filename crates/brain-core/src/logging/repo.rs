//! Log repository for database operations
//!
//! This module provides the data access layer for log entries stored in SQLite.
//! Each log entry contains structured information about system operations including
//! CRUD actions, AI processing events, pipeline state transitions, and system events.
//!
//! # Database Schema
//!
//! The `logs` table contains:
//! - `id` (TEXT PRIMARY KEY) - UUID for the log entry
//! - `timestamp` (INTEGER) - Unix timestamp
//! - `level` (TEXT) - Log level (trace/debug/info/warn/error)
//! - `log_type` (TEXT) - Category of log (crud/ai_processing/pipeline/system/tag/...)
//! - `operation` (TEXT) - Specific operation name
//! - `target_type` (TEXT) - Type of entity being operated on
//! - `target_id` (TEXT) - ID of the entity (nullable)
//! - `source_device` (TEXT) - Source device identifier (nullable)
//! - `source_channel` (TEXT) - Channel/source of the operation (nullable)
//! - `source_agent` (TEXT) - Agent that generated the log (nullable)
//! - `success` (INTEGER) - 1 if successful, 0 if failed
//! - `error_message` (TEXT) - Error message if failed (nullable)
//! - `duration_ms` (INTEGER) - Operation duration in milliseconds (nullable)
//! - `metadata` (TEXT) - JSON-encoded additional metadata (nullable)

use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};

use super::log::{LogEntry, LogLevel, LogSource, LogType, TargetType};

/// Repository for accessing and manipulating log entries in SQLite.
/// Provides insert and query operations with support for filtering by
/// type, target, and time range.
pub struct LogRepository<'a> {
    /// SQLite connection, borrowed from the caller
    conn: &'a Connection,
}

impl<'a> LogRepository<'a> {
    /// Creates a new repository wrapping the given SQLite connection.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Inserts a single log entry into the database.
    ///
    /// # Arguments
    /// * `entry` - The log entry to insert
    ///
    /// # Errors
    /// Returns an error if the SQL execution fails.
    pub fn insert(&self, entry: &LogEntry) -> Result<()> {
        let timestamp = entry.timestamp.timestamp();
        // Convert u64 duration to i64 for SQLite (nullable)
        let duration_ms: Option<i64> = entry.duration_ms.map(|v| v as i64);

        self.conn
            .execute(
                r#"
            INSERT INTO logs (
                id, timestamp, level, log_type, operation, target_type, target_id,
                source_device, source_channel, source_agent,
                success, error_message, duration_ms, metadata
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
                params![
                    entry.id,
                    timestamp,
                    entry.level.to_string(),
                    entry.log_type.to_string(),
                    entry.operation,
                    entry.target_type.to_string(),
                    entry.target_id,
                    entry.source.device,
                    entry.source.channel,
                    entry.source.agent,
                    entry.success as i32,
                    entry.error_message,
                    duration_ms,
                    entry
                        .metadata
                        .as_ref()
                        .and_then(|m| serde_json::to_string(m).ok()),
                ],
            )
            .map_err(|e| Error::Config(format!("Failed to insert log: {}", e)))?;

        Ok(())
    }

    /// Queries logs filtered by log type, ordered by timestamp descending.
    ///
    /// # Arguments
    /// * `log_type` - The type of logs to query
    /// * `limit` - Maximum number of entries to return
    ///
    /// # Errors
    /// Returns an error if the SQL query preparation or execution fails.
    pub fn query_by_type(&self, log_type: LogType, limit: usize) -> Result<Vec<LogEntry>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
            SELECT id, timestamp, level, log_type, operation, target_type, target_id,
                   source_device, source_channel, source_agent,
                   success, error_message, duration_ms, metadata
            FROM logs
            WHERE log_type = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
            )
            .map_err(|e| Error::Config(format!("Failed to prepare query: {}", e)))?;

        let entries = stmt
            .query_map(params![log_type.to_string(), limit as i64], |row| {
                self.row_to_entry(row)
            })
            .map_err(|e| Error::Config(format!("Failed to query: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Queries logs for a specific target entity, ordered by timestamp descending.
    ///
    /// # Arguments
    /// * `target_type` - The type of entity to query logs for
    /// * `target_id` - The specific entity ID
    /// * `limit` - Maximum number of entries to return
    pub fn query_for_target(
        &self,
        target_type: TargetType,
        target_id: &str,
        limit: usize,
    ) -> Result<Vec<LogEntry>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
            SELECT id, timestamp, level, log_type, operation, target_type, target_id,
                   source_device, source_channel, source_agent,
                   success, error_message, duration_ms, metadata
            FROM logs
            WHERE target_type = ? AND target_id = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
            )
            .map_err(|e| Error::Config(format!("Failed to prepare query: {}", e)))?;

        let entries = stmt
            .query_map(
                params![target_type.to_string(), target_id, limit as i64],
                |row| self.row_to_entry(row),
            )
            .map_err(|e| Error::Config(format!("Failed to query: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Queries logs within a time range, ordered by timestamp descending.
    ///
    /// # Arguments
    /// * `start` - Start of the time range (inclusive)
    /// * `end` - End of the time range (inclusive)
    /// * `limit` - Maximum number of entries to return
    pub fn query_by_timerange(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<LogEntry>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
            SELECT id, timestamp, level, log_type, operation, target_type, target_id,
                   source_device, source_channel, source_agent,
                   success, error_message, duration_ms, metadata
            FROM logs
            WHERE timestamp >= ? AND timestamp <= ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
            )
            .map_err(|e| Error::Config(format!("Failed to prepare query: {}", e)))?;

        let entries = stmt
            .query_map(
                params![start.timestamp(), end.timestamp(), limit as i64],
                |row| self.row_to_entry(row),
            )
            .map_err(|e| Error::Config(format!("Failed to query: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Gets the most recent logs, ordered by timestamp descending.
    ///
    /// # Arguments
    /// * `limit` - Maximum number of entries to return
    pub fn query_recent(&self, limit: usize) -> Result<Vec<LogEntry>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
            SELECT id, timestamp, level, log_type, operation, target_type, target_id,
                   source_device, source_channel, source_agent,
                   success, error_message, duration_ms, metadata
            FROM logs
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
            )
            .map_err(|e| Error::Config(format!("Failed to prepare query: {}", e)))?;

        let entries = stmt
            .query_map(params![limit as i64], |row| self.row_to_entry(row))
            .map_err(|e| Error::Config(format!("Failed to query: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Gets aggregated AI processing statistics for the specified number of days.
    ///
    /// # Arguments
    /// * `days` - Number of days to look back for statistics
    ///
    /// Returns counts of total, successful, and failed operations, plus average duration.
    pub fn get_ai_stats(&self, days: u32) -> Result<AiProcessingStats> {
        let cutoff = Utc::now() - chrono::Duration::days(days as i64);

        let mut stmt = self
            .conn
            .prepare(
                r#"
            SELECT
                COUNT(*) as total,
                COALESCE(SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END), 0) as successful,
                COALESCE(SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END), 0) as failed,
                COALESCE(AVG(duration_ms), 0) as avg_duration
            FROM logs
            WHERE log_type = 'ai_processing' AND timestamp >= ?
            "#,
            )
            .map_err(|e| Error::Config(format!("Failed to prepare query: {}", e)))?;

        let stats = stmt
            .query_row(params![cutoff.timestamp()], |row| {
                Ok(AiStatsRow {
                    total: row.get::<_, i64>(0)?,
                    successful: row.get::<_, i64>(1)?,
                    failed: row.get::<_, i64>(2)?,
                    avg_duration: row.get::<_, f64>(3)?,
                })
            })
            .map_err(|e| Error::Config(format!("Failed to query: {}", e)))?;

        Ok(AiProcessingStats {
            total_operations: stats.total as u64,
            successful_operations: stats.successful as u64,
            failed_operations: stats.failed as u64,
            avg_duration_ms: stats.avg_duration,
            by_data_type: std::collections::HashMap::new(),
            by_model: std::collections::HashMap::new(),
        })
    }

    /// Converts a database row into a LogEntry struct.
    /// Handles parsing of string representations back to enum types
    /// and deserialization of JSON metadata.
    fn row_to_entry(&self, row: &rusqlite::Row) -> rusqlite::Result<LogEntry> {
        let log_type_str: String = row.get(3)?;
        let target_type_str: String = row.get(5)?;

        let metadata_str: Option<String> = row.get(13)?;
        let metadata: Option<serde_json::Value> =
            metadata_str.and_then(|s| serde_json::from_str(&s).ok());

        Ok(LogEntry {
            id: row.get(0)?,
            timestamp: DateTime::from_timestamp(row.get::<_, i64>(1)?, 0)
                .unwrap_or_else(chrono::Utc::now),
            level: parse_log_level(&row.get::<_, String>(2)?),
            log_type: parse_log_type(&log_type_str),
            operation: row.get(4)?,
            target_type: parse_target_type(&target_type_str),
            target_id: row.get(6)?,
            source: LogSource {
                device: row.get(7)?,
                channel: row.get(8)?,
                agent: row.get(9)?,
            },
            success: row.get::<_, i32>(10)? != 0,
            error_message: row.get(11)?,
            duration_ms: row.get::<_, Option<i64>>(12)?.map(|v| v as u64),
            metadata,
        })
    }
}

/// Temporary struct for collecting AI stats aggregation results from a single row.
struct AiStatsRow {
    total: i64,
    successful: i64,
    failed: i64,
    avg_duration: f64,
}

/// Aggregated statistics for AI processing operations.
#[derive(Debug, Default)]
pub struct AiProcessingStats {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub avg_duration_ms: f64,
    pub by_data_type: std::collections::HashMap<String, u64>,
    pub by_model: std::collections::HashMap<String, u64>,
}

/// Parses a string representation into a LogLevel enum.
/// Defaults to Info if the string doesn't match any known level.
fn parse_log_level(s: &str) -> LogLevel {
    match s {
        "trace" => LogLevel::Trace,
        "debug" => LogLevel::Debug,
        "info" => LogLevel::Info,
        "warn" => LogLevel::Warn,
        "error" => LogLevel::Error,
        _ => LogLevel::Info,
    }
}

/// Parses a string representation into a LogType enum.
/// Defaults to Custom if the string doesn't match any known type.
fn parse_log_type(s: &str) -> LogType {
    match s {
        "crud" => LogType::Crud,
        "ai_processing" => LogType::AiProcessing,
        "pipeline" => LogType::Pipeline,
        "system" => LogType::System,
        "tag" => LogType::Tag,
        "cognition" => LogType::Cognition,
        "evaluation" => LogType::Evaluation,
        _ => LogType::Custom,
    }
}

/// Parses a string representation into a TargetType enum.
/// Defaults to System if the string doesn't match any known type.
fn parse_target_type(s: &str) -> TargetType {
    match s {
        "event" => TargetType::Event,
        "entity" => TargetType::Entity,
        "tag" => TargetType::Tag,
        "pipeline_task" => TargetType::PipelineTask,
        "config" => TargetType::Config,
        "system" => TargetType::System,
        _ => TargetType::System,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::log::{CrudOperation, LogEntry};
    use rusqlite::Connection;

    /// Creates an in-memory SQLite connection with the logs table.
    fn create_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            r#"
            CREATE TABLE logs (
                id TEXT PRIMARY KEY,
                timestamp INTEGER NOT NULL,
                level TEXT NOT NULL,
                log_type TEXT NOT NULL,
                operation TEXT NOT NULL,
                target_type TEXT NOT NULL,
                target_id TEXT,
                source_device TEXT,
                source_channel TEXT,
                source_agent TEXT,
                success INTEGER NOT NULL,
                error_message TEXT,
                duration_ms INTEGER,
                metadata TEXT
            )
            "#,
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_insert_and_query_by_type() {
        let conn = create_test_db();
        let repo = LogRepository::new(&conn);

        let entry = LogEntry::new(LogType::Crud, "create", TargetType::Event)
            .with_target_id("evt-001")
            .with_duration_ms(100);

        repo.insert(&entry).unwrap();

        let results = repo.query_by_type(LogType::Crud, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, entry.id);
        assert_eq!(results[0].operation, "create");
    }

    #[test]
    fn test_insert_and_query_for_target() {
        let conn = create_test_db();
        let repo = LogRepository::new(&conn);

        let entry1 =
            LogEntry::new(LogType::Crud, "create", TargetType::Event).with_target_id("evt-001");
        let entry2 =
            LogEntry::new(LogType::Crud, "read", TargetType::Event).with_target_id("evt-002");
        let entry3 =
            LogEntry::new(LogType::Crud, "create", TargetType::Entity).with_target_id("ent-001");

        repo.insert(&entry1).unwrap();
        repo.insert(&entry2).unwrap();
        repo.insert(&entry3).unwrap();

        let results = repo
            .query_for_target(TargetType::Event, "evt-001", 10)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].target_id.as_deref(), Some("evt-001"));
    }

    #[test]
    fn test_insert_and_query_by_timerange() {
        let conn = create_test_db();
        let repo = LogRepository::new(&conn);

        let entry = LogEntry::new(LogType::System, "startup", TargetType::System);
        repo.insert(&entry).unwrap();

        let now = chrono::Utc::now();
        let results = repo
            .query_by_timerange(
                now - chrono::Duration::hours(1),
                now + chrono::Duration::hours(1),
                10,
            )
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_query_recent() {
        let conn = create_test_db();
        let repo = LogRepository::new(&conn);

        for i in 0..5 {
            let entry = LogEntry::new(
                LogType::Crud,
                CrudOperation::Create.to_string(),
                TargetType::Event,
            )
            .with_target_id(format!("evt-{}", i));
            repo.insert(&entry).unwrap();
        }

        let results = repo.query_recent(3).unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_insert_with_metadata() {
        let conn = create_test_db();
        let repo = LogRepository::new(&conn);

        let metadata = serde_json::json!({
            "file_size_bytes": 1024,
            "model_name": "gpt-4"
        });

        let entry = LogEntry::new(LogType::AiProcessing, "analyze", TargetType::PipelineTask)
            .with_metadata(metadata);

        repo.insert(&entry).unwrap();

        let results = repo.query_by_type(LogType::AiProcessing, 1).unwrap();
        assert!(results[0].metadata.is_some());
    }

    #[test]
    fn test_insert_with_error() {
        let conn = create_test_db();
        let repo = LogRepository::new(&conn);

        let entry = LogEntry::new(LogType::AiProcessing, "analyze", TargetType::PipelineTask)
            .with_error("Connection refused");

        repo.insert(&entry).unwrap();

        let results = repo.query_by_type(LogType::AiProcessing, 1).unwrap();
        assert!(!results[0].success);
        assert_eq!(
            results[0].error_message.as_deref(),
            Some("Connection refused")
        );
    }

    #[test]
    fn test_ai_processing_stats() {
        let conn = create_test_db();
        let repo = LogRepository::new(&conn);

        // Insert successful AI processing entry
        let entry1 = LogEntry::new(LogType::AiProcessing, "analyze", TargetType::PipelineTask)
            .with_duration_ms(100);
        repo.insert(&entry1).unwrap();

        // Insert failed AI processing entry
        let entry2 = LogEntry::new(LogType::AiProcessing, "analyze", TargetType::PipelineTask)
            .with_error("Timeout")
            .with_duration_ms(200);
        repo.insert(&entry2).unwrap();

        let stats = repo.get_ai_stats(7).unwrap();
        assert_eq!(stats.total_operations, 2);
        assert_eq!(stats.successful_operations, 1);
        assert_eq!(stats.failed_operations, 1);
    }

    #[test]
    fn test_parse_log_level() {
        assert_eq!(parse_log_level("trace"), LogLevel::Trace);
        assert_eq!(parse_log_level("debug"), LogLevel::Debug);
        assert_eq!(parse_log_level("info"), LogLevel::Info);
        assert_eq!(parse_log_level("warn"), LogLevel::Warn);
        assert_eq!(parse_log_level("error"), LogLevel::Error);
        // Default case
        assert_eq!(parse_log_level("unknown"), LogLevel::Info);
        assert_eq!(parse_log_level(""), LogLevel::Info);
    }

    #[test]
    fn test_parse_log_type() {
        assert_eq!(parse_log_type("crud"), LogType::Crud);
        assert_eq!(parse_log_type("ai_processing"), LogType::AiProcessing);
        assert_eq!(parse_log_type("pipeline"), LogType::Pipeline);
        assert_eq!(parse_log_type("system"), LogType::System);
        assert_eq!(parse_log_type("tag"), LogType::Tag);
        assert_eq!(parse_log_type("cognition"), LogType::Cognition);
        assert_eq!(parse_log_type("evaluation"), LogType::Evaluation);
        // Default case
        assert_eq!(parse_log_type("unknown"), LogType::Custom);
        assert_eq!(parse_log_type(""), LogType::Custom);
    }

    #[test]
    fn test_parse_target_type() {
        assert_eq!(parse_target_type("event"), TargetType::Event);
        assert_eq!(parse_target_type("entity"), TargetType::Entity);
        assert_eq!(parse_target_type("tag"), TargetType::Tag);
        assert_eq!(parse_target_type("pipeline_task"), TargetType::PipelineTask);
        assert_eq!(parse_target_type("config"), TargetType::Config);
        assert_eq!(parse_target_type("system"), TargetType::System);
        // Default case
        assert_eq!(parse_target_type("unknown"), TargetType::System);
        assert_eq!(parse_target_type(""), TargetType::System);
    }
}
