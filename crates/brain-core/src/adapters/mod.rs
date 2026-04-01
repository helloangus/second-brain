//! AI Model adapters

mod model_adapter;
mod ollama_adapter;
mod openai_adapter;

pub use model_adapter::*;
pub use ollama_adapter::OllamaAdapter;
pub use openai_adapter::OpenAIAdapter;

pub use crate::models::RawDataType;
