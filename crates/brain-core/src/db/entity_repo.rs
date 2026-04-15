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
        let mut stmt = self.conn.prepare(
            "SELECT id, schema_version, type, label, aliases, status, confidence,
                    classification_domain, classification_parent,
                    identity_description, summary,
                    images, voices, embeddings_text,
                    links_wikipedia, links_papers,
                    merged_from, split_to,
                    event_count, last_seen, activity_score,
                    created_at, updated_at
             FROM entities WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(self.row_to_entity(row)?))
        } else {
            Ok(None)
        }
    }

    /// List entities by type
    pub fn find_by_type(&self, entity_type: &EntityType) -> Result<Vec<Entity>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, schema_version, type, label, aliases, status, confidence,
                    classification_domain, classification_parent,
                    identity_description, summary,
                    images, voices, embeddings_text,
                    links_wikipedia, links_papers,
                    merged_from, split_to,
                    event_count, last_seen, activity_score,
                    created_at, updated_at
             FROM entities WHERE type = ?1 ORDER BY label",
        )?;
        let mut rows = stmt.query(params![entity_type.to_string()])?;
        let mut entities = Vec::new();

        while let Some(row) = rows.next()? {
            entities.push(self.row_to_entity(row)?);
        }

        Ok(entities)
    }

    /// Search entities by label
    pub fn search(&self, keyword: &str) -> Result<Vec<Entity>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, schema_version, type, label, aliases, status, confidence,
                    classification_domain, classification_parent,
                    identity_description, summary,
                    images, voices, embeddings_text,
                    links_wikipedia, links_papers,
                    merged_from, split_to,
                    event_count, last_seen, activity_score,
                    created_at, updated_at
             FROM entities WHERE label LIKE ?1 ORDER BY label",
        )?;
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
        let mut stmt = self.conn.prepare(
            "SELECT id, schema_version, type, label, aliases, status, confidence,
                    classification_domain, classification_parent,
                    identity_description, summary,
                    images, voices, embeddings_text,
                    links_wikipedia, links_papers,
                    merged_from, split_to,
                    event_count, last_seen, activity_score,
                    created_at, updated_at
             FROM entities ORDER BY type, label",
        )?;
        let mut rows = stmt.query([])?;
        let mut entities = Vec::new();

        while let Some(row) = rows.next()? {
            entities.push(self.row_to_entity(row)?);
        }

        Ok(entities)
    }

    /// 将数据库 Row 转换为 Entity 模型
    ///
    /// # 列索引映射（与 entities 表列顺序一致）
    ///
    /// 0=id, 1=schema_version, 2=type, 3=label, 4=aliases, 5=status, 6=confidence,
    /// 7=classification_domain, 8=classification_parent, 9=identity_description, 10=summary,
    /// 11=images, 12=voices, 13=embeddings_text, 14=links_wikipedia, 15=links_papers,
    /// 16=merged_from, 17=split_to, 18=event_count, 19=last_seen, 20=activity_score,
    /// 21=created_at, 22=updated_at
    fn row_to_entity(&self, row: &Row) -> Result<Entity, Error> {
        let type_str: String = row.get(2)?; // 2: type
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

        let status_str: String = row.get(5)?; // 5: status (原错误使用4)
        let status = match status_str.as_str() {
            "archived" => EntityStatus::Archived,
            "merged" => EntityStatus::Merged,
            _ => EntityStatus::Active,
        };

        let aliases_str: Option<String> = row.get(4)?; // 4: aliases (原错误使用3)
        let aliases: Vec<String> = aliases_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let parent_str: Option<String> = row.get(8)?; // 8: classification_parent
        let parent: Vec<String> = parent_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let images_str: Option<String> = row.get(11)?; // 11: images
        let images: Vec<String> = images_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let voices_str: Option<String> = row.get(12)?; // 12: voices
        let voices: Vec<String> = voices_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let papers_str: Option<String> = row.get(15)?; // 15: links_papers
        let papers: Vec<String> = papers_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let merged_from_str: Option<String> = row.get(16)?; // 16: merged_from
        let merged_from: Vec<String> = merged_from_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let split_to_str: Option<String> = row.get(17)?; // 17: split_to
        let split_to: Vec<String> = split_to_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let last_seen_ts: Option<i64> = row.get(19)?; // 19: last_seen

        Ok(Entity {
            schema: "entity/v1".to_string(),
            id: row.get(0)?, // 0: id
            type_: entity_type,
            label: row.get(3)?, // 3: label
            aliases,
            status,
            confidence: row.get(6)?, // 6: confidence (原错误使用5)
            classification: EntityClassification {
                domain: row.get(7)?, // 7: classification_domain
                parent,
            },
            identity: EntityIdentity {
                description: row.get(9)?, // 9: identity_description
                summary: row.get(10)?,    // 10: summary
            },
            multimedia: EntityMultimedia {
                images,
                voices,
                embeddings_text: row.get(13)?, // 13: embeddings_text
            },
            links: EntityLinks {
                wikipedia: row.get(14)?, // 14: links_wikipedia
                papers,
                custom: std::collections::HashMap::new(),
            },
            evolution: EntityEvolution {
                merged_from,
                split_to,
            },
            metrics: EntityMetrics {
                event_count: row.get(18)?, // 18: event_count
                last_seen: last_seen_ts.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                activity_score: row.get(20)?, // 20: activity_score
            },
            created_at: row
                .get::<_, Option<i64>>(21)? // 21: created_at
                .and_then(|ts| DateTime::from_timestamp(ts, 0)),
            updated_at: row
                .get::<_, Option<i64>>(22)? // 22: updated_at
                .and_then(|ts| DateTime::from_timestamp(ts, 0)),
            schema_version: row.get(1)?, // 1: schema_version
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::run_migrations;
    use chrono::Utc;

    fn create_test_db() -> Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    /// 创建一个测试用 Entity
    fn create_test_entity(id: &str, label: &str, entity_type: EntityType) -> Entity {
        Entity {
            schema: "entity/v1".to_string(),
            id: id.to_string(),
            type_: entity_type,
            label: label.to_string(),
            aliases: vec!["alias1".to_string(), "alias2".to_string()],
            status: EntityStatus::Active,
            confidence: 0.9,
            classification: EntityClassification {
                domain: Some("test".to_string()),
                parent: vec!["parent1".to_string()],
            },
            identity: EntityIdentity {
                description: Some("Test entity".to_string()),
                summary: Some("A test entity".to_string()),
            },
            multimedia: EntityMultimedia {
                images: vec!["img1.jpg".to_string()],
                voices: vec![],
                embeddings_text: None,
            },
            links: EntityLinks {
                wikipedia: Some("https://example.com".to_string()),
                papers: vec![],
                custom: std::collections::HashMap::new(),
            },
            evolution: EntityEvolution {
                merged_from: vec![],
                split_to: vec![],
            },
            metrics: EntityMetrics {
                event_count: 5,
                last_seen: Some(Utc::now()),
                activity_score: Some(0.7),
            },
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            schema_version: 1,
        }
    }

    #[test]
    fn test_upsert_new_entity() {
        let conn = create_test_db();
        let repo = EntityRepository::new(&conn);

        let entity = create_test_entity("ent-1", "Zhang San", EntityType::Person);
        let result = repo.upsert(&entity);
        assert!(result.is_ok(), "upsert should succeed");

        // 用直接 SQL 验证（避免 row_to_entity bug）
        let label: String = conn
            .query_row("SELECT label FROM entities WHERE id = 'ent-1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(label, "Zhang San");

        let aliases_raw: String = conn
            .query_row(
                "SELECT aliases FROM entities WHERE id = 'ent-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let aliases: Vec<String> = serde_json::from_str(&aliases_raw).unwrap();
        assert_eq!(aliases, vec!["alias1", "alias2"]);
    }

    // 注意：以下测试使用直接 SQL 查询验证存储结果，
    // 因为 row_to_entity 的列索引与 SELECT * 返回顺序不匹配（有原有 bug）
    // 这些测试验证 upsert 正确写入数据库，而不依赖 find_by_id 等读取方法

    #[test]
    fn test_upsert_inserts_data() {
        let conn = create_test_db();
        let repo = EntityRepository::new(&conn);

        repo.upsert(&create_test_entity(
            "ent-1",
            "Zhang San",
            EntityType::Person,
        ))
        .unwrap();

        // 用直接 SQL 验证
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entities WHERE id = 'ent-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_upsert_updates_existing() {
        let conn = create_test_db();
        let repo = EntityRepository::new(&conn);

        repo.upsert(&create_test_entity("ent-1", "Original", EntityType::Person))
            .unwrap();
        repo.upsert(&create_test_entity("ent-1", "Updated", EntityType::Person))
            .unwrap();

        // 用直接 SQL 验证（避免 row_to_entity bug）
        let label: String = conn
            .query_row("SELECT label FROM entities WHERE id = 'ent-1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(label, "Updated");
    }

    #[test]
    fn test_find_by_id_loads_correct_data() {
        let conn = create_test_db();
        let repo = EntityRepository::new(&conn);

        repo.upsert(&create_test_entity(
            "ent-find",
            "Zhang San",
            EntityType::Person,
        ))
        .unwrap();

        let found = repo.find_by_id("ent-find").unwrap().unwrap();
        assert_eq!(found.label, "Zhang San");
        assert_eq!(
            found.aliases,
            vec!["alias1".to_string(), "alias2".to_string()]
        );
        assert!(matches!(found.status, EntityStatus::Active));
        assert_eq!(found.confidence, 0.9);
    }

    #[test]
    fn test_find_by_id_not_found() {
        let conn = create_test_db();
        let repo = EntityRepository::new(&conn);

        let result = repo.find_by_id("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_find_by_type() {
        let conn = create_test_db();
        let repo = EntityRepository::new(&conn);

        repo.upsert(&create_test_entity("e1", "Person1", EntityType::Person))
            .unwrap();
        repo.upsert(&create_test_entity("e2", "Person2", EntityType::Person))
            .unwrap();
        repo.upsert(&create_test_entity("e3", "Org1", EntityType::Organization))
            .unwrap();

        let persons = repo.find_by_type(&EntityType::Person).unwrap();
        assert_eq!(persons.len(), 2);

        let orgs = repo.find_by_type(&EntityType::Organization).unwrap();
        assert_eq!(orgs.len(), 1);
    }

    #[test]
    fn test_search_by_label() {
        let conn = create_test_db();
        let repo = EntityRepository::new(&conn);

        repo.upsert(&create_test_entity("e1", "Zhang San", EntityType::Person))
            .unwrap();
        repo.upsert(&create_test_entity("e2", "Li Si", EntityType::Person))
            .unwrap();
        repo.upsert(&create_test_entity(
            "e3",
            "Zhang Organization",
            EntityType::Organization,
        ))
        .unwrap();

        let results = repo.search("Zhang").unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_all_entities() {
        let conn = create_test_db();
        let repo = EntityRepository::new(&conn);

        repo.upsert(&create_test_entity("e1", "A", EntityType::Person))
            .unwrap();
        repo.upsert(&create_test_entity("e2", "B", EntityType::Topic))
            .unwrap();

        let all = repo.all().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_unknown_entity_type_defaults_to_topic() {
        let conn = create_test_db();
        let repo = EntityRepository::new(&conn);

        repo.upsert(&create_test_entity(
            "e-type",
            "Test Entity",
            EntityType::Person,
        ))
        .unwrap();

        // 直接修改数据库中的 type 列为未知值
        conn.execute(
            "UPDATE entities SET type = 'unknown_type' WHERE id = 'e-type'",
            [],
        )
        .unwrap();

        let found = repo.find_by_id("e-type").unwrap().unwrap();
        assert_eq!(found.type_, EntityType::Topic);
    }

    #[test]
    fn test_unknown_status_defaults_to_active() {
        let conn = create_test_db();
        let repo = EntityRepository::new(&conn);

        repo.upsert(&create_test_entity(
            "e-status",
            "Test Entity",
            EntityType::Person,
        ))
        .unwrap();

        // 直接修改数据库中的 status 列为未知值
        conn.execute(
            "UPDATE entities SET status = 'unknown_status' WHERE id = 'e-status'",
            [],
        )
        .unwrap();

        let found = repo.find_by_id("e-status").unwrap().unwrap();
        assert!(matches!(found.status, EntityStatus::Active));
    }

    #[test]
    fn test_delete_entity() {
        let conn = create_test_db();
        let repo = EntityRepository::new(&conn);

        repo.upsert(&create_test_entity(
            "ent-del",
            "ToDelete",
            EntityType::Person,
        ))
        .unwrap();

        repo.delete("ent-del").unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entities WHERE id = 'ent-del'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_delete_cascades_to_event_entities() {
        let conn = create_test_db();
        let repo = EntityRepository::new(&conn);

        repo.upsert(&create_test_entity("ent-1", "Test", EntityType::Person))
            .unwrap();

        conn.execute(
            "INSERT INTO events (id, time_start, type) VALUES ('evt-1', 0, 'note')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO event_entities (event_id, entity_id, entity_type) VALUES ('evt-1', 'ent-1', 'person')",
            [],
        )
        .unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM event_entities WHERE entity_id = 'ent-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        repo.delete("ent-1").unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM event_entities WHERE entity_id = 'ent-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_json_fields_stored_correctly() {
        let conn = create_test_db();
        let repo = EntityRepository::new(&conn);

        repo.upsert(&create_test_entity(
            "e-json",
            "JSON Test",
            EntityType::Concept,
        ))
        .unwrap();

        let aliases_raw: String = conn
            .query_row(
                "SELECT aliases FROM entities WHERE id = 'e-json'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let parsed: Vec<String> = serde_json::from_str(&aliases_raw).unwrap();
        assert_eq!(parsed, vec!["alias1", "alias2"]);
    }

    #[test]
    fn test_timestamps_stored_as_unix() {
        let conn = create_test_db();
        let repo = EntityRepository::new(&conn);

        let now = Utc::now();
        let entity = Entity {
            created_at: Some(now),
            updated_at: Some(now),
            ..create_test_entity("e-ts", "Timestamp Test", EntityType::Skill)
        };
        repo.upsert(&entity).unwrap();

        let ts: i64 = conn
            .query_row(
                "SELECT created_at FROM entities WHERE id = 'e-ts'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        let now_ts = Utc::now().timestamp();
        assert!((ts - now_ts).abs() < 2);
    }

    #[test]
    fn test_classification_parent_json_roundtrip() {
        let conn = create_test_db();
        let repo = EntityRepository::new(&conn);

        let entity = Entity {
            classification: EntityClassification {
                domain: Some("tech".to_string()),
                parent: vec!["work".to_string(), "open_source".to_string()],
            },
            ..create_test_entity("e-class", "Classification Test", EntityType::Project)
        };
        repo.upsert(&entity).unwrap();

        let parent_raw: String = conn
            .query_row(
                "SELECT classification_parent FROM entities WHERE id = 'e-class'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let parsed: Vec<String> = serde_json::from_str(&parent_raw).unwrap();
        assert_eq!(parsed, vec!["work", "open_source"]);
    }

    #[test]
    fn test_links_papers_json_roundtrip() {
        let conn = create_test_db();
        let repo = EntityRepository::new(&conn);

        let entity = Entity {
            links: EntityLinks {
                wikipedia: Some("https://en.wikipedia.org/wiki/Test".to_string()),
                papers: vec![
                    "https://arxiv.org/abs/test".to_string(),
                    "https://doi.org/test".to_string(),
                ],
                custom: std::collections::HashMap::new(),
            },
            ..create_test_entity("e-links", "Links Test", EntityType::Concept)
        };
        repo.upsert(&entity).unwrap();

        let papers_raw: String = conn
            .query_row(
                "SELECT links_papers FROM entities WHERE id = 'e-links'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let parsed: Vec<String> = serde_json::from_str(&papers_raw).unwrap();
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn test_entity_type_conversion() {
        let types = vec![
            (EntityType::Person, "person"),
            (EntityType::Organization, "organization"),
            (EntityType::Project, "project"),
            (EntityType::Place, "place"),
            (EntityType::Topic, "topic"),
            (EntityType::Concept, "concept"),
            (EntityType::Activity, "activity"),
            (EntityType::Goal, "goal"),
            (EntityType::Skill, "skill"),
            (EntityType::Artifact, "artifact"),
            (EntityType::Device, "device"),
            (EntityType::Resource, "resource"),
            (EntityType::MemoryCluster, "memory_cluster"),
            (EntityType::State, "state"),
        ];

        for (entity_type, expected_str) in types {
            assert_eq!(entity_type.to_string(), expected_str);
        }
    }
}
