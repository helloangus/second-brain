//! Markdown serializer

use crate::error::Result;
use crate::models::*;

/// Serializer for event markdown files
pub struct EventSerializer;

impl EventSerializer {
    /// Serialize an event to markdown string
    pub fn serialize(&self, event: &Event) -> Result<String> {
        let mut yaml = String::new();

        // Frontmatter header
        yaml.push_str("---\n");
        yaml.push_str(&format!("schema: {}\n", event.schema));
        yaml.push_str(&format!("id: {}\n", event.id));

        // Type
        yaml.push_str(&format!("type: {}\n", event.type_));
        if let Some(ref subtype) = event.subtype {
            yaml.push_str(&format!("subtype: {}\n", subtype));
        }

        // Time
        yaml.push_str("time:\n");
        yaml.push_str(&format!("  start: {}\n", event.time.start.to_rfc3339()));
        if let Some(ref end) = event.time.end {
            yaml.push_str(&format!("  end: {}\n", end.to_rfc3339()));
        }
        yaml.push_str(&format!("  timezone: {}\n", event.time.timezone));

        // Timestamps
        if let Some(ref created_at) = event.created_at {
            yaml.push_str(&format!("created_at: {}\n", created_at.to_rfc3339()));
        }
        if let Some(ref ingested_at) = event.ingested_at {
            yaml.push_str(&format!("ingested_at: {}\n", ingested_at.to_rfc3339()));
        }

        // Source
        if event.source.device.is_some()
            || event.source.channel.is_some()
            || event.source.capture_agent.is_some()
        {
            yaml.push_str("source:\n");
            if let Some(ref device) = event.source.device {
                yaml.push_str(&format!("  device: {}\n", device));
            }
            if let Some(ref channel) = event.source.channel {
                yaml.push_str(&format!("  channel: {}\n", channel));
            }
            if let Some(ref agent) = event.source.capture_agent {
                yaml.push_str(&format!("  capture_agent: {}\n", agent));
            }
        }

        // Confidence
        yaml.push_str(&format!("confidence: {}\n", event.confidence));

        // Entities
        if !event.entities.is_empty() {
            yaml.push_str("entities:\n");
            self.serialize_entities(&event.entities, &mut yaml);
        }

        // Tags
        if !event.tags.is_empty() {
            yaml.push_str("tags:\n");
            for tag in &event.tags {
                yaml.push_str(&format!("  - {}\n", tag));
            }
        }

        // Raw refs
        if !event.raw_refs.files.is_empty() {
            yaml.push_str("raw_refs:\n");
            for r#ref in &event.raw_refs.files {
                yaml.push_str(&format!("  - {}\n", r#ref));
            }
        }

        // Derived refs
        if event.derived_refs.transcript.is_some() || event.derived_refs.embedding.is_some() {
            yaml.push_str("derived_refs:\n");
            if let Some(ref transcript) = event.derived_refs.transcript {
                yaml.push_str(&format!("  transcript: {}\n", transcript));
            }
            if let Some(ref embedding) = event.derived_refs.embedding {
                yaml.push_str(&format!("  embedding: {}\n", embedding));
            }
        }

        // AI
        if event.ai.summary.is_some() || !event.ai.topics.is_empty() || event.ai.sentiment.is_some()
        {
            yaml.push_str("ai:\n");
            if let Some(ref summary) = event.ai.summary {
                yaml.push_str("  summary: >\n");
                for line in summary.lines() {
                    yaml.push_str(&format!("    {}\n", line));
                }
            }
            if !event.ai.topics.is_empty() {
                yaml.push_str("  topics:\n");
                for topic in &event.ai.topics {
                    yaml.push_str(&format!("    - {}\n", topic));
                }
            }
            if let Some(ref sentiment) = event.ai.sentiment {
                yaml.push_str(&format!("  sentiment: {}\n", sentiment));
            }
            if let Some(ref version) = event.ai.extraction_version {
                yaml.push_str(&format!("  extraction_version: {}\n", version));
            }
        }

        // Relations
        if !event.relations.inferred_from.is_empty() {
            yaml.push_str("relations:\n");
            yaml.push_str("  inferred_from:\n");
            for src in &event.relations.inferred_from {
                yaml.push_str(&format!("    - {}\n", src));
            }
        }

        // Graph hints
        yaml.push_str("graph_hints:\n");
        if let Some(importance) = event.graph_hints.importance {
            yaml.push_str(&format!("  importance: {}\n", importance));
        }
        yaml.push_str(&format!("  recurrence: {}\n", event.graph_hints.recurrence));

        // Schema version
        yaml.push_str(&format!("schema_version: {}\n", event.schema_version));

        yaml.push_str("---\n");

        Ok(yaml)
    }

    fn serialize_entities(&self, entities: &EventEntities, yaml: &mut String) {
        if !entities.people.is_empty() {
            yaml.push_str("  people:\n");
            for e in &entities.people {
                yaml.push_str(&format!("    - {}\n", e));
            }
        }
        if !entities.organizations.is_empty() {
            yaml.push_str("  organizations:\n");
            for e in &entities.organizations {
                yaml.push_str(&format!("    - {}\n", e));
            }
        }
        if !entities.projects.is_empty() {
            yaml.push_str("  projects:\n");
            for e in &entities.projects {
                yaml.push_str(&format!("    - {}\n", e));
            }
        }
        if !entities.artifacts.is_empty() {
            yaml.push_str("  artifacts:\n");
            for e in &entities.artifacts {
                yaml.push_str(&format!("    - {}\n", e));
            }
        }
        if !entities.concepts.is_empty() {
            yaml.push_str("  concepts:\n");
            for e in &entities.concepts {
                yaml.push_str(&format!("    - {}\n", e));
            }
        }
        if !entities.topics.is_empty() {
            yaml.push_str("  topics:\n");
            for e in &entities.topics {
                yaml.push_str(&format!("    - {}\n", e));
            }
        }
        if !entities.activities.is_empty() {
            yaml.push_str("  activities:\n");
            for e in &entities.activities {
                yaml.push_str(&format!("    - {}\n", e));
            }
        }
        if !entities.goals.is_empty() {
            yaml.push_str("  goals:\n");
            for e in &entities.goals {
                yaml.push_str(&format!("    - {}\n", e));
            }
        }
        if !entities.skills.is_empty() {
            yaml.push_str("  skills:\n");
            for e in &entities.skills {
                yaml.push_str(&format!("    - {}\n", e));
            }
        }
        if !entities.places.is_empty() {
            yaml.push_str("  places:\n");
            for e in &entities.places {
                yaml.push_str(&format!("    - {}\n", e));
            }
        }
        if !entities.devices.is_empty() {
            yaml.push_str("  devices:\n");
            for e in &entities.devices {
                yaml.push_str(&format!("    - {}\n", e));
            }
        }
        if !entities.resources.is_empty() {
            yaml.push_str("  resources:\n");
            for e in &entities.resources {
                yaml.push_str(&format!("    - {}\n", e));
            }
        }
        if !entities.memory_clusters.is_empty() {
            yaml.push_str("  memory_clusters:\n");
            for e in &entities.memory_clusters {
                yaml.push_str(&format!("    - {}\n", e));
            }
        }
        if !entities.states.is_empty() {
            yaml.push_str("  states:\n");
            for e in &entities.states {
                yaml.push_str(&format!("    - {}\n", e));
            }
        }
    }
}

/// Serializer for entity markdown files
pub struct EntitySerializer;

impl EntitySerializer {
    /// Serialize an entity to markdown string
    pub fn serialize(&self, entity: &Entity) -> Result<String> {
        let mut yaml = String::new();

        yaml.push_str("---\n");
        yaml.push_str(&format!("schema: {}\n", entity.schema));
        yaml.push_str(&format!("id: {}\n", entity.id));
        yaml.push_str(&format!("type: {}\n", entity.type_));
        yaml.push_str(&format!("label: {}\n", entity.label));

        if !entity.aliases.is_empty() {
            yaml.push_str("aliases:\n");
            for alias in &entity.aliases {
                yaml.push_str(&format!("  - {}\n", alias));
            }
        }

        yaml.push_str(&format!(
            "status: {}\n",
            match entity.status {
                EntityStatus::Active => "active",
                EntityStatus::Archived => "archived",
                EntityStatus::Merged => "merged",
            }
        ));
        yaml.push_str(&format!("confidence: {}\n", entity.confidence));

        // Classification
        yaml.push_str("classification:\n");
        if let Some(ref domain) = entity.classification.domain {
            yaml.push_str(&format!("  domain: {}\n", domain));
        }
        if !entity.classification.parent.is_empty() {
            yaml.push_str("  parent:\n");
            for p in &entity.classification.parent {
                yaml.push_str(&format!("    - {}\n", p));
            }
        }

        // Identity
        yaml.push_str("identity:\n");
        if let Some(ref desc) = entity.identity.description {
            yaml.push_str("  description: >\n");
            for line in desc.lines() {
                yaml.push_str(&format!("    {}\n", line));
            }
        }
        if let Some(ref summary) = entity.identity.summary {
            yaml.push_str("  summary: >\n");
            for line in summary.lines() {
                yaml.push_str(&format!("    {}\n", line));
            }
        }

        // Multimedia
        yaml.push_str("multimedia:\n");
        if !entity.multimedia.images.is_empty() {
            yaml.push_str("  images:\n");
            for img in &entity.multimedia.images {
                yaml.push_str(&format!("    - {}\n", img));
            }
        }
        if !entity.multimedia.voices.is_empty() {
            yaml.push_str("  voices:\n");
            for voice in &entity.multimedia.voices {
                yaml.push_str(&format!("    - {}\n", voice));
            }
        }
        if let Some(ref emb) = entity.multimedia.embeddings_text {
            yaml.push_str("  embeddings:\n");
            yaml.push_str(&format!("    text: {}\n", emb));
        }

        // Links
        yaml.push_str("links:\n");
        if let Some(ref wiki) = entity.links.wikipedia {
            yaml.push_str(&format!("  wikipedia: {}\n", wiki));
        }
        if !entity.links.papers.is_empty() {
            yaml.push_str("  papers:\n");
            for paper in &entity.links.papers {
                yaml.push_str(&format!("    - {}\n", paper));
            }
        }

        // Evolution
        if !entity.evolution.merged_from.is_empty() || !entity.evolution.split_to.is_empty() {
            yaml.push_str("evolution:\n");
            if !entity.evolution.merged_from.is_empty() {
                yaml.push_str("  merged_from:\n");
                for m in &entity.evolution.merged_from {
                    yaml.push_str(&format!("    - {}\n", m));
                }
            }
            if !entity.evolution.split_to.is_empty() {
                yaml.push_str("  split_to:\n");
                for s in &entity.evolution.split_to {
                    yaml.push_str(&format!("    - {}\n", s));
                }
            }
        }

        // Metrics
        yaml.push_str("metrics:\n");
        yaml.push_str(&format!("  event_count: {}\n", entity.metrics.event_count));
        if let Some(ref last_seen) = entity.metrics.last_seen {
            yaml.push_str(&format!("  last_seen: {}\n", last_seen.to_rfc3339()));
        }
        if let Some(ref score) = entity.metrics.activity_score {
            yaml.push_str(&format!("  activity_score: {}\n", score));
        }

        // Timestamps
        if let Some(ref created_at) = entity.created_at {
            yaml.push_str(&format!("created_at: {}\n", created_at.to_rfc3339()));
        }
        if let Some(ref updated_at) = entity.updated_at {
            yaml.push_str(&format!("updated_at: {}\n", updated_at.to_rfc3339()));
        }

        yaml.push_str(&format!("schema_version: {}\n", entity.schema_version));
        yaml.push_str("---\n");

        Ok(yaml)
    }
}

impl EventEntities {
    fn is_empty(&self) -> bool {
        self.people.is_empty()
            && self.organizations.is_empty()
            && self.projects.is_empty()
            && self.artifacts.is_empty()
            && self.concepts.is_empty()
            && self.topics.is_empty()
            && self.activities.is_empty()
            && self.goals.is_empty()
            && self.skills.is_empty()
            && self.places.is_empty()
            && self.devices.is_empty()
            && self.resources.is_empty()
            && self.memory_clusters.is_empty()
            && self.states.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn test_serialize_event() {
        let event = Event {
            schema: "event/v1".to_string(),
            id: "evt-20260331-001".to_string(),
            type_: EventType::Meeting,
            subtype: Some("research".to_string()),
            time: EventTime {
                start: Utc.with_ymd_and_hms(2026, 3, 31, 10, 0, 0).unwrap(),
                end: Some(Utc.with_ymd_and_hms(2026, 3, 31, 11, 0, 0).unwrap()),
                timezone: "Asia/Tokyo".to_string(),
            },
            created_at: None,
            ingested_at: None,
            source: EventSource::default(),
            confidence: 0.9,
            entities: EventEntities::default(),
            tags: vec!["research".to_string(), "gpu".to_string()],
            raw_refs: RawRefs::default(),
            derived_refs: DerivedRefs::default(),
            ai: EventAi::default(),
            relations: EventRelations::default(),
            graph_hints: GraphHints::default(),
            schema_version: 1,
        };

        let yaml = EventSerializer.serialize(&event).unwrap();
        assert!(yaml.contains("id: evt-20260331-001"));
        assert!(yaml.contains("type: meeting"));
    }
}
