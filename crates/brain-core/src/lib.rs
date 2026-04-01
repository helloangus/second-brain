//! brain-core - Core library for Second Brain
//!
//! Provides data models, database operations, and markdown parsing.

pub mod adapters;
pub mod config;
pub mod db;
pub mod error;
pub mod markdown;
pub mod models;

pub use config::BrainConfig;
pub use db::{Database, EntityRepository, EventRepository, TagRepository};
pub use error::{Error, Result};
pub use models::*;
