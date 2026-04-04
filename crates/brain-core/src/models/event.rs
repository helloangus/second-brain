//! Event model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::models::EntityType;
use crate::DictSet;

/// Source information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct EventSource {
    pub device: Option<String>,
    pub channel: Option<String>,
    pub capture_agent: Option<String>,
}

/// Time information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EventTime {
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    #[serde(default = "default_timezone")]
    pub timezone: String,
}

fn default_timezone() -> String {
    "UTC".to_string()
}

/// AI analysis output
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct EventAi {
    pub summary: Option<String>,
    /// Extended content - longer text that doesn't fit in summary
    #[serde(default)]
    pub extended: Option<String>,
    #[serde(default)]
    pub topics: Vec<String>,
    pub sentiment: Option<String>,
    pub extraction_version: Option<i32>,
}

/// Relations to other events
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct EventRelations {
    #[serde(default)]
    pub inferred_from: Vec<String>,
}

/// Graph hints for the cognitive engine
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct GraphHints {
    pub importance: Option<f64>,
    #[serde(default)]
    pub recurrence: bool,
}

/// Entity references within an event, keyed by entity type
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventEntities(pub(crate) BTreeMap<EntityType, Vec<String>>);

impl EventEntities {
    /// Create from untyped entity list (e.g., from AI output) - stored under Concept
    pub fn from_untyped(ids: impl IntoIterator<Item = String>) -> Self {
        let ids: Vec<String> = ids.into_iter().collect();
        if ids.is_empty() {
            Self(BTreeMap::new())
        } else {
            let mut map = BTreeMap::new();
            map.insert(EntityType::Concept, ids);
            Self(map)
        }
    }

    /// Add an entity of the given type
    pub fn add_entity(&mut self, type_: EntityType, id: impl Into<String>) {
        self.0.entry(type_).or_default().push(id.into());
    }

    /// Get all entities of a given type
    pub fn get(&self, type_: EntityType) -> &[String] {
        self.0.get(&type_).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get total count of all entities
    pub fn total_count(&self) -> usize {
        self.0.values().map(|v| v.len()).sum()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.0.values().all(|v| v.is_empty())
    }
}

/// References to raw data files
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct RawRefs {
    #[serde(default)]
    pub files: Vec<String>,
}

/// References to derived data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct DerivedRefs {
    pub transcript: Option<String>,
    pub embedding: Option<String>,
}

/// Full Event struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub schema: String,
    pub id: String,
    #[serde(default)]
    pub type_: String,
    pub subtype: Option<String>,
    pub time: EventTime,
    pub created_at: Option<DateTime<Utc>>,
    pub ingested_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub source: EventSource,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    #[serde(default)]
    pub entities: EventEntities,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub raw_refs: RawRefs,
    #[serde(default)]
    pub derived_refs: DerivedRefs,
    #[serde(default)]
    pub ai: EventAi,
    #[serde(default)]
    pub relations: EventRelations,
    #[serde(default)]
    pub graph_hints: GraphHints,
    #[serde(default = "default_schema_version")]
    pub schema_version: i32,
}

fn default_confidence() -> f64 {
    0.5
}

fn default_schema_version() -> i32 {
    1
}

impl Event {
    /// Generate a new event ID
    pub fn generate_id() -> String {
        let now = chrono::Utc::now();
        let rand_hex: String = (0..3)
            .map(|_| {
                let idx = rand_u8() % 16;
                "0123456789abcdef".chars().nth(idx as usize).unwrap()
            })
            .collect();
        format!("evt-{}-{}", now.format("%Y%m%d-%H%M%S"), rand_hex)
    }

    /// Get Chinese display name for event type via dictionary lookup
    pub fn type_display_zh(&self, dicts: &DictSet) -> String {
        dicts
            .event_type
            .lookup(&self.type_)
            .map(|e| e.zh.clone().unwrap_or_else(|| self.type_.clone()))
            .unwrap_or_else(|| self.type_.clone())
    }
}

fn rand_u8() -> u8 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (nanos % 256) as u8
}
