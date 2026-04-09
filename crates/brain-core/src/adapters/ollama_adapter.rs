//! Ollama adapter implementation

use crate::adapters::{
    build_dict_context, build_step1_prompt, build_step2_prompt, http_client::HttpClient,
    parse_json, AnalysisOutput, AnalysisOutputWithNewEntries, HttpClientConfig, ModelAdapter,
    RawDataInput, RawDataType, Step1Output, Step2Output, SummarizeAdapter,
    DEFAULT_MAX_CONTENT_CHARS,
};
use crate::error::{Error, Result};
use crate::models::TaskType;
use serde::{Deserialize, Serialize};

/// Ollama adapter for local LLM inference
pub struct OllamaAdapter {
    client: HttpClient,
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

impl OllamaAdapter {
    /// Create a new Ollama adapter
    pub fn new(endpoint: &str, model: &str) -> Result<Self> {
        let config = HttpClientConfig::new(endpoint, None);
        Ok(Self {
            client: HttpClient::new(config),
            model: model.to_string(),
        })
    }

    fn post<T: for<'de> Deserialize<'de>>(&self, path: &str, body: &impl Serialize) -> Result<T> {
        self.client.post(path, body)
    }
}

impl ModelAdapter for OllamaAdapter {
    fn name(&self) -> &str {
        "ollama"
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

    fn analyze(
        &self,
        task: TaskType,
        input: &RawDataInput,
    ) -> Result<AnalysisOutputWithNewEntries> {
        match task {
            TaskType::Summarize => self.summarize(input),
            _ => Err(Error::Config(format!(
                "OllamaAdapter does not support {:?}",
                task
            ))),
        }
    }
}

impl SummarizeAdapter for OllamaAdapter {
    fn summarize(&self, input: &RawDataInput) -> Result<AnalysisOutputWithNewEntries> {
        let content = std::fs::read_to_string(&input.path).map_err(Error::Io)?;
        let max_chars = input.max_chars.unwrap_or(DEFAULT_MAX_CONTENT_CHARS);
        let truncated_content = content.chars().take(max_chars).collect::<String>();

        // === STEP 1: Free analysis (no dictionary constraints) ===
        let step1_prompt = build_step1_prompt(&input.data_type.to_string(), &truncated_content);

        let step1_response: OllamaResponse = self.post(
            "api/generate",
            &OllamaRequest {
                model: self.model.clone(),
                prompt: step1_prompt.clone(),
                stream: false,
            },
        )?;

        // Parse Step 1 output
        let step1: Step1Output = parse_json(&step1_response.response, "Step1")?;

        // === STEP 2: Dictionary-aligned analysis ===
        let dict_context_str = if let Some(ref dict_set) = input.dict_set {
            build_dict_context(dict_set)
        } else {
            String::new()
        };

        let step2_prompt = build_step2_prompt(&step1, &dict_context_str);

        let step2_response: OllamaResponse = self.post(
            "api/generate",
            &OllamaRequest {
                model: self.model.clone(),
                prompt: step2_prompt.clone(),
                stream: false,
            },
        )?;

        // Parse Step 2 output
        let step2: Step2Output = parse_json(&step2_response.response, "Step2")?;

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
            raw_response: serde_json::Value::String(step2_response.response.clone()),
        };

        Ok(AnalysisOutputWithNewEntries {
            analysis,
            new_entries: step2.new_entries,
        })
    }
}
