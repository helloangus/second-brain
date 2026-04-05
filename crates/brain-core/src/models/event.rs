//! Event model
//!
//! Represents timestamped records in the Second Brain system. Events are the
//! core data entity - everything that happens is captured as an event. Events
//! are stored as Markdown files with YAML frontmatter.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::models::EntityType;
use crate::DictSet;

/// Source information
///
/// Describes where the event data originated from.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct EventSource {
    /// Device or application that captured this event
    pub device: Option<String>,
    /// Channel through which the event was received
    pub channel: Option<String>,
    /// Software agent that performed the capture
    pub capture_agent: Option<String>,
}

/// Time information
///
/// Represents the temporal span of an event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EventTime {
    /// Start timestamp of the event
    pub start: DateTime<Utc>,
    /// Optional end timestamp for events with duration
    pub end: Option<DateTime<Utc>>,
    /// IANA timezone name (e.g., "Asia/Tokyo", "UTC")
    #[serde(default = "default_timezone")]
    pub timezone: String,
}

/// Returns the default timezone string ("UTC")
pub fn default_timezone() -> String {
    "UTC".to_string()
}

/// AI analysis output
///
/// Contains AI-generated metadata about the event.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct EventAi {
    /// Brief summary of the event
    pub summary: Option<String>,
    /// Extended content - longer text that doesn't fit in summary
    #[serde(default)]
    pub extended: Option<String>,
    /// Topics associated with this event
    #[serde(default)]
    pub topics: Vec<String>,
    /// Sentiment analysis result (e.g., "positive", "negative", "neutral")
    pub sentiment: Option<String>,
    /// Version of the AI extraction model used
    pub extraction_version: Option<i32>,
}

/// Relations to other events
///
/// Tracks relationships between events.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct EventRelations {
    /// Event IDs from which this event was inferred
    #[serde(default)]
    pub inferred_from: Vec<String>,
}

/// Graph hints for the cognitive engine
///
/// Provides hints to the graph processing engine about how to treat this event.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct GraphHints {
    /// Relative importance score (0.0 - 1.0)
    pub importance: Option<f64>,
    /// Whether this event represents a recurring pattern
    #[serde(default)]
    pub recurrence: bool,
}

/// Entity references within an event, keyed by entity type
///
/// Stores references to entities linked from this event, organized by type.
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
///
/// Stores paths or URLs to raw data files associated with this event.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct RawRefs {
    /// List of raw file paths or URLs
    #[serde(default)]
    pub files: Vec<String>,
}

/// References to derived data
///
/// Stores references to data derived from raw files (e.g., transcriptions, embeddings).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct DerivedRefs {
    /// Path to transcript file (e.g., from audio transcription)
    pub transcript: Option<String>,
    /// Path to embedding file (e.g., from AI embedding generation)
    pub embedding: Option<String>,
}

/// Full Event struct
///
/// Represents a timestamped record in the Second Brain system.
/// Events are the core data entity - everything that happens is captured as an event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Schema identifier for serialization version control (e.g., "event/v1")
    pub schema: String,
    /// Unique identifier for this event (e.g., "evt-20260331-143052-a1b2")
    pub id: String,
    /// Event type category (e.g., "meeting", "photo", "note", "task")
    #[serde(default)]
    pub type_: String,
    /// Optional subtype for more specific categorization
    pub subtype: Option<String>,
    /// Temporal information (start/end time)
    pub time: EventTime,
    /// Creation timestamp in the source system
    pub created_at: Option<DateTime<Utc>>,
    /// Timestamp when this event was ingested into Second Brain
    pub ingested_at: Option<DateTime<Utc>>,
    /// Source information about where this event originated
    #[serde(default)]
    pub source: EventSource,
    /// Confidence score of the AI extraction (0.0 - 1.0)
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    /// Entity references linked from this event
    #[serde(default)]
    pub entities: EventEntities,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
    /// References to raw data files
    #[serde(default)]
    pub raw_refs: RawRefs,
    /// References to derived data
    #[serde(default)]
    pub derived_refs: DerivedRefs,
    /// AI-generated metadata
    #[serde(default)]
    pub ai: EventAi,
    /// Relations to other events
    #[serde(default)]
    pub relations: EventRelations,
    /// Hints for the graph processing engine
    #[serde(default)]
    pub graph_hints: GraphHints,
    /// Schema version for this event
    #[serde(default = "default_schema_version")]
    pub schema_version: i32,
}

/// Returns the default confidence score (0.5)
pub fn default_confidence() -> f64 {
    0.5
}

/// Returns the default schema version (1)
pub fn default_schema_version() -> i32 {
    1
}

/// Returns the default event schema string ("event/v1")
pub fn default_schema() -> String {
    format!("event/v{}", default_schema_version())
}

/// Default event type for new events
pub const DEFAULT_EVENT_TYPE: &str = "observation";

impl Event {
    /// Generate a new event ID
    ///
    /// Creates a unique ID in the format: evt-YYYYMMDD-HHMMSS-rrr
    /// where rrr is a 3-digit random hex string.
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
    ///
    /// Falls back to the type string itself if no dictionary entry exists.
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    // =============================================================================
    // EventTime Tests
    // =============================================================================

    #[test]
    fn test_event_time_construction() {
        let time = EventTime {
            start: Utc.with_ymd_and_hms(2026, 3, 31, 10, 0, 0).unwrap(),
            end: Some(Utc.with_ymd_and_hms(2026, 3, 31, 11, 0, 0).unwrap()),
            timezone: "Asia/Tokyo".to_string(),
        };

        assert_eq!(time.timezone, "Asia/Tokyo");
        assert!(time.end.is_some());
    }

    #[test]
    fn test_event_time_without_end() {
        let time = EventTime {
            start: Utc.with_ymd_and_hms(2026, 3, 31, 10, 0, 0).unwrap(),
            end: None,
            timezone: "UTC".to_string(),
        };

        assert!(time.end.is_none());
    }

    // =============================================================================
    // EventSource Tests
    // =============================================================================

    #[test]
    fn test_event_source_default() {
        let source = EventSource::default();
        assert!(source.device.is_none());
        assert!(source.channel.is_none());
        assert!(source.capture_agent.is_none());
    }

    #[test]
    fn test_event_source_full() {
        let source = EventSource {
            device: Some("macbook".to_string()),
            channel: Some("cli".to_string()),
            capture_agent: Some("brain-cli".to_string()),
        };

        assert_eq!(source.device.as_deref(), Some("macbook"));
        assert_eq!(source.channel.as_deref(), Some("cli"));
        assert_eq!(source.capture_agent.as_deref(), Some("brain-cli"));
    }

    // =============================================================================
    // EventAi Tests
    // =============================================================================

    #[test]
    fn test_event_ai_default() {
        let ai = EventAi::default();
        assert!(ai.summary.is_none());
        assert!(ai.extended.is_none());
        assert!(ai.topics.is_empty());
        assert!(ai.sentiment.is_none());
        assert!(ai.extraction_version.is_none());
    }

    #[test]
    fn test_event_ai_full() {
        let ai = EventAi {
            summary: Some("Meeting about Q1 planning".to_string()),
            extended: Some("Detailed discussion of sprint goals".to_string()),
            topics: vec!["planning".to_string(), "sprint".to_string()],
            sentiment: Some("positive".to_string()),
            extraction_version: Some(1),
        };

        assert_eq!(ai.summary.as_deref(), Some("Meeting about Q1 planning"));
        assert_eq!(ai.topics.len(), 2);
        assert_eq!(ai.sentiment.as_deref(), Some("positive"));
        assert_eq!(ai.extraction_version, Some(1));
    }

    // =============================================================================
    // EventRelations Tests
    // =============================================================================

    #[test]
    fn test_event_relations_default() {
        let relations = EventRelations::default();
        assert!(relations.inferred_from.is_empty());
    }

    #[test]
    fn test_event_relations_with_data() {
        let relations = EventRelations {
            inferred_from: vec!["evt-previous-1".to_string(), "evt-previous-2".to_string()],
        };

        assert_eq!(relations.inferred_from.len(), 2);
    }

    // =============================================================================
    // GraphHints Tests
    // =============================================================================

    #[test]
    fn test_graph_hints_default() {
        let hints = GraphHints::default();
        assert!(hints.importance.is_none());
        assert!(!hints.recurrence);
    }

    #[test]
    fn test_graph_hints_full() {
        let hints = GraphHints {
            importance: Some(0.85),
            recurrence: true,
        };

        assert_eq!(hints.importance, Some(0.85));
        assert!(hints.recurrence);
    }

    // =============================================================================
    // EventEntities Tests
    // =============================================================================

    #[test]
    fn test_event_entities_default() {
        let entities = EventEntities::default();
        assert!(entities.is_empty());
        assert_eq!(entities.total_count(), 0);
    }

    #[test]
    fn test_event_entities_from_untyped() {
        let entities = EventEntities::from_untyped(vec!["ent-1".to_string(), "ent-2".to_string()]);
        assert_eq!(entities.total_count(), 2);
        assert_eq!(entities.get(EntityType::Concept), &["ent-1", "ent-2"]);
        assert!(entities.get(EntityType::Person).is_empty());
    }

    #[test]
    fn test_event_entities_add_entity() {
        let mut entities = EventEntities::default();
        entities.add_entity(EntityType::Person, "ent-person-alice");
        entities.add_entity(EntityType::Person, "ent-person-bob");
        entities.add_entity(EntityType::Project, "ent-project-x");

        assert_eq!(
            entities.get(EntityType::Person),
            &["ent-person-alice", "ent-person-bob"]
        );
        assert_eq!(entities.get(EntityType::Project), &["ent-project-x"]);
        assert_eq!(entities.total_count(), 3);
    }

    #[test]
    fn test_event_entities_get_empty() {
        let entities = EventEntities::default();
        assert!(entities.get(EntityType::Person).is_empty());
        assert!(entities.get(EntityType::Topic).is_empty());
    }

    #[test]
    fn test_event_entities_from_untyped_empty() {
        let entities = EventEntities::from_untyped(Vec::<String>::new());
        assert!(entities.is_empty());
    }

    // =============================================================================
    // RawRefs and DerivedRefs Tests
    // =============================================================================

    #[test]
    fn test_raw_refs_default() {
        let refs = RawRefs::default();
        assert!(refs.files.is_empty());
    }

    #[test]
    fn test_raw_refs_full() {
        let refs = RawRefs {
            files: vec![
                "/raw/photo-001.jpg".to_string(),
                "/raw/photo-002.jpg".to_string(),
            ],
        };

        assert_eq!(refs.files.len(), 2);
    }

    #[test]
    fn test_derived_refs_default() {
        let refs = DerivedRefs::default();
        assert!(refs.transcript.is_none());
        assert!(refs.embedding.is_none());
    }

    #[test]
    fn test_derived_refs_full() {
        let refs = DerivedRefs {
            transcript: Some("/derived/transcript-001.txt".to_string()),
            embedding: Some("/derived/embedding-001.bin".to_string()),
        };

        assert!(refs.transcript.is_some());
        assert!(refs.embedding.is_some());
    }

    // =============================================================================
    // Event Struct Tests
    // =============================================================================

    #[test]
    fn test_event_generate_id() {
        let id = Event::generate_id();
        assert!(id.starts_with("evt-"));
        // Format: evt-YYYYMMDD-HHMMSS-rrr (3 random hex chars)
        assert!(id.len() >= 20);
    }

    #[test]
    fn test_event_generate_id_uniqueness() {
        let id1 = Event::generate_id();
        let id2 = Event::generate_id();
        // IDs should be unique (though with tiny random portion, collision is possible)
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_event_full_construction() {
        let event = Event {
            schema: "event/v1".to_string(),
            id: "evt-test-001".to_string(),
            type_: "meeting".to_string(),
            subtype: Some("standup".to_string()),
            time: EventTime {
                start: Utc.with_ymd_and_hms(2026, 3, 31, 10, 0, 0).unwrap(),
                end: Some(Utc.with_ymd_and_hms(2026, 3, 31, 11, 0, 0).unwrap()),
                timezone: "UTC".to_string(),
            },
            created_at: Some(Utc.with_ymd_and_hms(2026, 3, 31, 9, 0, 0).unwrap()),
            ingested_at: Some(Utc.with_ymd_and_hms(2026, 3, 31, 12, 0, 0).unwrap()),
            source: EventSource {
                device: Some("macbook".to_string()),
                channel: None,
                capture_agent: None,
            },
            confidence: 0.9,
            entities: EventEntities::from_untyped(vec!["ent-person-alice".to_string()]),
            tags: vec!["work".to_string(), "team".to_string()],
            raw_refs: RawRefs {
                files: vec!["/raw/photo-001.jpg".to_string()],
            },
            derived_refs: DerivedRefs {
                transcript: Some("/derived/transcript.txt".to_string()),
                embedding: None,
            },
            ai: EventAi {
                summary: Some("Daily standup meeting".to_string()),
                extended: None,
                topics: vec!["planning".to_string()],
                sentiment: Some("neutral".to_string()),
                extraction_version: Some(1),
            },
            relations: EventRelations::default(),
            graph_hints: GraphHints {
                importance: Some(0.7),
                recurrence: true,
            },
            schema_version: 1,
        };

        assert_eq!(event.schema, "event/v1");
        assert_eq!(event.type_, "meeting");
        assert_eq!(event.subtype.as_deref(), Some("standup"));
        assert_eq!(event.confidence, 0.9);
        assert_eq!(event.tags.len(), 2);
        assert_eq!(event.entities.total_count(), 1);
        assert_eq!(event.ai.topics.len(), 1);
    }

    #[test]
    fn test_default_confidence() {
        assert_eq!(default_confidence(), 0.5);
    }

    #[test]
    fn test_default_schema_version() {
        assert_eq!(default_schema_version(), 1);
    }

    #[test]
    fn test_default_schema() {
        let schema = default_schema();
        assert_eq!(schema, "event/v1");
    }

    #[test]
    fn test_default_event_type() {
        assert_eq!(DEFAULT_EVENT_TYPE, "observation");
    }
}
