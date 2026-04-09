//! Model adapter trait

use crate::dicts::{DictEntry, DictSet};
use crate::error::Result;
use crate::models::{RawDataType, TaskType};
use serde::{Deserialize, Serialize};

/// Input for model analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawDataInput {
    pub data_type: RawDataType,
    pub path: String,
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
    /// Dictionary set for AI Step 2 alignment
    #[serde(default)]
    pub dict_set: Option<DictSet>,
}

/// New dictionary entries discovered during analysis
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NewDictEntries {
    #[serde(default)]
    pub event_types: Vec<DictEntry>,
    #[serde(default)]
    pub event_subtypes: Vec<DictEntry>,
    #[serde(default)]
    pub tags: Vec<DictEntry>,
    #[serde(default)]
    pub topics: Vec<DictEntry>,
}

/// Analysis output with newly discovered dictionary entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisOutputWithNewEntries {
    pub analysis: AnalysisOutput,
    pub new_entries: NewDictEntries,
}

/// Output from model analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisOutput {
    pub summary: Option<String>,
    /// Extended content - longer text that doesn't fit in summary
    #[serde(default)]
    pub extended: Option<String>,
    /// Event type suggested by AI
    #[serde(default)]
    pub type_: Option<String>,
    /// Event subtype suggested by AI
    #[serde(default)]
    pub subtype: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub entities: Vec<String>,
    pub confidence: Option<f64>,
    #[serde(default)]
    pub raw_response: serde_json::Value,
}

/// Model adapter trait - base trait for all AI models
pub trait ModelAdapter: Send + Sync {
    /// Get the name of this adapter
    fn name(&self) -> &str;

    /// Get supported raw data types for this adapter
    fn supported_data_types(&self) -> Vec<RawDataType>;

    /// Get supported task types for this adapter
    fn supported_task_types(&self) -> Vec<TaskType>;

    /// Health check - verify the model is reachable
    fn health_check(&self) -> Result<bool>;

    /// Analyze input data - dispatches to summarize or reason based on task type
    fn analyze(&self, task: TaskType, input: &RawDataInput)
        -> Result<AnalysisOutputWithNewEntries>;
}

/// Summarize adapter trait - for models that support Summarize task type
pub trait SummarizeAdapter: ModelAdapter {
    /// Summarize raw data with two-step process (free analysis + dictionary alignment)
    /// Returns analysis output along with any new dictionary entries discovered
    fn summarize(&self, input: &RawDataInput) -> Result<AnalysisOutputWithNewEntries>;
}

/// Reasoning adapter trait - for models that support Reasoning task type
pub trait ReasoningAdapter: ModelAdapter {
    /// Perform reasoning on raw data with two-step process (free analysis + dictionary alignment)
    /// Returns analysis output along with any new dictionary entries discovered
    fn reason(&self, input: &RawDataInput) -> Result<AnalysisOutputWithNewEntries>;
}

/// Configuration for creating adapters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    pub adapter_type: String,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub default_model: String,
    #[serde(default)]
    pub timeout_secs: u64,
    /// Enable thinking mode for models that support it (MiniMax)
    #[serde(default)]
    pub thinking: bool,
}

use std::sync::Arc;

/// Factory for creating model adapters
pub fn create_adapter(config: &AdapterConfig) -> Result<Arc<dyn ModelAdapter>> {
    match config.adapter_type.as_str() {
        "ollama" => {
            let endpoint = config
                .endpoint
                .clone()
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            Ok(
                Arc::new(super::OllamaAdapter::new(&endpoint, &config.default_model)?)
                    as Arc<dyn ModelAdapter>,
            )
        }
        "openai" => {
            let api_key = config
                .api_key
                .clone()
                .ok_or_else(|| crate::Error::Config("OpenAI API key required".to_string()))?;
            Ok(
                Arc::new(super::OpenAIAdapter::new(&api_key, &config.default_model)?)
                    as Arc<dyn ModelAdapter>,
            )
        }
        "minimax" => {
            let api_key = config
                .api_key
                .clone()
                .ok_or_else(|| crate::Error::Config("MiniMax API key required".to_string()))?;
            let endpoint = config
                .endpoint
                .clone()
                .unwrap_or_else(|| "https://api.minimaxi.com/v1".to_string());
            Ok(Arc::new(super::MiniMaxAdapter::new(
                &api_key,
                &config.default_model,
                &endpoint,
                config.thinking,
            )?) as Arc<dyn ModelAdapter>)
        }
        _ => Err(crate::Error::Config(format!(
            "Unknown adapter type: {}",
            config.adapter_type
        ))),
    }
}
