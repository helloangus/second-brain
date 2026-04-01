//! Entity model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Entity type enum (extended set)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Person,
    Organization,
    Project,
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EntityStatus {
    #[default]
    Active,
    Archived,
    Merged,
}

/// Links to external resources
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct EntityLinks {
    pub wikipedia: Option<String>,
    #[serde(default)]
    pub papers: Vec<String>,
    #[serde(default)]
    pub custom: std::collections::HashMap<String, String>,
}

/// Classification information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct EntityClassification {
    pub domain: Option<String>,
    #[serde(default)]
    pub parent: Vec<String>,
}

/// Identity description
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct EntityIdentity {
    pub description: Option<String>,
    pub summary: Option<String>,
}

/// Multimedia references
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct EntityMultimedia {
    #[serde(default)]
    pub images: Vec<String>,
    #[serde(default)]
    pub voices: Vec<String>,
    pub embeddings_text: Option<String>,
}

/// Evolution tracking
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct EntityEvolution {
    #[serde(default)]
    pub merged_from: Vec<String>,
    #[serde(default)]
    pub split_to: Vec<String>,
}

/// Usage metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct EntityMetrics {
    #[serde(default)]
    pub event_count: i32,
    pub last_seen: Option<DateTime<Utc>>,
    pub activity_score: Option<f64>,
}

/// Full Entity struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub schema: String,
    pub id: String,
    pub type_: EntityType,
    pub label: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub status: EntityStatus,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    #[serde(default)]
    pub classification: EntityClassification,
    #[serde(default)]
    pub identity: EntityIdentity,
    #[serde(default)]
    pub multimedia: EntityMultimedia,
    #[serde(default)]
    pub links: EntityLinks,
    #[serde(default)]
    pub evolution: EntityEvolution,
    #[serde(default)]
    pub metrics: EntityMetrics,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(default = "default_schema_version")]
    pub schema_version: i32,
}

fn default_confidence() -> f64 {
    0.5
}

fn default_schema_version() -> i32 {
    1
}

impl Entity {
    /// Generate entity ID based on type
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
