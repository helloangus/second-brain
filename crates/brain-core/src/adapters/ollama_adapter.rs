//! Ollama adapter implementation

use crate::adapters::{AnalysisOutput, ModelAdapter, RawDataInput, RawDataType};
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

/// Ollama adapter for local LLM inference
pub struct OllamaAdapter {
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
        Ok(Self {
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
        let body_str =
            serde_json::to_string(body).map_err(|e| Error::Config(format!("JSON error: {}", e)))?;

        let response = ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_string(&body_str)
            .map_err(|e| Error::Http(format!("Request failed: {}", e)))?;

        if response.status() >= 400 {
            return Err(Error::Http(format!("HTTP error: {}", response.status())));
        }

        let text = response.into_string().map_err(Error::Io)?;
        let result = serde_json::from_str(&text)
            .map_err(|e| Error::Config(format!("Parse error: {}", e)))?;
        Ok(result)
    }
}

impl ModelAdapter for OllamaAdapter {
    fn name(&self) -> &str {
        "ollama"
    }

    fn supported_data_types(&self) -> Vec<RawDataType> {
        vec![RawDataType::Text, RawDataType::Image, RawDataType::Document]
    }

    fn analyze(&self, input: &RawDataInput) -> Result<AnalysisOutput> {
        // Read file content if it's text
        let content = std::fs::read_to_string(&input.path).map_err(Error::Io)?;

        // Build dict context for the prompt
        let dict_context_str = if let Some(ref ctx) = input.dict_context {
            let types = if ctx.event_types.is_empty() {
                "none".to_string()
            } else {
                ctx.event_types.join(", ")
            };
            let subtypes = if ctx.event_subtypes.is_empty() {
                "none".to_string()
            } else {
                ctx.event_subtypes.join(", ")
            };
            let tags = if ctx.tags.is_empty() {
                "none".to_string()
            } else {
                ctx.tags.join(", ")
            };
            let topics = if ctx.topics.is_empty() {
                "none".to_string()
            } else {
                ctx.topics.join(", ")
            };
            format!(
                "\n\nWhen choosing event type, subtype, tags, and topics, prefer from these existing values:\n- Event types (use one): {}\n- Event subtypes (use one): {}\n- Tags (use existing when relevant): {}\n- Topics (use existing when relevant): {}",
                types, subtypes, tags, topics
            )
        } else {
            String::new()
        };

        // Create a prompt for analysis
        let prompt = format!(
            r#"Analyze this {} and provide:
1. A brief summary (2-3 sentences)
2. Extended content - use this field whenever the content has complexity, multiple points, or details that don't fit in a 2-3 sentence summary. This field has no length limit.
3. Event type (prefer from existing: see below)
4. Event subtype (prefer from existing: see below)
5. Key tags (prefer from existing when relevant)
6. Key topics (prefer from existing when relevant)
7. Any entities mentioned

Content:
{}{}

Respond in JSON format:
{{
    "summary": "2-3 sentence brief summary",
    "extended": "detailed content - use whenever summary cannot capture all important information, OR null if content is simple enough",
    "type": "prefer from existing event types",
    "subtype": "prefer from existing subtypes",
    "tags": ["prefer existing tags"],
    "topics": ["prefer existing topics"],
    "entities": ["entity1"],
    "confidence": 0.0-1.0 (how certain you are about the analysis - lower if content is ambiguous or lacks clear signals)
}}"#,
            input.data_type,
            content.chars().take(2000).collect::<String>(),
            dict_context_str
        );

        let request = OllamaRequest {
            model: self.model.clone(),
            prompt,
            stream: false,
        };

        let response: OllamaResponse = self.post("api/generate", &request)?;

        // Try to parse as JSON
        let output: AnalysisOutput =
            serde_json::from_str(&response.response).unwrap_or_else(|_| AnalysisOutput {
                summary: Some(response.response.clone()),
                extended: None,
                type_: None,
                subtype: None,
                tags: Vec::new(),
                topics: Vec::new(),
                entities: Vec::new(),
                confidence: None,
                raw_response: serde_json::Value::String(response.response),
            });

        Ok(output)
    }

    fn summarize(&self, text: &str) -> Result<String> {
        let prompt = format!("Summarize the following text in 2-3 sentences:\n\n{}", text);

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
        let url = format!("{}/api/tags", self.endpoint);
        match ureq::get(&url).call() {
            Ok(response) => Ok(response.status() < 400),
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
