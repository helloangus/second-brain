//! Entity model
//!
//! Represents long-lived objects in the Second Brain system such as people,
//! organizations, projects, and concepts. Entities are stored as Markdown files
//! and linked from events by their ID.

use crate::models::event::{default_confidence, default_schema_version};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Entity type enum (extended set)
///
/// Represents the category of an entity. Each type maps to a specific domain
/// of knowledge representation in the cognitive system.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord, Default)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    /// A person individual
    Person,
    Organization,
    Project,
    /// A created artifact or deliverable
    Artifact,
    Concept,
    #[default]
    Topic,
    Activity,
    Goal,
    Skill,
    Place,
    Device,
    Resource,
    /// A cluster of related memories
    MemoryCluster,
    State,
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityType::Person => write!(f, "person"),
            EntityType::Organization => write!(f, "organization"),
            EntityType::Project => write!(f, "project"),
            EntityType::Artifact => write!(f, "artifact"),
            EntityType::Concept => write!(f, "concept"),
            EntityType::Topic => write!(f, "topic"),
            EntityType::Activity => write!(f, "activity"),
            EntityType::Goal => write!(f, "goal"),
            EntityType::Skill => write!(f, "skill"),
            EntityType::Place => write!(f, "place"),
            EntityType::Device => write!(f, "device"),
            EntityType::Resource => write!(f, "resource"),
            EntityType::MemoryCluster => write!(f, "memory_cluster"),
            EntityType::State => write!(f, "state"),
        }
    }
}

/// Entity status
///
/// Tracks the lifecycle state of an entity.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EntityStatus {
    /// Entity is actively being used and referenced
    #[default]
    Active,
    /// Entity has been archived and is no longer in active use
    Archived,
    /// Entity has been merged into another entity
    Merged,
}

impl std::fmt::Display for EntityStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityStatus::Active => write!(f, "active"),
            EntityStatus::Archived => write!(f, "archived"),
            EntityStatus::Merged => write!(f, "merged"),
        }
    }
}

/// Links to external resources
///
/// Provides references to external information sources such as Wikipedia
/// articles, academic papers, or custom URLs.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct EntityLinks {
    /// Wikipedia article URL for this entity
    pub wikipedia: Option<String>,
    /// List of academic paper URLs or identifiers
    #[serde(default)]
    pub papers: Vec<String>,
    /// Custom key-value pairs for additional external references
    #[serde(default)]
    pub custom: std::collections::HashMap<String, String>,
}

/// Classification information
///
/// Provides hierarchical classification of the entity within a domain taxonomy.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct EntityClassification {
    /// The high-level domain this entity belongs to (e.g., "technology", "science")
    pub domain: Option<String>,
    /// Parent entity IDs in the classification hierarchy
    #[serde(default)]
    pub parent: Vec<String>,
}

/// Identity description
///
/// Textual identity information for the entity.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct EntityIdentity {
    /// Detailed description of the entity
    pub description: Option<String>,
    /// Short summary of the entity's identity
    pub summary: Option<String>,
}

/// Multimedia references
///
/// Stores references to multimedia assets associated with the entity.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct EntityMultimedia {
    /// Paths or URLs to image files
    #[serde(default)]
    pub images: Vec<String>,
    /// Paths or URLs to voice/audio files
    #[serde(default)]
    pub voices: Vec<String>,
    /// Text embedding identifier for semantic search
    pub embeddings_text: Option<String>,
}

/// Evolution tracking
///
/// Tracks how this entity has evolved through merges and splits.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct EntityEvolution {
    /// IDs of entities that were merged into this entity
    #[serde(default)]
    pub merged_from: Vec<String>,
    /// IDs of entities this entity was split into
    #[serde(default)]
    pub split_to: Vec<String>,
}

/// Usage metrics
///
/// Tracks usage statistics for this entity.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct EntityMetrics {
    /// Number of events referencing this entity
    #[serde(default)]
    pub event_count: i32,
    /// Timestamp of the most recent event referencing this entity
    pub last_seen: Option<DateTime<Utc>>,
    /// Activity score calculated by the pipeline (0.0 - 1.0)
    pub activity_score: Option<f64>,
}

/// Full Entity struct
///
/// Represents a long-lived object in the Second Brain system.
/// Entities are stored as Markdown files with YAML frontmatter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Schema identifier for serialization version control (e.g., "entity/v1")
    pub schema: String,
    /// Unique identifier for this entity (e.g., "ent-person-john")
    pub id: String,
    /// The type/category of this entity
    pub type_: EntityType,
    /// Human-readable display name
    pub label: String,
    /// Alternative names or aliases for this entity
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Current lifecycle status
    #[serde(default)]
    pub status: EntityStatus,
    /// Confidence score of the AI extraction (0.0 - 1.0)
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    /// Classification hierarchy information
    #[serde(default)]
    pub classification: EntityClassification,
    /// Textual identity information
    #[serde(default)]
    pub identity: EntityIdentity,
    /// Multimedia references
    #[serde(default)]
    pub multimedia: EntityMultimedia,
    /// External resource links
    #[serde(default)]
    pub links: EntityLinks,
    /// Evolution history (merges and splits)
    #[serde(default)]
    pub evolution: EntityEvolution,
    /// Usage metrics
    #[serde(default)]
    pub metrics: EntityMetrics,
    /// Creation timestamp
    pub created_at: Option<DateTime<Utc>>,
    /// Last update timestamp
    pub updated_at: Option<DateTime<Utc>>,
    /// Schema version for this entity
    #[serde(default = "default_schema_version")]
    pub schema_version: i32,
}

/// Returns the default schema string for entities
pub fn default_entity_schema() -> String {
    format!("entity/v{}", crate::models::event::default_schema_version())
}

impl EntityType {
    /// Get Chinese display name
    pub fn display_zh(&self) -> &'static str {
        match self {
            EntityType::Person => "人物",
            EntityType::Organization => "组织",
            EntityType::Project => "项目",
            EntityType::Artifact => "产物",
            EntityType::Concept => "概念",
            EntityType::Topic => "主题",
            EntityType::Activity => "活动",
            EntityType::Goal => "目标",
            EntityType::Skill => "技能",
            EntityType::Place => "地点",
            EntityType::Device => "设备",
            EntityType::Resource => "资源",
            EntityType::MemoryCluster => "记忆簇",
            EntityType::State => "状态",
        }
    }

    /// Returns plural snake_case string for YAML keys (e.g., "people", "topics")
    pub fn plural(&self) -> &'static str {
        match self {
            EntityType::Person => "people",
            EntityType::Organization => "organizations",
            EntityType::Project => "projects",
            EntityType::Artifact => "artifacts",
            EntityType::Concept => "concepts",
            EntityType::Topic => "topics",
            EntityType::Activity => "activities",
            EntityType::Goal => "goals",
            EntityType::Skill => "skills",
            EntityType::Place => "places",
            EntityType::Device => "devices",
            EntityType::Resource => "resources",
            EntityType::MemoryCluster => "memory_clusters",
            EntityType::State => "states",
        }
    }

    /// Parse from singular snake_case string (e.g., "person" -> EntityType::Person)
    ///
    /// Returns `EntityType::Topic` for unknown type strings.
    pub fn from_singular(s: &str) -> Self {
        match s {
            "person" => EntityType::Person,
            "organization" => EntityType::Organization,
            "project" => EntityType::Project,
            "artifact" => EntityType::Artifact,
            "concept" => EntityType::Concept,
            "topic" => EntityType::Topic,
            "activity" => EntityType::Activity,
            "goal" => EntityType::Goal,
            "skill" => EntityType::Skill,
            "place" => EntityType::Place,
            "device" => EntityType::Device,
            "resource" => EntityType::Resource,
            "memory_cluster" => EntityType::MemoryCluster,
            "state" => EntityType::State,
            _ => EntityType::Topic,
        }
    }
}

impl Entity {
    /// Generate entity ID based on type
    ///
    /// Creates a deterministic ID from the entity type and a slug.
    /// Example: `Entity::generate_id(&EntityType::Person, "john")` -> "person-john"
    pub fn generate_id(entity_type: &EntityType, slug: &str) -> String {
        let prefix = match entity_type {
            EntityType::Person => "person",
            EntityType::Organization => "org",
            EntityType::Project => "proj",
            EntityType::Artifact => "artifact",
            EntityType::Concept => "concept",
            EntityType::Topic => "topic",
            EntityType::Activity => "activity",
            EntityType::Goal => "goal",
            EntityType::Skill => "skill",
            EntityType::Place => "place",
            EntityType::Device => "device",
            EntityType::Resource => "resource",
            EntityType::MemoryCluster => "memory",
            EntityType::State => "state",
        };
        format!("{}-{}", prefix, slug)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // EntityType Tests
    // =============================================================================

    #[test]
    fn test_entity_type_display() {
        assert_eq!(EntityType::Person.to_string(), "person");
        assert_eq!(EntityType::Organization.to_string(), "organization");
        assert_eq!(EntityType::Topic.to_string(), "topic");
        assert_eq!(EntityType::MemoryCluster.to_string(), "memory_cluster");
    }

    #[test]
    fn test_entity_type_plural() {
        assert_eq!(EntityType::Person.plural(), "people");
        assert_eq!(EntityType::Organization.plural(), "organizations");
        assert_eq!(EntityType::Project.plural(), "projects");
        assert_eq!(EntityType::Artifact.plural(), "artifacts");
        assert_eq!(EntityType::Topic.plural(), "topics");
        assert_eq!(EntityType::MemoryCluster.plural(), "memory_clusters");
    }

    #[test]
    fn test_entity_type_from_singular() {
        assert_eq!(EntityType::from_singular("person"), EntityType::Person);
        assert_eq!(
            EntityType::from_singular("organization"),
            EntityType::Organization
        );
        assert_eq!(EntityType::from_singular("topic"), EntityType::Topic);
        assert_eq!(
            EntityType::from_singular("memory_cluster"),
            EntityType::MemoryCluster
        );
    }

    #[test]
    fn test_entity_type_from_singular_unknown_defaults_to_topic() {
        assert_eq!(EntityType::from_singular("unknown_type"), EntityType::Topic);
        assert_eq!(EntityType::from_singular(""), EntityType::Topic);
    }

    #[test]
    fn test_entity_type_display_zh() {
        assert_eq!(EntityType::Person.display_zh(), "人物");
        assert_eq!(EntityType::Organization.display_zh(), "组织");
        assert_eq!(EntityType::Project.display_zh(), "项目");
        assert_eq!(EntityType::Topic.display_zh(), "主题");
        assert_eq!(EntityType::MemoryCluster.display_zh(), "记忆簇");
    }

    // =============================================================================
    // EntityStatus Tests
    // =============================================================================

    #[test]
    fn test_entity_status_display() {
        assert_eq!(EntityStatus::Active.to_string(), "active");
        assert_eq!(EntityStatus::Archived.to_string(), "archived");
        assert_eq!(EntityStatus::Merged.to_string(), "merged");
    }

    #[test]
    fn test_entity_status_default() {
        let status = EntityStatus::default();
        assert!(matches!(status, EntityStatus::Active));
    }

    // =============================================================================
    // Entity Struct Tests
    // =============================================================================

    #[test]
    fn test_generate_id() {
        assert_eq!(
            Entity::generate_id(&EntityType::Person, "john"),
            "person-john"
        );
        assert_eq!(
            Entity::generate_id(&EntityType::Organization, "acme"),
            "org-acme"
        );
        assert_eq!(Entity::generate_id(&EntityType::Topic, "ai"), "topic-ai");
        assert_eq!(
            Entity::generate_id(&EntityType::MemoryCluster, "cluster1"),
            "memory-cluster1"
        );
    }

    #[test]
    fn test_entity_full_construction() {
        let entity = Entity {
            schema: "entity/v1".to_string(),
            id: "ent-person-test".to_string(),
            type_: EntityType::Person,
            label: "Test Person".to_string(),
            aliases: vec!["Test".to_string(), "T".to_string()],
            status: EntityStatus::Active,
            confidence: 0.95,
            classification: EntityClassification {
                domain: Some("technology".to_string()),
                parent: vec!["ent-org-acme".to_string()],
            },
            identity: EntityIdentity {
                description: Some("A test person entity".to_string()),
                summary: Some("Test summary".to_string()),
            },
            multimedia: EntityMultimedia {
                images: vec!["/img/test.jpg".to_string()],
                voices: vec![],
                embeddings_text: Some("emb-001".to_string()),
            },
            links: EntityLinks {
                wikipedia: Some("https://example.com".to_string()),
                papers: vec!["https://paper.example.com".to_string()],
                custom: std::collections::HashMap::new(),
            },
            evolution: EntityEvolution {
                merged_from: vec![],
                split_to: vec!["ent-new".to_string()],
            },
            metrics: EntityMetrics {
                event_count: 10,
                last_seen: None,
                activity_score: Some(0.8),
            },
            created_at: None,
            updated_at: None,
            schema_version: 1,
        };

        assert_eq!(entity.schema, "entity/v1");
        assert_eq!(entity.id, "ent-person-test");
        assert!(matches!(entity.type_, EntityType::Person));
        assert!(matches!(entity.status, EntityStatus::Active));
        assert_eq!(entity.aliases.len(), 2);
        assert_eq!(entity.classification.domain.as_deref(), Some("technology"));
        assert_eq!(entity.multimedia.images.len(), 1);
        assert_eq!(entity.metrics.event_count, 10);
    }

    #[test]
    fn test_entity_default() {
        let entity = Entity {
            schema: "entity/v1".to_string(),
            id: "ent-test".to_string(),
            type_: EntityType::default(),
            label: "Test".to_string(),
            aliases: vec![],
            status: EntityStatus::default(),
            confidence: default_confidence(),
            classification: EntityClassification::default(),
            identity: EntityIdentity::default(),
            multimedia: EntityMultimedia::default(),
            links: EntityLinks::default(),
            evolution: EntityEvolution::default(),
            metrics: EntityMetrics::default(),
            created_at: None,
            updated_at: None,
            schema_version: default_schema_version(),
        };

        assert!(matches!(entity.type_, EntityType::Topic));
        assert!(matches!(entity.status, EntityStatus::Active));
        assert!(entity.aliases.is_empty());
        assert!(entity.classification.domain.is_none());
    }

    #[test]
    fn test_default_entity_schema() {
        let schema = default_entity_schema();
        assert!(schema.starts_with("entity/v"));
    }
}
