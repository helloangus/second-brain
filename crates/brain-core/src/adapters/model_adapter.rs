//! Model adapter trait

use crate::dicts::{DictEntry, DictSet};
use crate::error::Result;
use crate::models::RawDataType;
use serde::{Deserialize, Serialize};

/// Input for model analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawDataInput {
    pub data_type: RawDataType,
    pub path: String,
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
    /// Dictionary context for AI to prefer existing values
    #[serde(default)]
    pub dict_context: Option<DictContext>,
}

/// Dictionary context for AI analysis
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DictContext {
    /// Event type keys for backward compatibility
    #[serde(default)]
    pub event_types: Vec<String>,
    /// Event subtype keys for backward compatibility
    #[serde(default)]
    pub event_subtypes: Vec<String>,
    /// Tag keys for backward compatibility
    #[serde(default)]
    pub tags: Vec<String>,
    /// Topic keys for backward compatibility
    #[serde(default)]
    pub topics: Vec<String>,
    /// Full dictionary set for Step 2 alignment (not serialized, set at runtime)
    #[serde(skip)]
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

/// Model adapter trait - all AI models must implement this
pub trait ModelAdapter: Send + Sync {
    /// Get the name of this adapter
    fn name(&self) -> &str;

    /// Get supported raw data types for this adapter
    fn supported_data_types(&self) -> Vec<RawDataType>;

    /// Check if this adapter supports the given data type
    fn supports(&self, data_type: &RawDataType) -> bool {
        self.supported_data_types().contains(data_type)
    }

    /// Analyze raw data with two-step process (free analysis + dictionary alignment)
    /// Returns analysis output along with any new dictionary entries discovered
    fn analyze(&self, input: &RawDataInput) -> Result<AnalysisOutputWithNewEntries>;

    /// Generate a summary of text content
    fn summarize(&self, text: &str) -> Result<String>;

    /// Generate embeddings for text
    fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Health check - verify the model is reachable
    fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
}

/// Configuration for creating adapters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    pub adapter_type: String,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_model")]
    pub default_model: String,
    #[serde(default)]
    pub timeout_secs: u64,
    /// Enable thinking mode for models that support it (MiniMax)
    #[serde(default)]
    pub thinking: bool,
}

fn default_model() -> String {
    "llama3".to_string()
}

impl AdapterConfig {
    /// Create an Ollama adapter config
    pub fn ollama(endpoint: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            adapter_type: "ollama".to_string(),
            endpoint: Some(endpoint.into()),
            api_key: None,
            default_model: model.into(),
            timeout_secs: 60,
            thinking: false,
        }
    }

    /// Create an OpenAI adapter config
    pub fn openai(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            adapter_type: "openai".to_string(),
            endpoint: None,
            api_key: Some(api_key.into()),
            default_model: model.into(),
            timeout_secs: 30,
            thinking: false,
        }
    }

    /// Create a MiniMax adapter config
    pub fn minimax(
        api_key: impl Into<String>,
        model: impl Into<String>,
        endpoint: impl Into<String>,
        thinking: bool,
    ) -> Self {
        Self {
            adapter_type: "minimax".to_string(),
            endpoint: Some(endpoint.into()),
            api_key: Some(api_key.into()),
            default_model: model.into(),
            timeout_secs: 60,
            thinking,
        }
    }
}

/// Factory for creating model adapters
pub fn create_adapter(config: &AdapterConfig) -> Result<Box<dyn ModelAdapter>> {
    match config.adapter_type.as_str() {
        "ollama" => {
            let endpoint = config
                .endpoint
                .clone()
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            Ok(
                Box::new(super::OllamaAdapter::new(&endpoint, &config.default_model)?)
                    as Box<dyn ModelAdapter>,
            )
        }
        "openai" => {
            let api_key = config
                .api_key
                .clone()
                .ok_or_else(|| crate::Error::Config("OpenAI API key required".to_string()))?;
            Ok(
                Box::new(super::OpenAIAdapter::new(&api_key, &config.default_model)?)
                    as Box<dyn ModelAdapter>,
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
            Ok(Box::new(super::MiniMaxAdapter::new(
                &api_key,
                &config.default_model,
                &endpoint,
                config.thinking,
            )?) as Box<dyn ModelAdapter>)
        }
        _ => Err(crate::Error::Config(format!(
            "Unknown adapter type: {}",
            config.adapter_type
        ))),
    }
}
