//! Configuration management

use crate::error::Result;
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Brain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BrainConfig {
    /// Path to the SQLite database
    pub db_path: PathBuf,
    /// Path to events directory
    pub events_path: PathBuf,
    /// Path to entities directory
    pub entities_path: PathBuf,
    /// Path to raw data directory
    pub raw_data_path: PathBuf,
    /// Path to pipeline queue directory
    pub pipeline_queue_path: PathBuf,
    /// Path to dictionaries directory
    pub dicts_path: PathBuf,
    /// Path to logs database (can be different from main db)
    pub log_db_path: PathBuf,
    /// Log rotation strategy: "weekly" or "monthly"
    pub log_rotation: String,
    /// Adapter configurations
    pub adapters: Vec<crate::adapters::AdapterConfig>,
}

impl Default for BrainConfig {
    fn default() -> Self {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        Self {
            db_path: root.join("index/events.db"),
            events_path: root.join("events"),
            entities_path: root.join("entities"),
            raw_data_path: root.join("data/raw"),
            pipeline_queue_path: root.join("pipelines/queue"),
            dicts_path: root.join("dicts"),
            log_db_path: root.join("logs/brain"),
            log_rotation: "monthly".to_string(),
            adapters: Vec::new(),
        }
    }
}

impl BrainConfig {
    /// Load configuration from file
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: BrainConfig = serde_yaml::from_str(&content)?;
            Ok(config)
        } else {
            let config = Self::default();
            config.save_template()?;
            eprintln!(
                "Warning: config file not found at {:?}. Created template with defaults.",
                config_path
            );
            eprintln!("Please configure your AI adapter in config/brain.yaml");
            Ok(config)
        }
    }

    /// Save configuration template with adapters section as comments
    pub fn save_template(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let template = format!(
            r#"# Brain Configuration
# This file is auto-generated. Edit values as needed.

# Log rotation: "weekly" or "monthly"
log_rotation: "{}"

# AI Adapters Configuration
# Uncomment and fill in the adapter you want to use:
#
# adapters:
#   - adapter_type: ollama
#     endpoint: http://localhost:11434
#     api_key: ""  # No API key for local Ollama
#     default_model: llama3.2
#     thinking: false
#     timeout_secs: 300
#
#   - adapter_type: openai
#     endpoint: https://api.openai.com/v1
#     api_key: "your-api-key-here"
#     default_model: gpt-4
#     thinking: false
#     timeout_secs: 60
#
#   - adapter_type: minimax
#     endpoint: https://api.minimaxi.com/v1
#     api_key: "your-api-key-here"
#     default_model: MiniMax-Text-01
#     thinking: false
#     timeout_secs: 60
"#,
            self.log_rotation
        );

        fs::write(&config_path, template)?;
        Ok(())
    }

    /// Get the default config path
    fn config_path() -> Result<PathBuf> {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Ok(root.join("config").join("brain.yaml"))
    }

    /// Get the log database path for the current time period
    pub fn log_db_path_for_time(&self) -> PathBuf {
        let now = chrono::Utc::now();
        let year = now.format("%Y").to_string();
        let month = now.format("%m").to_string();
        let week = ((now.day() - 1) / 7 + 1).to_string();

        let dir = if self.log_rotation == "weekly" {
            self.log_db_path.join(&year).join(format!("week{}", week))
        } else {
            self.log_db_path.join(&year).join(&month)
        };

        fs::create_dir_all(&dir).ok();
        dir.join("logs.db")
    }

    /// Iterate over all log database paths (for querying historical logs)
    /// Returns an iterator to avoid loading all paths into memory at once.
    pub fn iter_log_db_paths(&self) -> LogDbPathIter {
        LogDbPathIter {
            base_path: self.log_db_path.clone(),
            year_iter: None,
            month_iter: None,
            db_iter: None,
        }
    }
}

/// Iterator over log database paths
pub struct LogDbPathIter {
    base_path: PathBuf,
    year_iter: Option<fs::ReadDir>,
    month_iter: Option<fs::ReadDir>,
    db_iter: Option<fs::ReadDir>,
}

impl Iterator for LogDbPathIter {
    type Item = PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Try to get from current db_iter first
            if let Some(ref mut db_iter) = self.db_iter {
                if let Some(Ok(entry)) = db_iter.next() {
                    let path = entry.path();
                    if path.is_file() && path.extension().map(|e| e == "db").unwrap_or(false) {
                        return Some(path);
                    }
                }
            }

            // db_iter exhausted, need to advance month_iter
            self.db_iter = None;

            if let Some(ref mut month_iter) = self.month_iter {
                if let Some(Ok(entry)) = month_iter.next() {
                    let path = entry.path();
                    if path.is_dir() {
                        // Try to open db dir (for weekly) or use this dir directly
                        if let Ok(sub_entries) = fs::read_dir(&path) {
                            // Check if this dir directly contains a .db file
                            for sub_entry in sub_entries.filter_map(|e| e.ok()) {
                                let sub_path = sub_entry.path();
                                if sub_path.is_file()
                                    && sub_path.extension().map(|e| e == "db").unwrap_or(false)
                                {
                                    self.db_iter = Some(fs::read_dir(&path).ok().unwrap());
                                    break;
                                }
                            }
                        }
                        // Try this dir for weekly format
                        if self.db_iter.is_none() {
                            self.db_iter = fs::read_dir(&path).ok();
                        }
                        if self.db_iter.is_some() {
                            continue;
                        }
                    }
                }
            }

            // month_iter exhausted, need to advance year_iter
            self.month_iter = None;

            if let Some(ref mut year_iter) = self.year_iter {
                if let Some(Ok(entry)) = year_iter.next() {
                    let path = entry.path();
                    if path.is_dir() {
                        self.month_iter = fs::read_dir(&path).ok();
                        if self.month_iter.is_some() {
                            continue;
                        }
                    }
                }
            }

            // year_iter exhausted, try to open base_path
            if self.year_iter.is_none() {
                self.year_iter = fs::read_dir(&self.base_path).ok();
                if self.year_iter.is_some() {
                    continue;
                }
            }

            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = BrainConfig::default();
        assert!(config.db_path.ends_with("events.db"));
    }
}
