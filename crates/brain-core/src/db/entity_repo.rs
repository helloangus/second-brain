//! Entity repository

use crate::error::Error;
use crate::models::{
    Entity, EntityClassification, EntityEvolution, EntityIdentity, EntityLinks, EntityMetrics,
    EntityMultimedia, EntityStatus, EntityType,
};
use chrono::DateTime;
use rusqlite::{params, Connection, Row};

pub struct EntityRepository<'a> {
    conn: &'a Connection,
}

impl<'a> EntityRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Insert or update an entity
    pub fn upsert(&self, entity: &Entity) -> Result<(), Error> {
        let entity_type_str = entity.type_.to_string();
        let status_str = match entity.status {
            EntityStatus::Active => "active",
            EntityStatus::Archived => "archived",
            EntityStatus::Merged => "merged",
        };

        self.conn.execute(
            r#"INSERT OR REPLACE INTO entities
               (id, schema_version, type, label, aliases, status, confidence,
                classification_domain, classification_parent,
                identity_description, summary,
                images, voices, embeddings_text,
                links_wikipedia, links_papers,
                merged_from, split_to,
                event_count, last_seen, activity_score,
                created_at, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)"#,
            params![
                entity.id,
                entity.schema_version,
                entity_type_str,
                entity.label,
                serde_json::to_string(&entity.aliases).ok(),
                status_str,
                entity.confidence,
                entity.classification.domain,
                serde_json::to_string(&entity.classification.parent).ok(),
                entity.identity.description,
                entity.identity.summary,
                serde_json::to_string(&entity.multimedia.images).ok(),
                serde_json::to_string(&entity.multimedia.voices).ok(),
                entity.multimedia.embeddings_text,
                entity.links.wikipedia,
                serde_json::to_string(&entity.links.papers).ok(),
                serde_json::to_string(&entity.evolution.merged_from).ok(),
                serde_json::to_string(&entity.evolution.split_to).ok(),
                entity.metrics.event_count,
                entity.metrics.last_seen.map(|t| t.timestamp()),
                entity.metrics.activity_score,
                entity.created_at.map(|t| t.timestamp()),
                entity.updated_at.map(|t| t.timestamp()),
            ],
        )?;

        Ok(())
    }

    /// Delete an entity by ID
    pub fn delete(&self, id: &str) -> Result<(), Error> {
        self.conn.execute(
            "DELETE FROM event_entities WHERE entity_id = ?1",
            params![id],
        )?;
        self.conn
            .execute("DELETE FROM entities WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Find entity by ID
    pub fn find_by_id(&self, id: &str) -> Result<Option<Entity>, Error> {
        let mut stmt = self.conn.prepare("SELECT * FROM entities WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(self.row_to_entity(row)?))
        } else {
            Ok(None)
        }
    }

    /// List entities by type
    pub fn find_by_type(&self, entity_type: &EntityType) -> Result<Vec<Entity>, Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM entities WHERE type = ?1 ORDER BY label")?;
        let mut rows = stmt.query(params![entity_type.to_string()])?;
        let mut entities = Vec::new();

        while let Some(row) = rows.next()? {
            entities.push(self.row_to_entity(row)?);
        }

        Ok(entities)
    }

    /// Search entities by label
    pub fn search(&self, keyword: &str) -> Result<Vec<Entity>, Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM entities WHERE label LIKE ?1 ORDER BY label")?;
        let pattern = format!("%{}%", keyword);
        let mut rows = stmt.query(params![pattern])?;
        let mut entities = Vec::new();

        while let Some(row) = rows.next()? {
            entities.push(self.row_to_entity(row)?);
        }

        Ok(entities)
    }

    /// Get all entities
    pub fn all(&self) -> Result<Vec<Entity>, Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM entities ORDER BY type, label")?;
        let mut rows = stmt.query([])?;
        let mut entities = Vec::new();

        while let Some(row) = rows.next()? {
            entities.push(self.row_to_entity(row)?);
        }

        Ok(entities)
    }

    fn row_to_entity(&self, row: &Row) -> Result<Entity, Error> {
        let type_str: String = row.get(2)?;
        let entity_type = match type_str.as_str() {
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
        };

        let status_str: String = row.get(4)?;
        let status = match status_str.as_str() {
            "archived" => EntityStatus::Archived,
            "merged" => EntityStatus::Merged,
            _ => EntityStatus::Active,
        };

        let aliases_str: Option<String> = row.get(3)?;
        let aliases: Vec<String> = aliases_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let parent_str: Option<String> = row.get(8)?;
        let parent: Vec<String> = parent_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let images_str: Option<String> = row.get(11)?;
        let images: Vec<String> = images_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let voices_str: Option<String> = row.get(12)?;
        let voices: Vec<String> = voices_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let papers_str: Option<String> = row.get(15)?;
        let papers: Vec<String> = papers_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let merged_from_str: Option<String> = row.get(16)?;
        let merged_from: Vec<String> = merged_from_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let split_to_str: Option<String> = row.get(17)?;
        let split_to: Vec<String> = split_to_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let last_seen_ts: Option<i64> = row.get(19)?;

        Ok(Entity {
            schema: "entity/v1".to_string(),
            id: row.get(0)?,
            type_: entity_type,
            label: row.get(3)?,
            aliases,
            status,
            confidence: row.get(5)?,
            classification: EntityClassification {
                domain: row.get(7)?,
                parent,
            },
            identity: EntityIdentity {
                description: row.get(9)?,
                summary: row.get(10)?,
            },
            multimedia: EntityMultimedia {
                images,
                voices,
                embeddings_text: row.get(13)?,
            },
            links: EntityLinks {
                wikipedia: row.get(14)?,
                papers,
                custom: std::collections::HashMap::new(),
            },
            evolution: EntityEvolution {
                merged_from,
                split_to,
            },
            metrics: EntityMetrics {
                event_count: row.get(18)?,
                last_seen: last_seen_ts.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                activity_score: row.get(20)?,
            },
            created_at: row
                .get::<_, Option<i64>>(21)?
                .and_then(|ts| DateTime::from_timestamp(ts, 0)),
            updated_at: row
                .get::<_, Option<i64>>(22)?
                .and_then(|ts| DateTime::from_timestamp(ts, 0)),
            schema_version: row.get(1)?,
        })
    }
}
