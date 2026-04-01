//! Markdown parser

use crate::error::{Error, Result};
use crate::models::*;
use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Parser for event markdown files
pub struct EventParser;

impl EventParser {
    /// Parse an event from markdown content
    pub fn parse(content: &str) -> Result<Event> {
        // Extract frontmatter between --- markers
        let frontmatter = extract_frontmatter(content)?;

        let event: ParsedEventFrontmatter =
            serde_yaml::from_str(&frontmatter).map_err(|e| Error::MarkdownParse(e.to_string()))?;

        Ok(event.into_event())
    }
}

/// Parser for entity markdown files
pub struct EntityParser;

impl EntityParser {
    /// Parse an entity from markdown content
    pub fn parse(content: &str) -> Result<Entity> {
        // Extract frontmatter between --- markers
        let frontmatter = extract_frontmatter(content)?;

        let entity: ParsedEntityFrontmatter =
            serde_yaml::from_str(&frontmatter).map_err(|e| Error::MarkdownParse(e.to_string()))?;

        Ok(entity.into_entity())
    }
}

/// Extract YAML frontmatter from markdown content
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

/// Intermediate struct for parsing event frontmatter
#[derive(Debug, Deserialize, Clone)]
struct ParsedEventFrontmatter {
    id: String,
    #[serde(default = "default_schema")]
    schema: String,
    #[serde(default, alias = "type")]
    type_: Option<String>,
    #[serde(default)]
    subtype: Option<String>,
    time: ParsedEventTime,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    ingested_at: Option<String>,
    #[serde(default)]
    source: Option<ParsedEventSource>,
    #[serde(default = "default_confidence")]
    confidence: f64,
    #[serde(default)]
    entities: Option<ParsedEventEntities>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    raw_refs: Option<Vec<String>>,
    #[serde(default)]
    derived_refs: Option<ParsedDerivedRefs>,
    #[serde(default)]
    ai: Option<ParsedEventAi>,
    #[serde(default)]
    relations: Option<ParsedEventRelations>,
    #[serde(default)]
    graph_hints: Option<ParsedGraphHints>,
    #[serde(default = "default_version")]
    schema_version: i32,
}

fn default_schema() -> String {
    "event/v1".to_string()
}

fn default_confidence() -> f64 {
    0.5
}

fn default_version() -> i32 {
    1
}

#[derive(Debug, Deserialize, Clone)]
struct ParsedEventTime {
    start: String,
    #[serde(default)]
    end: Option<String>,
    #[serde(default = "default_timezone")]
    timezone: String,
}

fn default_timezone() -> String {
    "UTC".to_string()
}

#[derive(Debug, Deserialize, Clone)]
struct ParsedEventSource {
    #[serde(default)]
    device: Option<String>,
    #[serde(default)]
    channel: Option<String>,
    #[serde(default)]
    capture_agent: Option<String>,
}

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

#[derive(Debug, Deserialize, Clone)]
struct ParsedDerivedRefs {
    #[serde(default)]
    transcript: Option<String>,
    #[serde(default)]
    embedding: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct ParsedEventAi {
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    topics: Vec<String>,
    #[serde(default)]
    sentiment: Option<String>,
    #[serde(default)]
    extraction_version: Option<i32>,
}

#[derive(Debug, Deserialize, Clone)]
struct ParsedEventRelations {
    #[serde(default)]
    inferred_from: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct ParsedGraphHints {
    #[serde(default)]
    importance: Option<f64>,
    #[serde(default)]
    recurrence: Option<bool>,
}

impl ParsedEventFrontmatter {
    fn into_event(self) -> Event {
        let time_start = DateTime::parse_from_rfc3339(&self.time.start)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let time_end = self.time.end.as_ref().and_then(|s| {
            DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
        });

        let event_type = match self.type_.as_deref() {
            Some("meeting") => EventType::Meeting,
            Some("photo") => EventType::Photo,
            Some("note") => EventType::Note,
            Some("activity") => EventType::Activity,
            Some("research") => EventType::Research,
            Some("reading") => EventType::Reading,
            Some("exercise") => EventType::Exercise,
            Some("meal") => EventType::Meal,
            Some("work") => EventType::Work,
            _ => EventType::Other,
        };

        let entities = self.entities.unwrap_or_else(|| ParsedEventEntities {
            people: Vec::new(),
            organizations: Vec::new(),
            projects: Vec::new(),
            artifacts: Vec::new(),
            concepts: Vec::new(),
            topics: Vec::new(),
            activities: Vec::new(),
            goals: Vec::new(),
            skills: Vec::new(),
            places: Vec::new(),
            devices: Vec::new(),
            resources: Vec::new(),
            memory_clusters: Vec::new(),
            states: Vec::new(),
        });

        Event {
            schema: self.schema,
            id: self.id,
            type_: event_type,
            subtype: self.subtype.clone(),
            time: EventTime {
                start: time_start,
                end: time_end,
                timezone: self.time.timezone,
            },
            created_at: self.created_at.as_ref().and_then(|s| {
                DateTime::parse_from_rfc3339(s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            }),
            ingested_at: self.ingested_at.as_ref().and_then(|s| {
                DateTime::parse_from_rfc3339(s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            }),
            source: self
                .source
                .map(|s| EventSource {
                    device: s.device,
                    channel: s.channel,
                    capture_agent: s.capture_agent,
                })
                .unwrap_or_default(),
            confidence: self.confidence,
            entities: EventEntities {
                people: entities.people,
                organizations: entities.organizations,
                projects: entities.projects,
                artifacts: entities.artifacts,
                concepts: entities.concepts,
                topics: entities.topics,
                activities: entities.activities,
                goals: entities.goals,
                skills: entities.skills,
                places: entities.places,
                devices: entities.devices,
                resources: entities.resources,
                memory_clusters: entities.memory_clusters,
                states: entities.states,
            },
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

/// Intermediate struct for parsing entity frontmatter
#[derive(Debug, Deserialize, Clone)]
struct ParsedEntityFrontmatter {
    id: String,
    #[serde(default = "default_entity_schema")]
    schema: String,
    #[serde(default, alias = "type")]
    type_: Option<String>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default = "default_confidence")]
    confidence: f64,
    #[serde(default)]
    classification: Option<ParsedEntityClassification>,
    #[serde(default)]
    identity: Option<ParsedEntityIdentity>,
    #[serde(default)]
    multimedia: Option<ParsedEntityMultimedia>,
    #[serde(default)]
    links: Option<ParsedEntityLinks>,
    #[serde(default)]
    evolution: Option<ParsedEntityEvolution>,
    #[serde(default)]
    metrics: Option<ParsedEntityMetrics>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default = "default_version")]
    schema_version: i32,
}

fn default_entity_schema() -> String {
    "entity/v1".to_string()
}

#[derive(Debug, Deserialize, Clone)]
struct ParsedEntityClassification {
    #[serde(default)]
    domain: Option<String>,
    #[serde(default)]
    parent: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct ParsedEntityIdentity {
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    summary: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct ParsedEntityMultimedia {
    #[serde(default)]
    images: Vec<String>,
    #[serde(default)]
    voices: Vec<String>,
    #[serde(default)]
    embeddings: Option<ParsedEntityEmbeddings>,
}

#[derive(Debug, Deserialize, Clone)]
struct ParsedEntityEmbeddings {
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct ParsedEntityLinks {
    #[serde(default)]
    wikipedia: Option<String>,
    #[serde(default)]
    papers: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct ParsedEntityEvolution {
    #[serde(default)]
    merged_from: Vec<String>,
    #[serde(default)]
    split_to: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct ParsedEntityMetrics {
    #[serde(default)]
    event_count: Option<i32>,
    #[serde(default)]
    last_seen: Option<String>,
    #[serde(default)]
    activity_score: Option<f64>,
}

impl ParsedEntityFrontmatter {
    fn into_entity(self) -> Entity {
        let entity_type = match self.type_.as_deref() {
            Some("person") => EntityType::Person,
            Some("organization") => EntityType::Organization,
            Some("project") => EntityType::Project,
            Some("artifact") => EntityType::Artifact,
            Some("concept") => EntityType::Concept,
            Some("topic") => EntityType::Topic,
            Some("activity") => EntityType::Activity,
            Some("goal") => EntityType::Goal,
            Some("skill") => EntityType::Skill,
            Some("place") => EntityType::Place,
            Some("device") => EntityType::Device,
            Some("resource") => EntityType::Resource,
            Some("memory_cluster") => EntityType::MemoryCluster,
            Some("state") => EntityType::State,
            _ => EntityType::Topic,
        };

        let status = match self.status.as_deref() {
            Some("archived") => EntityStatus::Archived,
            Some("merged") => EntityStatus::Merged,
            _ => EntityStatus::Active,
        };

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
                    last_seen: m.last_seen.and_then(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .map(|dt| dt.with_timezone(&Utc))
                            .ok()
                    }),
                    activity_score: m.activity_score,
                })
                .unwrap_or_default(),
            created_at: self.created_at.as_ref().and_then(|s| {
                DateTime::parse_from_rfc3339(s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            }),
            updated_at: self.updated_at.as_ref().and_then(|s| {
                DateTime::parse_from_rfc3339(s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            }),
            schema_version: self.schema_version,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_event() {
        let content = r#"---
schema: event/v1
id: evt-20260331-001
type: meeting
time:
  start: 2026-03-31T10:00:00+09:00
  end: 2026-03-31T11:00:00+09:00
  timezone: Asia/Tokyo
status: manual
confidence: 0.9
---
"#;
        let event = EventParser::parse(content).unwrap();
        assert_eq!(event.id, "evt-20260331-001");
        assert_eq!(event.type_, EventType::Meeting);
    }
}
