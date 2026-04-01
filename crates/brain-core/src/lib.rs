//! brain-core - Core library for Second Brain
//!
//! Provides data models, database operations, and markdown parsing.

pub mod error;
pub mod models;
pub mod db;
pub mod markdown;
pub mod adapters;
pub mod config;

pub use error::{Error, Result};
pub use models::*;
pub use db::{Database, EventRepository, EntityRepository, TagRepository};
pub use config::BrainConfig;
