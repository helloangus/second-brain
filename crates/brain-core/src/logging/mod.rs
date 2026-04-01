//! Logging system for Second Brain
//!
//! Provides structured logging for CRUD operations, AI processing,
//! pipeline operations, and system events.

pub mod log;
pub mod logger;
pub mod repo;

pub use log::*;
pub use logger::Logger;
pub use repo::{AiProcessingStats, LogRepository};
