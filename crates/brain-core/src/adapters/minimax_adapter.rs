//! MiniMax adapter implementation

use crate::adapters::{
    build_dict_context, build_reasoning_step1_prompt, build_reasoning_step2_prompt,
    build_step1_prompt, build_step2_prompt, http_client::HttpClient, parse_json, AnalysisOutput,
    AnalysisOutputWithNewEntries, HttpClientConfig, ModelAdapter, RawDataInput, RawDataType,
    ReasoningAdapter, Step1Output, Step2Output, SummarizeAdapter, DEFAULT_MAX_CONTENT_CHARS,
};
use crate::error::{Error, Result};
use crate::models::TaskType;
use serde::Deserialize;
use serde::Serialize;

/// MiniMax adapter for cloud LLM inference
pub struct MiniMaxAdapter {
    client: HttpClient,
    model: String,
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
    pub fn new(endpoint: &str, api_key: &str, model: &str, thinking: bool) -> Result<Self> {
        let config = HttpClientConfig::new(endpoint, Some(api_key));
        Ok(Self {
            client: HttpClient::new(config),
            model: model.to_string(),
            thinking,
        })
    }

    fn post<T: for<'de> Deserialize<'de>>(&self, path: &str, body: &impl Serialize) -> Result<T> {
        self.client.post(path, body)
    }

    fn build_thinking_config(&self) -> Option<ThinkingConfig> {
        if self.thinking {
            Some(ThinkingConfig {
                type_: "thinking".to_string(),
                enabled: Some(true),
            })
        } else {
            None
        }
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
        vec![TaskType::Summarize, TaskType::Reasoning]
    }

    fn health_check(&self) -> Result<bool> {
        Ok(true)
    }

    fn analyze(
        &self,
        task: TaskType,
        input: &RawDataInput,
    ) -> Result<AnalysisOutputWithNewEntries> {
        match task {
            TaskType::Summarize => self.summarize(input),
            TaskType::Reasoning => self.reason(input),
            _ => Err(Error::Config(format!(
                "MiniMaxAdapter does not support {:?}",
                task
            ))),
        }
    }
}

impl SummarizeAdapter for MiniMaxAdapter {
    fn summarize(&self, input: &RawDataInput) -> Result<AnalysisOutputWithNewEntries> {
        let content = std::fs::read_to_string(&input.path).map_err(Error::Io)?;
        let max_chars = input.max_chars.unwrap_or(DEFAULT_MAX_CONTENT_CHARS);
        let truncated_content = content.chars().take(max_chars).collect::<String>();

        // === STEP 1: Free analysis (no dictionary constraints) ===
        let step1_prompt = build_step1_prompt(&input.data_type.to_string(), &truncated_content);

        let step1_response: MiniMaxResponse = self.post(
            "chat/completions",
            &MiniMaxRequest {
                model: self.model.clone(),
                messages: vec![Message {
                    role: "user".to_string(),
                    content: step1_prompt,
                }],
                temperature: 0.7,
                thinking: self.build_thinking_config(),
            },
        )?;

        let step1_content = step1_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        // Parse Step 1 output
        let step1: Step1Output = parse_json(&step1_content, "Step1")?;

        // === STEP 2: Dictionary-aligned analysis ===
        let dict_context_str = if let Some(ref dict_set) = input.dict_set {
            build_dict_context(dict_set)
        } else {
            String::new()
        };

        let step2_prompt = build_step2_prompt(&step1, &dict_context_str);

        let step2_response: MiniMaxResponse = self.post(
            "chat/completions",
            &MiniMaxRequest {
                model: self.model.clone(),
                messages: vec![Message {
                    role: "user".to_string(),
                    content: step2_prompt,
                }],
                temperature: 0.7,
                thinking: self.build_thinking_config(),
            },
        )?;

        let step2_content = step2_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        // Parse Step 2 output
        let step2: Step2Output = parse_json(&step2_content, "Step2")?;

        // Build final output
        let analysis = AnalysisOutput {
            summary: step2.final_.summary,
            extended: step2.final_.extended,
            type_: step2.final_.type_,
            subtype: step2.final_.subtype,
            tags: step2.final_.tags,
            topics: step2.final_.topics,
            entities: step2.final_.entities,
            confidence: step2.final_.confidence,
            raw_response: serde_json::Value::String(step2_content),
        };

        Ok(AnalysisOutputWithNewEntries {
            analysis,
            new_entries: step2.new_entries,
        })
    }
}

impl ReasoningAdapter for MiniMaxAdapter {
    fn reason(&self, input: &RawDataInput) -> Result<AnalysisOutputWithNewEntries> {
        let content = std::fs::read_to_string(&input.path).map_err(Error::Io)?;
        let max_chars = input.max_chars.unwrap_or(DEFAULT_MAX_CONTENT_CHARS);
        let truncated_content = content.chars().take(max_chars).collect::<String>();

        // === STEP 1: Free reasoning analysis ===
        let step1_prompt =
            build_reasoning_step1_prompt(&input.data_type.to_string(), &truncated_content);

        let step1_response: MiniMaxResponse = self.post(
            "chat/completions",
            &MiniMaxRequest {
                model: self.model.clone(),
                messages: vec![Message {
                    role: "user".to_string(),
                    content: step1_prompt,
                }],
                temperature: 0.7,
                thinking: self.build_thinking_config(),
            },
        )?;

        let step1_content = step1_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        // Parse Step 1 output
        let step1: Step1Output = parse_json(&step1_content, "Step1")?;

        // === STEP 2: Dictionary-aligned analysis ===
        let dict_context_str = if let Some(ref dict_set) = input.dict_set {
            build_dict_context(dict_set)
        } else {
            String::new()
        };

        let step2_prompt = build_reasoning_step2_prompt(&step1, &dict_context_str);

        let step2_response: MiniMaxResponse = self.post(
            "chat/completions",
            &MiniMaxRequest {
                model: self.model.clone(),
                messages: vec![Message {
                    role: "user".to_string(),
                    content: step2_prompt,
                }],
                temperature: 0.7,
                thinking: self.build_thinking_config(),
            },
        )?;

        let step2_content = step2_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        // Parse Step 2 output
        let step2: Step2Output = parse_json(&step2_content, "Step2")?;

        let analysis = AnalysisOutput {
            summary: step2.final_.summary,
            extended: step2.final_.extended,
            type_: step2.final_.type_,
            subtype: step2.final_.subtype,
            tags: step2.final_.tags,
            topics: step2.final_.topics,
            entities: step2.final_.entities,
            confidence: step2.final_.confidence,
            raw_response: serde_json::Value::String(step2_content),
        };

        Ok(AnalysisOutputWithNewEntries {
            analysis,
            new_entries: step2.new_entries,
        })
    }
}
