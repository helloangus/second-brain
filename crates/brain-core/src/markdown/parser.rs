//! Markdown parser
//!
//! Parses event and entity markdown files with YAML frontmatter into strongly-typed
//! Rust structs. The markdown format follows the Second Brain schema:
//!
//! ```markdown
//! ---
//! schema: event/v1
//! id: evt-20260331-001
//! type: meeting
//! time:
//!   start: 2026-03-31T10:00:00+09:00
//!   end: 2026-03-31T11:00:00+09:00
//!   timezone: Asia/Tokyo
//! ---
//! [optional body content]
//! ```
//!
//! ## Architecture
//!
//! 1. `EventParser` / `EntityParser` - entry points for parsing
//! 2. `extract_frontmatter()` - extracts YAML between `---` markers
//! 3. `serde_yaml::from_str()` - deserializes YAML to intermediate Parsed* structs
//! 4. `into_event()` / `into_entity()` - converts to final Event/Entity models

use crate::error::{Error, Result};
use crate::models::{
    default_confidence, default_entity_schema, default_schema, default_schema_version,
    default_timezone, DerivedRefs, Entity, EntityClassification, EntityEvolution, EntityIdentity,
    EntityLinks, EntityMetrics, EntityMultimedia, EntityStatus, EntityType, Event, EventAi,
    EventEntities, EventRelations, EventSource, EventTime, GraphHints, RawRefs, DEFAULT_EVENT_TYPE,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;

// ============================================================================
// Parser Entry Points
// ============================================================================

/// Parser for event markdown files.
///
/// # Example
///
/// ```
/// use brain_core::markdown::EventParser;
///
/// let content = r#"---
/// schema: event/v1
/// id: evt-20260331-001
/// type: meeting
/// time:
///   start: 2026-03-31T10:00:00+09:00
/// ---
/// "#;
///
/// let event = EventParser::parse(content).unwrap();
/// assert_eq!(event.id, "evt-20260331-001");
/// ```
pub struct EventParser;

impl EventParser {
    /// Parse an event from markdown content.
    ///
    /// Extracts YAML frontmatter, deserializes it, and converts to an Event struct.
    /// Returns error if frontmatter is missing, malformed, or deserialization fails.
    pub fn parse(content: &str) -> Result<Event> {
        let frontmatter = extract_frontmatter(content)?;

        let event: ParsedEventFrontmatter =
            serde_yaml::from_str(&frontmatter).map_err(|e| Error::MarkdownParse(e.to_string()))?;

        Ok(event.into_event())
    }
}

/// Parser for entity markdown files.
///
/// # Example
///
/// ```
/// use brain_core::markdown::EntityParser;
///
/// let content = r#"---
/// schema: entity/v1
/// id: ent-person-john
/// type: person
/// label: John Doe
/// ---
/// "#;
///
/// let entity = EntityParser::parse(content).unwrap();
/// assert_eq!(entity.id, "ent-person-john");
/// ```
pub struct EntityParser;

impl EntityParser {
    /// Parse an entity from markdown content.
    ///
    /// Extracts YAML frontmatter, deserializes it, and converts to an Entity struct.
    /// Returns error if frontmatter is missing, malformed, or deserialization fails.
    pub fn parse(content: &str) -> Result<Entity> {
        let frontmatter = extract_frontmatter(content)?;

        let entity: ParsedEntityFrontmatter =
            serde_yaml::from_str(&frontmatter).map_err(|e| Error::MarkdownParse(e.to_string()))?;

        Ok(entity.into_entity())
    }
}

// ============================================================================
// Frontmatter Extraction
// ============================================================================

/// Extract YAML frontmatter from markdown content.
///
/// Frontmatter is delimited by `---` markers at the start of the content.
/// Whitespace is trimmed from the content before parsing.
///
/// # Errors
///
/// Returns `Error::MarkdownParse` if:
/// - Content doesn't start with `---`
/// - Closing `---` marker is not found
///
/// # Arguments
///
/// * `content` - Raw markdown string with optional frontmatter
///
/// # Returns
///
/// The YAML string between the opening and closing `---` markers
fn extract_frontmatter(content: &str) -> Result<String> {
    let trimmed = content.trim();
    if !trimmed.starts_with("---") {
        return Err(Error::MarkdownParse("Missing frontmatter".to_string()));
    }

    let rest = &trimmed[3..];
    if let Some(end_pos) = rest.find("---") {
        Ok(rest[..end_pos].trim().to_string())
    } else {
        Err(Error::MarkdownParse("Unclosed frontmatter".to_string()))
    }
}

// ============================================================================
// Date Parsing Helpers
// ============================================================================

/// Parse an RFC3339 datetime string to UTC DateTime.
///
/// # Arguments
///
/// * `s` - RFC3339 formatted datetime string (e.g., "2026-03-31T10:00:00+09:00")
///
/// # Returns
///
/// `Some(DateTime<Utc>)` if parsing succeeds, `None` if the string is malformed
fn parse_rfc3339(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
}

/// Parse an RFC3339 datetime string, falling back to current time on parse failure.
///
/// Use this for required datetime fields where a fallback to "now" is acceptable.
///
/// # Arguments
///
/// * `s` - RFC3339 formatted datetime string
///
/// # Returns
///
/// Parsed datetime if valid, otherwise `Utc::now()`
fn parse_rfc3339_or_now(s: &str) -> DateTime<Utc> {
    parse_rfc3339(s).unwrap_or_else(Utc::now)
}

// ============================================================================
// Event Parsing Structs
// ============================================================================

/// Intermediate struct for parsing event frontmatter from YAML.
///
/// This struct mirrors the YAML schema with serde defaults applied.
/// After deserialization, it is converted to the final `Event` struct via `into_event()`.
#[derive(Debug, Deserialize, Clone)]
struct ParsedEventFrontmatter {
    /// Unique event identifier (e.g., "evt-20260331-001")
    id: String,
    /// Schema version, defaults to "event/v1"
    #[serde(default = "default_schema")]
    schema: String,
    /// Event type, defaults to "observation" (e.g., "meeting", "task", "photo")
    #[serde(default, alias = "type")]
    type_: Option<String>,
    /// Optional subtype for finer categorization
    #[serde(default)]
    subtype: Option<String>,
    /// Event time range (required)
    time: ParsedEventTime,
    /// When the event was first created
    #[serde(default)]
    created_at: Option<String>,
    /// When the event was ingested into the system
    #[serde(default)]
    ingested_at: Option<String>,
    /// Source information about how the event was captured
    #[serde(default)]
    source: Option<ParsedEventSource>,
    /// Confidence score, defaults to 0.5
    #[serde(default = "default_confidence")]
    confidence: f64,
    /// Linked entity references, keyed by entity type
    #[serde(default)]
    entities: Option<ParsedEventEntities>,
    /// User-defined tags
    #[serde(default)]
    tags: Option<Vec<String>>,
    /// References to raw data files
    #[serde(default)]
    raw_refs: Option<Vec<String>>,
    /// Derived data like transcripts or embeddings
    #[serde(default)]
    derived_refs: Option<ParsedDerivedRefs>,
    /// AI-generated analysis
    #[serde(default)]
    ai: Option<ParsedEventAi>,
    /// Relationship hints
    #[serde(default)]
    relations: Option<ParsedEventRelations>,
    /// Graph traversal hints
    #[serde(default)]
    graph_hints: Option<ParsedGraphHints>,
    /// Schema version integer, defaults to 1
    #[serde(default = "default_schema_version")]
    schema_version: i32,
}

/// Time range for an event, with start (required) and end (optional).
#[derive(Debug, Deserialize, Clone)]
struct ParsedEventTime {
    /// Start time in RFC3339 format (required)
    start: String,
    /// End time in RFC3339 format (optional)
    #[serde(default)]
    end: Option<String>,
    /// Timezone name, defaults to "UTC"
    #[serde(default = "default_timezone")]
    timezone: String,
}

/// Source information for event capture.
#[derive(Debug, Deserialize, Clone)]
struct ParsedEventSource {
    /// Device that captured the event
    #[serde(default)]
    device: Option<String>,
    /// Channel through which the event arrived
    #[serde(default)]
    channel: Option<String>,
    /// Software agent that captured the event
    #[serde(default)]
    capture_agent: Option<String>,
}

/// Entity references grouped by type, deserialized from snake_case YAML keys.
///
/// Maps 1:1 to the `EventEntities` BTreeMap structure via `From` implementation.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
struct ParsedEventEntities {
    #[serde(default)]
    people: Vec<String>,
    #[serde(default)]
    organizations: Vec<String>,
    #[serde(default)]
    projects: Vec<String>,
    #[serde(default)]
    artifacts: Vec<String>,
    #[serde(default)]
    concepts: Vec<String>,
    #[serde(default)]
    topics: Vec<String>,
    #[serde(default)]
    activities: Vec<String>,
    #[serde(default)]
    goals: Vec<String>,
    #[serde(default)]
    skills: Vec<String>,
    #[serde(default)]
    places: Vec<String>,
    #[serde(default)]
    devices: Vec<String>,
    #[serde(default)]
    resources: Vec<String>,
    #[serde(default)]
    memory_clusters: Vec<String>,
    #[serde(default)]
    states: Vec<String>,
}

impl From<ParsedEventEntities> for EventEntities {
    /// Converts parsed entity references to the EventEntities BTreeMap format.
    ///
    /// Only includes entity types that have at least one reference.
    /// The conversion is iterative rather than field-by-field for maintainability.
    fn from(parsed: ParsedEventEntities) -> Self {
        use std::collections::BTreeMap;
        let mut map = BTreeMap::new();

        // Array of (EntityType, references) pairs for iterative conversion.
        // Order doesn't matter since we use a BTreeMap for sorted keys.
        let fields: &[(EntityType, Vec<String>)] = &[
            (EntityType::Person, parsed.people),
            (EntityType::Organization, parsed.organizations),
            (EntityType::Project, parsed.projects),
            (EntityType::Artifact, parsed.artifacts),
            (EntityType::Concept, parsed.concepts),
            (EntityType::Topic, parsed.topics),
            (EntityType::Activity, parsed.activities),
            (EntityType::Goal, parsed.goals),
            (EntityType::Skill, parsed.skills),
            (EntityType::Place, parsed.places),
            (EntityType::Device, parsed.devices),
            (EntityType::Resource, parsed.resources),
            (EntityType::MemoryCluster, parsed.memory_clusters),
            (EntityType::State, parsed.states),
        ];

        for (etype, values) in fields {
            if !values.is_empty() {
                map.insert((*etype).clone(), values.clone());
            }
        }

        EventEntities(map)
    }
}

/// References to derived data extracted from raw content.
#[derive(Debug, Deserialize, Clone)]
struct ParsedDerivedRefs {
    /// Transcript text if audio/video was transcribed
    #[serde(default)]
    transcript: Option<String>,
    /// Embedding vector reference
    #[serde(default)]
    embedding: Option<String>,
}

/// AI-generated analysis of the event.
#[derive(Debug, Deserialize, Clone)]
struct ParsedEventAi {
    /// Brief summary of the event
    #[serde(default)]
    summary: Option<String>,
    /// Extended analysis
    #[serde(default)]
    extended: Option<String>,
    /// Topics identified by AI
    #[serde(default)]
    topics: Vec<String>,
    /// Sentiment analysis result
    #[serde(default)]
    sentiment: Option<String>,
    /// Version of the extraction model used
    #[serde(default)]
    extraction_version: Option<i32>,
}

/// Hints about relationships between events.
#[derive(Debug, Deserialize, Clone)]
struct ParsedEventRelations {
    /// Event IDs from which this event was inferred
    #[serde(default)]
    inferred_from: Vec<String>,
}

/// Hints for graph traversal algorithms.
#[derive(Debug, Deserialize, Clone)]
struct ParsedGraphHints {
    /// Importance score for ranking
    #[serde(default)]
    importance: Option<f64>,
    /// Whether this event recurs
    #[serde(default)]
    recurrence: Option<bool>,
}

impl ParsedEventFrontmatter {
    /// Converts parsed frontmatter to the final Event struct.
    ///
    /// Time fields are parsed from RFC3339 strings to DateTime<Utc>.
    /// Optional fields default to sensible empty values when not provided.
    fn into_event(self) -> Event {
        // Parse required start time, falling back to current time if malformed
        let time_start = parse_rfc3339_or_now(&self.time.start);

        // Parse optional end time, returning None if malformed
        let time_end = self.time.end.as_deref().and_then(parse_rfc3339);

        // Default event type if not specified
        let type_ = self.type_.unwrap_or_else(|| DEFAULT_EVENT_TYPE.to_string());

        // Convert entity references, defaulting to empty if not provided
        let entities = self.entities.map(EventEntities::from).unwrap_or_default();

        Event {
            schema: self.schema,
            id: self.id,
            type_,
            subtype: self.subtype.clone(),
            time: EventTime {
                start: time_start,
                end: time_end,
                timezone: self.time.timezone,
            },
            created_at: self.created_at.as_deref().and_then(parse_rfc3339),
            ingested_at: self.ingested_at.as_deref().and_then(parse_rfc3339),
            source: self
                .source
                .map(|s| EventSource {
                    device: s.device,
                    channel: s.channel,
                    capture_agent: s.capture_agent,
                })
                .unwrap_or_default(),
            confidence: self.confidence,
            entities,
            tags: self.tags.unwrap_or_default(),
            raw_refs: RawRefs {
                files: self.raw_refs.unwrap_or_default(),
            },
            derived_refs: self
                .derived_refs
                .map(|d| DerivedRefs {
                    transcript: d.transcript,
                    embedding: d.embedding,
                })
                .unwrap_or_default(),
            ai: self
                .ai
                .map(|a| EventAi {
                    summary: a.summary,
                    extended: a.extended,
                    topics: a.topics,
                    sentiment: a.sentiment,
                    extraction_version: a.extraction_version,
                })
                .unwrap_or_default(),
            relations: self
                .relations
                .map(|r| EventRelations {
                    inferred_from: r.inferred_from,
                })
                .unwrap_or_default(),
            graph_hints: self
                .graph_hints
                .map(|g| GraphHints {
                    importance: g.importance,
                    recurrence: g.recurrence.unwrap_or(false),
                })
                .unwrap_or_default(),
            schema_version: self.schema_version,
        }
    }
}

// ============================================================================
// Entity Parsing Structs
// ============================================================================

/// Intermediate struct for parsing entity frontmatter from YAML.
///
/// Entities represent persistent objects like people, places, projects,
/// and are linked from events by ID.
#[derive(Debug, Deserialize, Clone)]
struct ParsedEntityFrontmatter {
    /// Unique entity identifier (e.g., "ent-person-john")
    id: String,
    /// Schema version, defaults to "entity/v1"
    #[serde(default = "default_entity_schema")]
    schema: String,
    /// Entity type discriminator (e.g., "person", "organization")
    #[serde(default, alias = "type")]
    type_: Option<String>,
    /// Human-readable label
    #[serde(default)]
    label: Option<String>,
    /// Alternative names
    #[serde(default)]
    aliases: Vec<String>,
    /// Entity status
    #[serde(default)]
    status: Option<String>,
    /// Confidence score, defaults to 0.5
    #[serde(default = "default_confidence")]
    confidence: f64,
    /// Classification hierarchy
    #[serde(default)]
    classification: Option<ParsedEntityClassification>,
    /// Identity information
    #[serde(default)]
    identity: Option<ParsedEntityIdentity>,
    /// Multimedia attachments
    #[serde(default)]
    multimedia: Option<ParsedEntityMultimedia>,
    /// External links
    #[serde(default)]
    links: Option<ParsedEntityLinks>,
    /// Evolution history (merged/split)
    #[serde(default)]
    evolution: Option<ParsedEntityEvolution>,
    /// Usage metrics
    #[serde(default)]
    metrics: Option<ParsedEntityMetrics>,
    /// When the entity was created
    #[serde(default)]
    created_at: Option<String>,
    /// When the entity was last updated
    #[serde(default)]
    updated_at: Option<String>,
    /// Schema version integer, defaults to 1
    #[serde(default = "default_schema_version")]
    schema_version: i32,
}

/// Classification hierarchy for an entity.
#[derive(Debug, Deserialize, Clone)]
struct ParsedEntityClassification {
    /// Domain name (e.g., "academic", "business")
    #[serde(default)]
    domain: Option<String>,
    /// Parent entity IDs in the hierarchy
    #[serde(default)]
    parent: Vec<String>,
}

/// Identity information for an entity.
#[derive(Debug, Deserialize, Clone)]
struct ParsedEntityIdentity {
    /// Natural language description
    #[serde(default)]
    description: Option<String>,
    /// Short summary
    #[serde(default)]
    summary: Option<String>,
}

/// Multimedia attachments for an entity.
#[derive(Debug, Deserialize, Clone)]
struct ParsedEntityMultimedia {
    /// Image file references
    #[serde(default)]
    images: Vec<String>,
    /// Voice recording references
    #[serde(default)]
    voices: Vec<String>,
    /// Text embeddings
    #[serde(default)]
    embeddings: Option<ParsedEntityEmbeddings>,
}

/// Text embedding references.
#[derive(Debug, Deserialize, Clone)]
struct ParsedEntityEmbeddings {
    /// Text embedding reference
    #[serde(default)]
    text: Option<String>,
}

/// External link references.
#[derive(Debug, Deserialize, Clone)]
struct ParsedEntityLinks {
    /// Wikipedia URL
    #[serde(default)]
    wikipedia: Option<String>,
    /// Academic paper references
    #[serde(default)]
    papers: Vec<String>,
}

/// Evolution history for merged or split entities.
#[derive(Debug, Deserialize, Clone)]
struct ParsedEntityEvolution {
    /// Entity IDs this was merged from
    #[serde(default)]
    merged_from: Vec<String>,
    /// Entity IDs this was split to
    #[serde(default)]
    split_to: Vec<String>,
}

/// Usage metrics for an entity.
#[derive(Debug, Deserialize, Clone)]
struct ParsedEntityMetrics {
    /// Number of events referencing this entity
    #[serde(default)]
    event_count: Option<i32>,
    /// Last seen timestamp
    #[serde(default)]
    last_seen: Option<String>,
    /// Activity score for ranking
    #[serde(default)]
    activity_score: Option<f64>,
}

impl ParsedEntityFrontmatter {
    /// Converts parsed frontmatter to the final Entity struct.
    ///
    /// Type string is mapped to the EntityType enum. Unknown types default to Topic.
    /// Status string is mapped to EntityStatus enum. Unknown statuses default to Active.
    fn into_entity(self) -> Entity {
        // Map type string to EntityType enum
        let entity_type = self
            .type_
            .as_deref()
            .map(EntityType::from_singular)
            .unwrap_or_default();

        // Map status string to EntityStatus enum
        let status = match self.status.as_deref() {
            Some("archived") => EntityStatus::Archived,
            Some("merged") => EntityStatus::Merged,
            // Default to Active for unknown statuses
            _ => EntityStatus::Active,
        };

        // Label defaults to id if not provided
        let id = self.id.clone();
        let label = self.label.clone().unwrap_or_else(|| id.clone());

        Entity {
            schema: self.schema.clone(),
            id,
            type_: entity_type,
            label,
            aliases: self.aliases.clone(),
            status,
            confidence: self.confidence,
            classification: self
                .classification
                .clone()
                .map(|c| EntityClassification {
                    domain: c.domain,
                    parent: c.parent,
                })
                .unwrap_or_default(),
            identity: self
                .identity
                .clone()
                .map(|i| EntityIdentity {
                    description: i.description,
                    summary: i.summary,
                })
                .unwrap_or_default(),
            multimedia: self
                .multimedia
                .clone()
                .map(|m| EntityMultimedia {
                    images: m.images,
                    voices: m.voices,
                    embeddings_text: m.embeddings.and_then(|e| e.text),
                })
                .unwrap_or_default(),
            links: self
                .links
                .clone()
                .map(|l| EntityLinks {
                    wikipedia: l.wikipedia,
                    papers: l.papers,
                    custom: std::collections::HashMap::new(),
                })
                .unwrap_or_default(),
            evolution: self
                .evolution
                .clone()
                .map(|e| EntityEvolution {
                    merged_from: e.merged_from,
                    split_to: e.split_to,
                })
                .unwrap_or_default(),
            metrics: self
                .metrics
                .clone()
                .map(|m| EntityMetrics {
                    event_count: m.event_count.unwrap_or(0),
                    last_seen: m.last_seen.as_deref().and_then(parse_rfc3339),
                    activity_score: m.activity_score,
                })
                .unwrap_or_default(),
            created_at: self.created_at.as_deref().and_then(parse_rfc3339),
            updated_at: self.updated_at.as_deref().and_then(parse_rfc3339),
            schema_version: self.schema_version,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    // -------------------------------------------------------------------------
    // extract_frontmatter tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_extract_frontmatter_success() {
        let content = r#"---
key: value
another: 123
---
body"#;

        let result = extract_frontmatter(content);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "key: value\nanother: 123");
    }

    #[test]
    fn test_extract_frontmatter_with_whitespace() {
        let content = r#"
---
key: value
---
body"#;

        let result = extract_frontmatter(content);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "key: value");
    }

    #[test]
    fn test_extract_frontmatter_missing_start_marker() {
        let content = r#"key: value
---
body"#;

        let result = extract_frontmatter(content);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing frontmatter"));
    }

    #[test]
    fn test_extract_frontmatter_unclosed() {
        let content = r#"---
key: value
body"#;

        let result = extract_frontmatter(content);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unclosed frontmatter"));
    }

    #[test]
    fn test_extract_frontmatter_empty_content() {
        let result = extract_frontmatter("");
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // parse_rfc3339 tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_rfc3339_valid_with_timezone() {
        let result = parse_rfc3339("2026-03-31T10:00:00+09:00");
        assert!(result.is_some());
        let dt = result.unwrap();
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 3);
        assert_eq!(dt.day(), 31);
        assert_eq!(dt.hour(), 1); // UTC = 10:00 +09:00 - 9 hours = 01:00
    }

    #[test]
    fn test_parse_rfc3339_valid_zulu() {
        let result = parse_rfc3339("2026-03-31T10:00:00Z");
        assert!(result.is_some());
        let dt = result.unwrap();
        assert_eq!(dt.hour(), 10);
    }

    #[test]
    fn test_parse_rfc3339_invalid() {
        assert!(parse_rfc3339("not-a-date").is_none());
        assert!(parse_rfc3339("2026-13-45T99:99:99Z").is_none());
        assert!(parse_rfc3339("").is_none());
    }

    #[test]
    fn test_parse_rfc3339_or_now_valid() {
        let result = parse_rfc3339_or_now("2026-03-31T10:00:00Z");
        // Should return the parsed date, not now
        assert_eq!(result.year(), 2026);
        assert_eq!(result.month(), 3);
        assert_eq!(result.day(), 31);
    }

    #[test]
    fn test_parse_rfc3339_or_now_invalid() {
        let before = Utc::now();
        let result = parse_rfc3339_or_now("invalid");
        let after = Utc::now();
        // Should fall back to approximately now (within a few seconds)
        assert!(result >= before && result <= after);
        // Should be very recent
        assert!(result.timestamp() >= before.timestamp() - 1);
        assert!(result.timestamp() <= after.timestamp() + 1);
    }

    // -------------------------------------------------------------------------
    // EventParser tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_event_minimal() {
        let content = r#"---
schema: event/v1
id: evt-20260331-001
time:
  start: 2026-03-31T10:00:00+09:00
---
"#;
        let event = EventParser::parse(content).unwrap();
        assert_eq!(event.id, "evt-20260331-001");
        assert_eq!(event.type_, DEFAULT_EVENT_TYPE); // defaults to "observation"
        assert_eq!(event.schema, "event/v1");
        assert_eq!(event.confidence, 0.5);
        assert_eq!(event.schema_version, 1);
    }

    #[test]
    fn test_parse_event_full() {
        let content = r#"---
schema: event/v1
id: evt-20260331-001
type: meeting
subtype: standup
time:
  start: 2026-03-31T10:00:00+09:00
  end: 2026-03-31T11:00:00+09:00
  timezone: Asia/Tokyo
created_at: 2026-03-31T00:00:00Z
ingested_at: 2026-03-31T12:00:00Z
source:
  device: macbook
  channel: manual
  capture_agent: brain-cli
confidence: 0.9
tags:
  - work
  - team
entities:
  people:
    - ent-person-alice
    - ent-person-bob
  projects:
    - ent-project-brain
raw_refs:
  - /raw/photo-001.jpg
derived_refs:
  transcript: "Meeting about Q1 planning"
ai:
  summary: "Team standup"
  extended: "Discussed sprint progress"
  topics:
    - planning
    - sprint
  sentiment: positive
relations:
  inferred_from:
    - evt-20260330-001
graph_hints:
  importance: 0.8
  recurrence: true
---
"#;
        let event = EventParser::parse(content).unwrap();

        assert_eq!(event.id, "evt-20260331-001");
        assert_eq!(event.type_, "meeting");
        assert_eq!(event.subtype.as_deref(), Some("standup"));
        assert_eq!(event.time.timezone, "Asia/Tokyo");
        assert_eq!(event.confidence, 0.9);
        assert_eq!(event.tags, vec!["work", "team"]);

        // Check entity conversion
        let people = event.entities.get(EntityType::Person);
        assert_eq!(people, &vec!["ent-person-alice", "ent-person-bob"]);
        let projects = event.entities.get(EntityType::Project);
        assert_eq!(projects, &vec!["ent-project-brain"]);

        // Check source
        assert_eq!(event.source.device.as_deref(), Some("macbook"));
        assert_eq!(event.source.channel.as_deref(), Some("manual"));

        // Check AI
        assert_eq!(event.ai.summary.as_deref(), Some("Team standup"));
        assert_eq!(event.ai.topics, vec!["planning", "sprint"]);

        // Check relations
        assert_eq!(event.relations.inferred_from, vec!["evt-20260330-001"]);

        // Check graph hints
        assert_eq!(event.graph_hints.importance, Some(0.8));
        assert!(event.graph_hints.recurrence);
    }

    #[test]
    fn test_parse_event_missing_frontmatter() {
        let content = "no frontmatter here";
        let result = EventParser::parse(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_event_invalid_yaml() {
        let content = r#"---
schema: [invalid yaml
---
"#;
        let result = EventParser::parse(content);
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // EntityParser tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_entity_minimal() {
        let content = r#"---
schema: entity/v1
id: ent-person-john
type: person
label: John Doe
---
"#;
        let entity = EntityParser::parse(content).unwrap();
        assert_eq!(entity.id, "ent-person-john");
        assert_eq!(entity.label, "John Doe");
        assert!(matches!(entity.type_, EntityType::Person));
        assert!(matches!(entity.status, EntityStatus::Active));
        assert_eq!(entity.schema, "entity/v1");
    }

    #[test]
    fn test_parse_entity_all_types() {
        let types = vec![
            ("person", EntityType::Person),
            ("organization", EntityType::Organization),
            ("project", EntityType::Project),
            ("artifact", EntityType::Artifact),
            ("concept", EntityType::Concept),
            ("topic", EntityType::Topic),
            ("activity", EntityType::Activity),
            ("goal", EntityType::Goal),
            ("skill", EntityType::Skill),
            ("place", EntityType::Place),
            ("device", EntityType::Device),
            ("resource", EntityType::Resource),
            ("memory_cluster", EntityType::MemoryCluster),
            ("state", EntityType::State),
            ("unknown_type", EntityType::Topic), // unknown defaults to Topic
        ];

        for (type_str, expected) in types {
            let content = format!(
                r#"---
schema: entity/v1
id: ent-test
type: {}
---
"#,
                type_str
            );
            let entity = EntityParser::parse(&content).unwrap();
            assert_eq!(entity.type_, expected, "Failed for type: {}", type_str);
        }
    }

    #[test]
    fn test_parse_entity_all_statuses() {
        let content = r#"---
schema: entity/v1
id: ent-test
type: person
status: archived
---
"#;
        let entity = EntityParser::parse(content).unwrap();
        assert!(matches!(entity.status, EntityStatus::Archived));

        let content = r#"---
schema: entity/v1
id: ent-test
type: person
status: merged
---
"#;
        let entity = EntityParser::parse(content).unwrap();
        assert!(matches!(entity.status, EntityStatus::Merged));

        // Unknown status defaults to Active
        let content = r#"---
schema: entity/v1
id: ent-test
type: person
status: unknown
---
"#;
        let entity = EntityParser::parse(content).unwrap();
        assert!(matches!(entity.status, EntityStatus::Active));
    }

    #[test]
    fn test_parse_entity_full() {
        let content = r#"---
schema: entity/v1
id: ent-person-john
type: person
label: John Doe
aliases:
  - Johnny
  - John D
status: active
confidence: 0.95
classification:
  domain: technology
  parent:
    - ent-org-acme
identity:
  description: "Senior software engineer"
  summary: "Tech lead at Acme"
multimedia:
  images:
    - /img/john-profile.jpg
  voices:
    - /audio/john-intro.mp3
  embeddings:
    text: emb-001
links:
  wikipedia: https://en.wikipedia.org/wiki/John_Doe
  papers:
    - https://arxiv.org/abs/1234.5678
evolution:
  merged_from:
    - ent-old-john-1
    - ent-old-john-2
  split_to:
    - ent-john-work
metrics:
  event_count: 42
  last_seen: 2026-03-31T12:00:00Z
  activity_score: 0.85
created_at: 2025-01-01T00:00:00Z
updated_at: 2026-03-31T12:00:00Z
---
"#;

        let entity = EntityParser::parse(content).unwrap();

        assert_eq!(entity.id, "ent-person-john");
        assert_eq!(entity.label, "John Doe");
        assert_eq!(entity.aliases, vec!["Johnny", "John D"]);
        assert_eq!(entity.confidence, 0.95);

        // Classification
        assert_eq!(entity.classification.domain.as_deref(), Some("technology"));
        assert_eq!(entity.classification.parent, vec!["ent-org-acme"]);

        // Identity
        assert_eq!(
            entity.identity.description.as_deref(),
            Some("Senior software engineer")
        );

        // Multimedia
        assert_eq!(entity.multimedia.images, vec!["/img/john-profile.jpg"]);
        assert_eq!(entity.multimedia.voices, vec!["/audio/john-intro.mp3"]);
        assert_eq!(
            entity.multimedia.embeddings_text.as_deref(),
            Some("emb-001")
        );

        // Links
        assert_eq!(
            entity.links.wikipedia.as_deref(),
            Some("https://en.wikipedia.org/wiki/John_Doe")
        );
        assert_eq!(entity.links.papers.len(), 1);

        // Evolution
        assert_eq!(
            entity.evolution.merged_from,
            vec!["ent-old-john-1", "ent-old-john-2"]
        );
        assert_eq!(entity.evolution.split_to, vec!["ent-john-work"]);

        // Metrics
        assert_eq!(entity.metrics.event_count, 42);
        assert!(entity.metrics.last_seen.is_some());
        assert_eq!(entity.metrics.activity_score, Some(0.85));
    }

    #[test]
    fn test_parse_entity_label_defaults_to_id() {
        let content = r#"---
schema: entity/v1
id: ent-person-jane
type: person
---
"#;
        let entity = EntityParser::parse(content).unwrap();
        assert_eq!(entity.label, "ent-person-jane"); // defaults to id
    }

    #[test]
    fn test_parse_entity_metrics_defaults() {
        let content = r#"---
schema: entity/v1
id: ent-test
type: topic
---
"#;
        let entity = EntityParser::parse(content).unwrap();
        assert_eq!(entity.metrics.event_count, 0); // defaults to 0
        assert!(entity.metrics.last_seen.is_none());
        assert!(entity.metrics.activity_score.is_none());
    }

    // -------------------------------------------------------------------------
    // ParsedEventEntities conversion tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parsed_event_entities_all_types() {
        let parsed = ParsedEventEntities {
            people: vec!["p1".to_string(), "p2".to_string()],
            organizations: vec!["org1".to_string()],
            projects: vec![],
            artifacts: vec!["art1".to_string()],
            concepts: vec![],
            topics: vec!["t1".to_string()],
            activities: vec![],
            goals: vec![],
            skills: vec![],
            places: vec!["place1".to_string()],
            devices: vec![],
            resources: vec![],
            memory_clusters: vec![],
            states: vec![],
        };

        let entities: EventEntities = EventEntities::from(parsed);

        assert_eq!(entities.get(EntityType::Person), &vec!["p1", "p2"]);
        assert_eq!(entities.get(EntityType::Organization), &vec!["org1"]);
        assert_eq!(entities.get(EntityType::Project), &Vec::<String>::new()); // empty
        assert_eq!(entities.get(EntityType::Artifact), &vec!["art1"]);
        assert_eq!(entities.get(EntityType::Topic), &vec!["t1"]);
        assert_eq!(entities.get(EntityType::Place), &vec!["place1"]);
        // total_count = sum of all entity references = 2 + 1 + 1 + 1 + 1 = 6
        assert_eq!(entities.total_count(), 6);
    }

    #[test]
    fn test_parsed_event_entities_empty() {
        let parsed = ParsedEventEntities {
            people: vec![],
            organizations: vec![],
            projects: vec![],
            artifacts: vec![],
            concepts: vec![],
            topics: vec![],
            activities: vec![],
            goals: vec![],
            skills: vec![],
            places: vec![],
            devices: vec![],
            resources: vec![],
            memory_clusters: vec![],
            states: vec![],
        };

        let entities: EventEntities = EventEntities::from(parsed);
        assert!(entities.is_empty());
    }

    // -------------------------------------------------------------------------
    // Edge case tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_event_invalid_time_uses_now() {
        let content = r#"---
schema: event/v1
id: evt-test
time:
  start: not-a-time
---
"#;
        let event = EventParser::parse(content).unwrap();
        // Should use Utc::now() as fallback
        let now = Utc::now();
        assert_eq!(event.time.start.year(), now.year());
        assert_eq!(event.time.start.month(), now.month());
        assert_eq!(event.time.start.day(), now.day());
    }

    #[test]
    fn test_parse_event_type_alias() {
        // YAML uses "type" which is a Rust keyword, serde alias handles it
        let content = r#"---
schema: event/v1
id: evt-test
type: task
time:
  start: 2026-03-31T10:00:00Z
---
"#;
        let event = EventParser::parse(content).unwrap();
        assert_eq!(event.type_, "task");
    }

    #[test]
    fn test_parse_entity_type_alias() {
        let content = r#"---
schema: entity/v1
id: ent-test
type: place
label: Test
---
"#;
        let entity = EntityParser::parse(content).unwrap();
        assert_eq!(entity.type_, EntityType::Place);
    }
}
