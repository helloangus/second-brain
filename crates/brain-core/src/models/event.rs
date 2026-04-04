//! Event model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Meeting,
    Photo,
    Note,
    Activity,
    Research,
    Reading,
    Exercise,
    Meal,
    Work,
    #[default]
    Other,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventType::Meeting => write!(f, "meeting"),
            EventType::Photo => write!(f, "photo"),
            EventType::Note => write!(f, "note"),
            EventType::Activity => write!(f, "activity"),
            EventType::Research => write!(f, "research"),
            EventType::Reading => write!(f, "reading"),
            EventType::Exercise => write!(f, "exercise"),
            EventType::Meal => write!(f, "meal"),
            EventType::Work => write!(f, "work"),
            EventType::Other => write!(f, "other"),
        }
    }
}

impl EventType {
    /// Get Chinese display name
    pub fn display_zh(&self) -> &'static str {
        match self {
            EventType::Meeting => "会议",
            EventType::Photo => "照片",
            EventType::Note => "笔记",
            EventType::Activity => "活动",
            EventType::Research => "研究",
            EventType::Reading => "阅读",
            EventType::Exercise => "锻炼",
            EventType::Meal => "用餐",
            EventType::Work => "工作",
            EventType::Other => "其他",
        }
    }

    /// Parse EventType from string
    pub fn try_from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "meeting" => Some(EventType::Meeting),
            "photo" => Some(EventType::Photo),
            "note" => Some(EventType::Note),
            "activity" => Some(EventType::Activity),
            "research" => Some(EventType::Research),
            "reading" => Some(EventType::Reading),
            "exercise" => Some(EventType::Exercise),
            "meal" => Some(EventType::Meal),
            "work" => Some(EventType::Work),
            "other" => Some(EventType::Other),
            _ => None,
        }
    }
}

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

/// Entity references within an event
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct EventEntities {
    #[serde(default)]
    pub people: Vec<String>,
    #[serde(default)]
    pub organizations: Vec<String>,
    #[serde(default)]
    pub projects: Vec<String>,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub concepts: Vec<String>,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub activities: Vec<String>,
    #[serde(default)]
    pub goals: Vec<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub places: Vec<String>,
    #[serde(default)]
    pub devices: Vec<String>,
    #[serde(default)]
    pub resources: Vec<String>,
    #[serde(default)]
    pub memory_clusters: Vec<String>,
    #[serde(default)]
    pub states: Vec<String>,
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
    pub type_: EventType,
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
}

fn rand_u8() -> u8 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (nanos % 256) as u8
}
