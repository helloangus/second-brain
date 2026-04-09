//! AI Model adapters

mod minimax_adapter;
mod model_adapter;
mod ollama_adapter;
mod openai_adapter;
mod router;

pub use minimax_adapter::MiniMaxAdapter;
pub use model_adapter::{ReasoningAdapter, SummarizeAdapter, *};
pub use ollama_adapter::OllamaAdapter;
pub use openai_adapter::OpenAIAdapter;
pub use router::ModelRegistry;

pub use crate::models::RawDataType;
