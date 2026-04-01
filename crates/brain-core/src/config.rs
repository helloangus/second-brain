//! Configuration management

use crate::error::{Error, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Brain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Adapter configurations
    #[serde(default)]
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
            Ok(Self::default())
        }
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_yaml::to_string(self)?;
        fs::write(&config_path, content)?;
        Ok(())
    }

    /// Get the default config path
    fn config_path() -> Result<PathBuf> {
        let base_dir = ProjectDirs::from("com", "secondbrain", "brain")
            .map(|dirs| dirs.config_dir().to_path_buf())
            .ok_or_else(|| Error::Config("Could not determine config directory".to_string()))?;

        Ok(base_dir.join("brain.yaml"))
    }

    /// Get the schema path
    pub fn schema_path(&self) -> PathBuf {
        self.events_path
            .parent()
            .map(|p| p.join("config/schema.yaml"))
            .unwrap_or_else(|| PathBuf::from("config/schema.yaml"))
    }
}

/// Schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub version: i32,
    pub event: EventSchema,
    pub entity_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSchema {
    pub required_fields: Vec<String>,
    #[serde(default)]
    pub optional_fields: Vec<String>,
}

impl Default for Schema {
    fn default() -> Self {
        Self {
            version: 1,
            event: EventSchema {
                required_fields: vec!["id".to_string(), "time".to_string(), "type".to_string()],
                optional_fields: vec![
                    "source".to_string(),
                    "entities".to_string(),
                    "raw_refs".to_string(),
                    "ai".to_string(),
                    "status".to_string(),
                ],
            },
            entity_types: vec![
                "person".to_string(),
                "organization".to_string(),
                "project".to_string(),
                "artifact".to_string(),
                "concept".to_string(),
                "topic".to_string(),
                "activity".to_string(),
                "goal".to_string(),
                "skill".to_string(),
                "place".to_string(),
                "device".to_string(),
                "resource".to_string(),
                "memory_cluster".to_string(),
                "state".to_string(),
            ],
        }
    }
}

impl Schema {
    /// Load schema from file
    pub fn load(path: &PathBuf) -> Result<Self> {
        if path.exists() {
            let content = fs::read_to_string(path)?;
            let schema: Schema = serde_yaml::from_str(&content)?;
            Ok(schema)
        } else {
            Ok(Self::default())
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

    #[test]
    fn test_default_schema() {
        let schema = Schema::default();
        assert_eq!(schema.version, 1);
        assert_eq!(schema.entity_types.len(), 14);
    }
}
