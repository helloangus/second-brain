//! OpenAI adapter implementation

use crate::adapters::{AnalysisOutput, ModelAdapter, RawDataInput, RawDataType};
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// OpenAI adapter for cloud LLM inference
pub struct OpenAIAdapter {
    client: reqwest::blocking::Client,
    api_key: String,
    model: String,
}

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: MessageContent,
}

#[derive(Debug, Deserialize)]
struct MessageContent {
    content: String,
}

impl OpenAIAdapter {
    /// Create a new OpenAI adapter
    pub fn new(api_key: &str, model: &str) -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| Error::Config(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            api_key: api_key.to_string(),
            model: model.to_string(),
        })
    }

    fn post<T: for<'de> Deserialize<'de>>(&self, path: &str, body: &impl Serialize) -> Result<T> {
        let url = format!("https://api.openai.com/v1/{}", path);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(body)
            .send()?
            .json::<T>()?;
        Ok(response)
    }
}

impl ModelAdapter for OpenAIAdapter {
    fn name(&self) -> &str {
        "openai"
    }

    fn supported_data_types(&self) -> Vec<RawDataType> {
        vec![RawDataType::Text, RawDataType::Image, RawDataType::Document]
    }

    fn analyze(&self, input: &RawDataInput) -> Result<AnalysisOutput> {
        let content = std::fs::read_to_string(&input.path).map_err(Error::Io)?;

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

        let request = OpenAIRequest {
            model: self.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt,
            }],
            temperature: 0.7,
        };

        let response: OpenAIResponse = self.post("chat/completions", &request)?;

        let content_str = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        let output: AnalysisOutput = serde_json::from_str(&content_str).unwrap_or(AnalysisOutput {
            summary: Some(content_str),
            tags: Vec::new(),
            entities: Vec::new(),
            confidence: None,
            raw_response: serde_json::Value::Null,
        });

        Ok(output)
    }

    fn summarize(&self, text: &str) -> Result<String> {
        let request = OpenAIRequest {
            model: self.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: format!("Summarize the following text in 2-3 sentences:\n\n{}", text),
            }],
            temperature: 0.7,
        };

        let response: OpenAIResponse = self.post("chat/completions", &request)?;

        Ok(response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default())
    }

    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        #[derive(Serialize)]
        struct EmbedRequest {
            model: String,
            input: String,
        }

        #[derive(Deserialize)]
        struct EmbedResponse {
            data: Vec<EmbedData>,
        }

        #[derive(Deserialize)]
        struct EmbedData {
            embedding: Vec<f32>,
        }

        let request = EmbedRequest {
            model: "text-embedding-3-small".to_string(),
            input: text.to_string(),
        };

        let response: EmbedResponse = self.post("embeddings", &request)?;

        Ok(response
            .data
            .first()
            .map(|d| d.embedding.clone())
            .unwrap_or_default())
    }

    fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
}
