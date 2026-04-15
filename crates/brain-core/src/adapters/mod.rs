//! AI Model adapters

mod dict_context;
mod http_client;
mod minimax_adapter;
mod model_adapter;
mod ollama_adapter;
mod prompts;
mod response;
mod router;

pub use minimax_adapter::MiniMaxAdapter;
pub use model_adapter::{ReasoningAdapter, SummarizeAdapter, *};
pub use ollama_adapter::OllamaAdapter;
pub use router::ModelRegistry;

pub use crate::models::RawDataType;

// Re-export shared items for internal adapter use
pub(crate) use dict_context::build_dict_context;
pub(crate) use http_client::HttpClientConfig;
pub(crate) use prompts::{
    build_reasoning_step1_prompt, build_reasoning_step2_prompt, build_step1_prompt,
    build_step2_prompt,
};
pub(crate) use response::{parse_json, Step1Output, Step2Input, Step2Output};
