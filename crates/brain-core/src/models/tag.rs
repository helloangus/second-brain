//! Tag model

use serde::{Deserialize, Serialize};

/// Tag with confidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
}

fn default_confidence() -> f64 {
    1.0
}

impl Tag {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            confidence: 1.0,
        }
    }

    pub fn with_confidence(name: impl Into<String>, confidence: f64) -> Self {
        Self {
            name: name.into(),
            confidence,
        }
    }
}

impl From<&str> for Tag {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for Tag {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}
