//! OpenAI adapter implementation

use crate::adapters::{AnalysisOutput, ModelAdapter, RawDataInput, RawDataType};
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

/// OpenAI adapter for cloud LLM inference
pub struct OpenAIAdapter {
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
        Ok(Self {
            api_key: api_key.to_string(),
            model: model.to_string(),
        })
    }

    fn post<T: for<'de> Deserialize<'de>>(&self, path: &str, body: &impl Serialize) -> Result<T> {
        let url = format!("https://api.openai.com/v1/{}", path);
        let body_str =
            serde_json::to_string(body).map_err(|e| Error::Config(format!("JSON error: {}", e)))?;

        let response = ureq::post(&url)
            .set("Content-Type", "application/json")
            .set("Authorization", &format!("Bearer {}", self.api_key))
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

impl ModelAdapter for OpenAIAdapter {
    fn name(&self) -> &str {
        "openai"
    }

    fn supported_data_types(&self) -> Vec<RawDataType> {
        vec![RawDataType::Text, RawDataType::Image, RawDataType::Document]
    }

    fn analyze(&self, input: &RawDataInput) -> Result<AnalysisOutput> {
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
            extended: None,
            type_: None,
            subtype: None,
            tags: Vec::new(),
            topics: Vec::new(),
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
