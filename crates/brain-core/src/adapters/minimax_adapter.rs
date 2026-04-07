//! MiniMax adapter implementation

use crate::adapters::{
    AnalysisOutputWithNewEntries, ModelAdapter, NewDictEntries, RawDataInput, RawDataType,
    SummarizeAdapter,
};
use crate::dicts::DictSet;
use crate::error::{Error, Result};
use crate::models::TaskType;
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

    fn supported_task_types(&self) -> Vec<TaskType> {
        vec![TaskType::Summarize]
    }

    fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
}

impl SummarizeAdapter for MiniMaxAdapter {
    fn summarize(&self, input: &RawDataInput) -> Result<AnalysisOutputWithNewEntries> {
        let content = std::fs::read_to_string(&input.path).map_err(Error::Io)?;
        let truncated_content = content.chars().take(2000).collect::<String>();

        // === STEP 1: Free analysis (no dictionary constraints) ===
        let step1_prompt = format!(
            r#"分析这个{}并提供:
1. 简短摘要（2-3句话）
2. 扩展内容 - 当内容复杂、有多个要点或细节无法用2-3句话概括时使用此字段。此字段没有长度限制。
3. 事件类型（根据内容含义自由选择）
4. 事件子类型（根据内容含义自由选择）
5. 关键标签（需要时创建新的 - 要有创意且具体）
6. 关键主题（需要时创建新的 - 要有创意且具体）
7. 提到的任何实体

重要：完全根据内容选择最描述性的类型、子类型、标签和主题。不要试图匹配现有值。如果没有完全合适的就创建新值。

内容:
{}

请以JSON格式回复:
{{
    "summary": "2-3句话的简短摘要",
    "extended": "详细内容，如果没有合适内容则填null",
    "type": "自由选择的事件类型",
    "subtype": "自由选择的子类型",
    "tags": ["标签1", "标签2"],
    "topics": ["主题1", "主题2"],
    "entities": ["实体1"],
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
        let dict_context_str = if let Some(ref dict_set) = input.dict_set {
            Self::build_dict_context(dict_set)
        } else {
            String::new()
        };

        let step2_prompt = format!(
            r#"回顾你的初步分析，并与现有字典进行对齐（如果适用）。

初步分析:
{}

现有字典:
{}

任务:
对于每个字段（类型、子类型、标签、主题）:
- 如果初步值匹配现有字典条目 → 使用现有条目（使用精确的key）
- 如果初步值是新的（不在字典中）→ 保留它作为新值，它将被添加到字典中

重要：当现有字典值合适时优先使用。但不要强制匹配，如果初步值确实不同或更准确的话。

请以JSON格式回复:
{{
    "final": {{
        "summary": "简短摘要",
        "extended": "详细内容或null",
        "type": "现有或新的事件类型",
        "subtype": "现有或新的子类型",
        "tags": ["标签1", "标签2"],
        "topics": ["主题1", "主题2"],
        "entities": ["实体1"],
        "confidence": 0.0-1.0
    }},
    "new_entries": {{
        "event_types": [{{"key": "新类型", "zh": null, "description": null}}],
        "event_subtypes": [],
        "tags": [{{"key": "新标签", "zh": null, "description": null}}],
        "topics": [{{"key": "新主题", "zh": null, "description": null}}]
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
}

impl MiniMaxAdapter {
    /// Build dictionary context string for Step 2 prompt
    pub fn build_dict_context(dict_set: &DictSet) -> String {
        let mut parts = Vec::new();

        // Event Types
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
            parts.push(format!("事件类型:\n{}", entries.join("\n")));
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
            parts.push(format!("事件子类型:\n{}", entries.join("\n")));
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
            parts.push(format!("标签:\n{}", entries.join("\n")));
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
            parts.push(format!("主题:\n{}", entries.join("\n")));
        }

        if parts.is_empty() {
            "暂无现有字典条目。".to_string()
        } else {
            parts.join("\n\n")
        }
    }
}
