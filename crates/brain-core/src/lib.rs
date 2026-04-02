//! brain-core - Core library for Second Brain
//!
//! Provides data models, database operations, and markdown parsing.

pub mod adapters;
pub mod config;
pub mod db;
pub mod dicts;
pub mod error;
pub mod logging;
pub mod markdown;
pub mod models;

pub use config::BrainConfig;
pub use db::{Database, EntityRepository, EventRepository, TagRepository};
pub use dicts::{Dict, DictEntry, DictSet};
pub use error::{Error, Result};
pub use logging::{CrudOperation, LogEntry, LogLevel, LogSource, LogType, Logger, TargetType};
pub use models::*;
