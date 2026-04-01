//! Log repository for database operations

use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};

use super::log::{LogEntry, LogLevel, LogSource, LogType, TargetType};

pub struct LogRepository<'a> {
    conn: &'a Connection,
}

impl<'a> LogRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Insert a log entry into the database
    pub fn insert(&self, entry: &LogEntry) -> Result<()> {
        let timestamp = entry.timestamp.timestamp();
        let duration_ms: Option<i64> = entry.duration_ms.map(|v| v as i64);

        self.conn.execute(
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
                entry.metadata.as_ref().and_then(|m| serde_json::to_string(m).ok()),
            ],
        ).map_err(|e| Error::Config(format!("Failed to insert log: {}", e)))?;

        Ok(())
    }

    /// Query logs by log type
    pub fn query_by_type(&self, log_type: LogType, limit: usize) -> Result<Vec<LogEntry>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, timestamp, level, log_type, operation, target_type, target_id,
                   source_device, source_channel, source_agent,
                   success, error_message, duration_ms, metadata
            FROM logs
            WHERE log_type = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        ).map_err(|e| Error::Config(format!("Failed to prepare query: {}", e)))?;

        let entries = stmt
            .query_map(params![log_type.to_string(), limit as i64], |row| {
                self.row_to_entry(row)
            })
            .map_err(|e| Error::Config(format!("Failed to query: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Query logs for a specific target
    pub fn query_for_target(
        &self,
        target_type: TargetType,
        target_id: &str,
        limit: usize,
    ) -> Result<Vec<LogEntry>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, timestamp, level, log_type, operation, target_type, target_id,
                   source_device, source_channel, source_agent,
                   success, error_message, duration_ms, metadata
            FROM logs
            WHERE target_type = ? AND target_id = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        ).map_err(|e| Error::Config(format!("Failed to prepare query: {}", e)))?;

        let entries = stmt
            .query_map(params![target_type.to_string(), target_id, limit as i64], |row| {
                self.row_to_entry(row)
            })
            .map_err(|e| Error::Config(format!("Failed to query: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Query logs within a time range
    pub fn query_by_timerange(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<LogEntry>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, timestamp, level, log_type, operation, target_type, target_id,
                   source_device, source_channel, source_agent,
                   success, error_message, duration_ms, metadata
            FROM logs
            WHERE timestamp >= ? AND timestamp <= ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        ).map_err(|e| Error::Config(format!("Failed to prepare query: {}", e)))?;

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

    /// Get recent logs
    pub fn query_recent(&self, limit: usize) -> Result<Vec<LogEntry>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, timestamp, level, log_type, operation, target_type, target_id,
                   source_device, source_channel, source_agent,
                   success, error_message, duration_ms, metadata
            FROM logs
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        ).map_err(|e| Error::Config(format!("Failed to prepare query: {}", e)))?;

        let entries = stmt
            .query_map(params![limit as i64], |row| self.row_to_entry(row))
            .map_err(|e| Error::Config(format!("Failed to query: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Get AI processing statistics
    pub fn get_ai_stats(&self, days: u32) -> Result<AiProcessingStats> {
        let cutoff = Utc::now() - chrono::Duration::days(days as i64);

        let mut stmt = self.conn.prepare(
            r#"
            SELECT
                COUNT(*) as total,
                COALESCE(SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END), 0) as successful,
                COALESCE(SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END), 0) as failed,
                COALESCE(AVG(duration_ms), 0) as avg_duration
            FROM logs
            WHERE log_type = 'ai_processing' AND timestamp >= ?
            "#,
        ).map_err(|e| Error::Config(format!("Failed to prepare query: {}", e)))?;

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
            duration_ms: row
                .get::<_, Option<i64>>(12)?
                .map(|v| v as u64),
            metadata,
        })
    }
}

struct AiStatsRow {
    total: i64,
    successful: i64,
    failed: i64,
    avg_duration: f64,
}

pub struct AiProcessingStats {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub avg_duration_ms: f64,
    pub by_data_type: std::collections::HashMap<String, u64>,
    pub by_model: std::collections::HashMap<String, u64>,
}

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
