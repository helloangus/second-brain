//! MiniMax adapter implementation

use crate::adapters::{
    AnalysisOutputWithNewEntries, DictContext, ModelAdapter, NewDictEntries, RawDataInput,
    RawDataType,
};
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

/// Deserialize a number that might be a string or a number
fn deserialize_number_or_string<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(opt.and_then(|v| match v {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    }))
}

/// MiniMax adapter for cloud LLM inference
pub struct MiniMaxAdapter {
    api_key: String,
    model: String,
    endpoint: String,
    thinking: bool,
}

#[derive(Debug, Serialize)]
struct MiniMaxRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<ThinkingConfig>,
}

#[derive(Debug, Clone, Serialize)]
struct ThinkingConfig {
    #[serde(rename = "type")]
    type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct MiniMaxResponse {
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

impl MiniMaxAdapter {
    /// Create a new MiniMax adapter
    pub fn new(api_key: &str, model: &str, endpoint: &str, thinking: bool) -> Result<Self> {
        Ok(Self {
            api_key: api_key.to_string(),
            model: model.to_string(),
            endpoint: endpoint.to_string().trim_end_matches('/').to_string(),
            thinking,
        })
    }

    /// Extract JSON from content that may be wrapped in markdown code blocks
    fn extract_json(content: &str) -> &str {
        // Try to find JSON inside markdown code blocks first
        if let Some(start) = content.find("```json") {
            let after_start = &content[start + 7..];
            // Find the closing ``` - look for it near the end of the content first
            // Sometimes there are multiple ``` patterns, so we try from the end
            let search_content = after_start.trim();
            if let Some(json_start) = search_content.find('{') {
                let potential = &search_content[json_start..];
                // Just use find_matching_brace since we're now working with trimmed content
                if let Some(end) = Self::find_matching_brace(potential) {
                    return &potential[..=end];
                }
            }
        }
        // Try generic code blocks (without json specifier)
        if let Some(start) = content.find("```") {
            let after_code = &content[start + 3..];
            let search_content = after_code.trim();
            if let Some(code_start) = search_content.find('{') {
                let potential = &search_content[code_start..];
                if let Some(end) = Self::find_matching_brace(potential) {
                    return &potential[..=end];
                }
            }
        }
        // Try to find raw JSON object
        if let Some(start) = content.find('{') {
            let potential = &content[start..];
            if let Some(end) = Self::find_matching_brace(potential) {
                return &potential[..=end];
            }
        }
        // Fallback: return original content
        content
    }

    /// Find the matching closing brace for an opening brace
    /// Returns the position of the closing brace that completes the JSON object
    fn find_matching_brace(s: &str) -> Option<usize> {
        // First, validate that we start with {
        let first_char = s.chars().next()?;
        if first_char != '{' {
            return None;
        }

        // Find the position of the last } in the string
        let last_brace = s.rfind('}')?;

        // Return the position of the last }
        // This works because the JSON structure should end with the outermost }
        Some(last_brace)
    }

    fn post<T: for<'de> Deserialize<'de>>(&self, path: &str, body: &impl Serialize) -> Result<T> {
        let url = format!("{}/{}", self.endpoint, path);
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
            .map_err(|e| Error::Config(format!("Parse error: {} | Response: {}", e, &text)))?;
        Ok(result)
    }
}

impl ModelAdapter for MiniMaxAdapter {
    fn name(&self) -> &str {
        "minimax"
    }

    fn supported_data_types(&self) -> Vec<RawDataType> {
        vec![RawDataType::Text, RawDataType::Image, RawDataType::Document]
    }

    fn analyze(&self, input: &RawDataInput) -> Result<AnalysisOutputWithNewEntries> {
        let content = std::fs::read_to_string(&input.path).map_err(Error::Io)?;
        let truncated_content = content.chars().take(2000).collect::<String>();

        // === STEP 1: Free analysis (no dictionary constraints) ===
        let step1_prompt = format!(
            r#"Analyze this {} and provide:
1. A brief summary (2-3 sentences)
2. Extended content - use this field whenever the content has complexity, multiple points, or details that don't fit in a 2-3 sentence summary. This field has no length limit.
3. Event type (choose freely based on content meaning)
4. Event subtype (choose freely based on content meaning)
5. Key tags (create new ones if needed - be creative and specific)
6. Key topics (create new ones if needed - be creative and specific)
7. Any entities mentioned

IMPORTANT: Choose type, subtype, tags, and topics based SOLELY on what best describes the content. Do NOT try to match existing values. Create new values if nothing existing fits perfectly.

Content:
{}

Respond in JSON format:
{{
    "summary": "2-3 sentence brief summary",
    "extended": "detailed content OR null if content is simple",
    "type": "freely chosen event type",
    "subtype": "freely chosen subtype",
    "tags": ["tag1", "tag2"],
    "topics": ["topic1", "topic2"],
    "entities": ["entity1"],
    "confidence": 0.0-1.0
}}"#,
            input.data_type, truncated_content
        );

        let thinking_config = if self.thinking {
            Some(ThinkingConfig {
                type_: "thinking".to_string(),
                enabled: Some(true),
            })
        } else {
            None
        };

        let step1_response: MiniMaxResponse = self.post(
            "chat/completions",
            &MiniMaxRequest {
                model: self.model.clone(),
                messages: vec![Message {
                    role: "user".to_string(),
                    content: step1_prompt,
                }],
                temperature: 0.7,
                thinking: thinking_config.clone(),
            },
        )?;

        let step1_content_raw = step1_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();
        let step1_content = Self::extract_json(&step1_content_raw);

        // Parse Step 1 output
        #[derive(Deserialize, Serialize)]
        struct Step1Output {
            summary: Option<String>,
            extended: Option<String>,
            #[serde(rename = "type")]
            type_: Option<String>,
            subtype: Option<String>,
            tags: Vec<String>,
            topics: Vec<String>,
            entities: Vec<String>,
            #[serde(deserialize_with = "deserialize_number_or_string", default)]
            confidence: Option<f64>,
        }

        let step1: Step1Output = serde_json::from_str(step1_content)
            .map_err(|e| Error::Config(format!("Step1 parse error: {}", e)))?;

        // === STEP 2: Dictionary-aligned analysis ===
        let dict_context_str = if let Some(ref ctx) = input.dict_context {
            Self::build_dict_context(ctx)
        } else {
            String::new()
        };

        let step2_prompt = format!(
            r#"Review your initial analysis and align with existing dictionary where possible.

INITIAL ANALYSIS:
{}

EXISTING DICTIONARY:
{}

TASK:
For each field (type, subtype, tags, topics):
- If initial value matches an existing dictionary entry -> USE the existing entry (with exact key)
- If initial value is NEW (not in dictionary) -> KEEP it as NEW and it will be added to the dictionary

IMPORTANT: Prefer existing dictionary values when they fit well. But don't force a match if the initial value is genuinely different or more accurate.

Respond in JSON format:
{{
    "final": {{
        "summary": "brief summary",
        "extended": "detailed content OR null",
        "type": "existing or new event type",
        "subtype": "existing or new subtype",
        "tags": ["tag1", "tag2"],
        "topics": ["topic1", "topic2"],
        "entities": ["entity1"],
        "confidence": 0.0-1.0
    }},
    "new_entries": {{
        "event_types": [{{"key": "new_type", "zh": null, "description": null}}],
        "event_subtypes": [],
        "tags": [{{"key": "new_tag", "zh": null, "description": null}}],
        "topics": [{{"key": "new_topic", "zh": null, "description": null}}]
    }}
}}"#,
            serde_json::to_string(&step1).unwrap_or_default(),
            dict_context_str
        );

        let step2_response: MiniMaxResponse = self.post(
            "chat/completions",
            &MiniMaxRequest {
                model: self.model.clone(),
                messages: vec![Message {
                    role: "user".to_string(),
                    content: step2_prompt,
                }],
                temperature: 0.7,
                thinking: thinking_config,
            },
        )?;

        let step2_content_raw = step2_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();
        let step2_content = Self::extract_json(&step2_content_raw);

        // Parse Step 2 output
        #[derive(Deserialize)]
        struct Step2Final {
            summary: Option<String>,
            extended: Option<String>,
            #[serde(rename = "type")]
            type_: Option<String>,
            subtype: Option<String>,
            tags: Vec<String>,
            topics: Vec<String>,
            entities: Vec<String>,
            #[serde(deserialize_with = "deserialize_number_or_string", default)]
            confidence: Option<f64>,
        }

        #[derive(Deserialize)]
        struct Step2Output {
            #[serde(rename = "final")]
            final_: Step2Final,
            new_entries: NewDictEntries,
        }

        let step2: Step2Output = serde_json::from_str(step2_content)
            .map_err(|e| Error::Config(format!("Step2 parse error: {}", e)))?;

        // Build final output
        let analysis = crate::adapters::AnalysisOutput {
            summary: step2.final_.summary,
            extended: step2.final_.extended,
            type_: step2.final_.type_,
            subtype: step2.final_.subtype,
            tags: step2.final_.tags,
            topics: step2.final_.topics,
            entities: step2.final_.entities,
            confidence: step2.final_.confidence,
            raw_response: serde_json::Value::String(step2_content.to_string()),
        };

        Ok(AnalysisOutputWithNewEntries {
            analysis,
            new_entries: step2.new_entries,
        })
    }

    fn summarize(&self, text: &str) -> Result<String> {
        let thinking_config = if self.thinking {
            Some(ThinkingConfig {
                type_: "thinking".to_string(),
                enabled: Some(true),
            })
        } else {
            None
        };

        let request = MiniMaxRequest {
            model: self.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: format!("Summarize the following text in 2-3 sentences:\n\n{}", text),
            }],
            temperature: 0.7,
            thinking: thinking_config,
        };

        let response: MiniMaxResponse = self.post("chat/completions", &request)?;

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
            model: "embo-01".to_string(),
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

impl MiniMaxAdapter {
    /// Build dictionary context string for Step 2 prompt
    pub fn build_dict_context(ctx: &DictContext) -> String {
        let mut parts = Vec::new();

        // Event Types
        if let Some(ref dict_set) = ctx.dict_set {
            let entries: Vec<String> = dict_set
                .event_type
                .list()
                .iter()
                .map(|e| {
                    let zh = e.zh.as_deref().unwrap_or("");
                    let desc = e.description.as_deref().unwrap_or("");
                    if zh.is_empty() && desc.is_empty() {
                        format!("  - {}", e.key)
                    } else if zh.is_empty() {
                        format!("  - {}: {}", e.key, desc)
                    } else if desc.is_empty() {
                        format!("  - {} ({})", e.key, zh)
                    } else {
                        format!("  - {} ({}): {}", e.key, zh, desc)
                    }
                })
                .collect();
            if !entries.is_empty() {
                parts.push(format!("Event Types:\n{}", entries.join("\n")));
            }

            // Event Subtypes
            let entries: Vec<String> = dict_set
                .event_subtype
                .list()
                .iter()
                .map(|e| {
                    let zh = e.zh.as_deref().unwrap_or("");
                    let desc = e.description.as_deref().unwrap_or("");
                    if zh.is_empty() && desc.is_empty() {
                        format!("  - {}", e.key)
                    } else if zh.is_empty() {
                        format!("  - {}: {}", e.key, desc)
                    } else if desc.is_empty() {
                        format!("  - {} ({})", e.key, zh)
                    } else {
                        format!("  - {} ({}): {}", e.key, zh, desc)
                    }
                })
                .collect();
            if !entries.is_empty() {
                parts.push(format!("Event Subtypes:\n{}", entries.join("\n")));
            }

            // Tags
            let entries: Vec<String> = dict_set
                .tags
                .list()
                .iter()
                .map(|e| {
                    let zh = e.zh.as_deref().unwrap_or("");
                    let desc = e.description.as_deref().unwrap_or("");
                    if zh.is_empty() && desc.is_empty() {
                        format!("  - {}", e.key)
                    } else if zh.is_empty() {
                        format!("  - {}: {}", e.key, desc)
                    } else if desc.is_empty() {
                        format!("  - {} ({})", e.key, zh)
                    } else {
                        format!("  - {} ({}): {}", e.key, zh, desc)
                    }
                })
                .collect();
            if !entries.is_empty() {
                parts.push(format!("Tags:\n{}", entries.join("\n")));
            }

            // Topics
            let entries: Vec<String> = dict_set
                .topics
                .list()
                .iter()
                .map(|e| {
                    let zh = e.zh.as_deref().unwrap_or("");
                    let desc = e.description.as_deref().unwrap_or("");
                    if zh.is_empty() && desc.is_empty() {
                        format!("  - {}", e.key)
                    } else if zh.is_empty() {
                        format!("  - {}: {}", e.key, desc)
                    } else if desc.is_empty() {
                        format!("  - {} ({})", e.key, zh)
                    } else {
                        format!("  - {} ({}): {}", e.key, zh, desc)
                    }
                })
                .collect();
            if !entries.is_empty() {
                parts.push(format!("Topics:\n{}", entries.join("\n")));
            }
        } else {
            // Fallback: use the old Vec<String> fields
            if !ctx.event_types.is_empty() {
                parts.push(format!("Event Types:\n  {}", ctx.event_types.join(", ")));
            }
            if !ctx.event_subtypes.is_empty() {
                parts.push(format!(
                    "Event Subtypes:\n  {}",
                    ctx.event_subtypes.join(", ")
                ));
            }
            if !ctx.tags.is_empty() {
                parts.push(format!("Tags:\n  {}", ctx.tags.join(", ")));
            }
            if !ctx.topics.is_empty() {
                parts.push(format!("Topics:\n  {}", ctx.topics.join(", ")));
            }
        }

        if parts.is_empty() {
            "No existing dictionary entries.".to_string()
        } else {
            parts.join("\n\n")
        }
    }
}
