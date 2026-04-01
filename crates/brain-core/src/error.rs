//! Error types for brain-core

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("YAML parse error: {0}")]
    YamlParse(#[from] serde_yaml::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Markdown parse error: {0}")]
    MarkdownParse(String),

    #[error("Event not found: {0}")]
    EventNotFound(String),

    #[error("Entity not found: {0}")]
    EntityNotFound(String),

    #[error("Invalid ID format: {0}")]
    InvalidIdFormat(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, Error>;
