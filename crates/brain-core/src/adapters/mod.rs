//! AI Model adapters

mod minimax_adapter;
mod model_adapter;
mod ollama_adapter;
mod openai_adapter;

pub use minimax_adapter::MiniMaxAdapter;
pub use model_adapter::*;
pub use ollama_adapter::OllamaAdapter;
pub use openai_adapter::OpenAIAdapter;

pub use crate::models::RawDataType;
