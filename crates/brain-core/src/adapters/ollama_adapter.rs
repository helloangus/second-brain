//! Ollama adapter implementation

use crate::adapters::{AnalysisOutput, ModelAdapter, RawDataInput, RawDataType};
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Ollama adapter for local LLM inference
pub struct OllamaAdapter {
    client: reqwest::blocking::Client,
    endpoint: String,
    model: String,
}

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaEmbedRequest {
    model: String,
    prompt: String,
}

#[derive(Debug, Deserialize)]
struct OllamaEmbedResponse {
    embedding: Vec<f32>,
}

impl OllamaAdapter {
    /// Create a new Ollama adapter
    pub fn new(endpoint: &str, model: &str) -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| Error::Config(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            endpoint: endpoint.trim_end_matches('/').to_string(),
            model: model.to_string(),
        })
    }

    /// Check if Ollama server is available
    pub fn is_available(&self) -> bool {
        self.health_check().unwrap_or(false)
    }

    fn post<T: for<'de> Deserialize<'de>>(&self, path: &str, body: &impl Serialize) -> Result<T> {
        let url = format!("{}/{}", self.endpoint, path);
        let response = self.client
            .post(&url)
            .json(body)
            .send()?
            .json::<T>()?;
        Ok(response)
    }
}

impl ModelAdapter for OllamaAdapter {
    fn name(&self) -> &str {
        "ollama"
    }

    fn supported_data_types(&self) -> Vec<RawDataType> {
        vec![
            RawDataType::Text,
            RawDataType::Image,
            RawDataType::Document,
        ]
    }

    fn analyze(&self, input: &RawDataInput) -> Result<AnalysisOutput> {
        // Read file content if it's text
        let content = std::fs::read_to_string(&input.path)
            .map_err(|e| Error::Io(e))?;

        // Create a prompt for analysis
        let prompt = format!(
            r#"Analyze this {} and provide:
1. A brief summary (2-3 sentences)
2. Key tags (comma-separated)
3. Any entities or topics mentioned

Content:
{}

Respond in JSON format:
{{
    "summary": "...",
    "tags": ["tag1", "tag2"],
    "entities": ["entity1"],
    "confidence": 0.85
}}"#,
            input.data_type,
            content.chars().take(2000).collect::<String>()
        );

        let request = OllamaRequest {
            model: self.model.clone(),
            prompt,
            stream: false,
        };

        let response: OllamaResponse = self.post("api/generate", &request)?;

        // Try to parse as JSON
        let output: AnalysisOutput = serde_json::from_str(&response.response)
            .unwrap_or_else(|_| AnalysisOutput {
                summary: Some(response.response.clone()),
                tags: Vec::new(),
                entities: Vec::new(),
                confidence: None,
                raw_response: serde_json::Value::String(response.response),
            });

        Ok(output)
    }

    fn summarize(&self, text: &str) -> Result<String> {
        let prompt = format!(
            "Summarize the following text in 2-3 sentences:\n\n{}",
            text
        );

        let request = OllamaRequest {
            model: self.model.clone(),
            prompt,
            stream: false,
        };

        let response: OllamaResponse = self.post("api/generate", &request)?;
        Ok(response.response)
    }

    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let request = OllamaEmbedRequest {
            model: self.model.clone(),
            prompt: text.to_string(),
        };

        let response: OllamaEmbedResponse = self.post("api/embeddings", &request)?;
        Ok(response.embedding)
    }

    fn health_check(&self) -> Result<bool> {
        match self.client.get(format!("{}/api/tags", self.endpoint)).send() {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_creation() {
        let adapter = OllamaAdapter::new("http://localhost:11434", "llama3").unwrap();
        assert_eq!(adapter.name(), "ollama");
    }
}
