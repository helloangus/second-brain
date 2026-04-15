//! Event repository
//!
//! # 什么是 Event
//!
//! Event 是 Second Brain 的核心数据结构，代表一次性的发生的事情。
//! 与 Entity（长生命周期对象）的区别：
//!
//! - **Event**: 会议、照片、笔记、任务完成 — 发生在特定时间点
//! - **Entity**: 人物、公司、项目、概念 — 持续存在
//!
//! # 数据写入流程（重要）
//!
//! ```text
//! .md 文件 → indexerd 解析 → Event 模型
//!   → upsert() 同时写入：
//!       1. events 表（主表）
//!       2. events_fts 表（全文搜索索引）
//!       3. event_entities 表（实体关联）
//!       4. tags 表（标签索引）
//! ```
//!
//! # 读取特性
//!
//! 注意：`row_to_event()` 在读取时不从 DB 加载 tags，
//! 返回空的 `Vec::new()`。这意味着 DB 只是索引，
//! 完整数据在 .md 文件中。
//!
//! # 设计决策
//!
//! ## 为什么 `upsert` 要手动更新 FTS？
//!
//! FTS5 virtual table 不支持 `INSERT OR REPLACE`。
//! 因此 upsert 时必须先 DELETE 再 INSERT。
//!
//! ## 为什么 `update_entities` 要关闭外键检查？
//!
//! AI 从文本中提取的 entity_id 可能尚未存在于 entities 表中。
//! 严格的外键约束会导致插入失败。解决方案：
//! 1. 关闭外键检查
//! 2. 写入 event_entities
//! 3. 重新开启外键检查
//!
//! 这是一种"先写入后验证"的策略。
//!
//! ## 为什么删除要做级联清理？
//!
//! SQLite 不支持 `ON DELETE CASCADE`。因此 delete() 方法
//! 必须手动清理 event_entities、tags、event_relations、events_fts。

use crate::error::Error;
use crate::models::{
    DerivedRefs, EntityType, Event, EventAi, EventEntities, EventRelations, EventSource, EventTime,
    GraphHints, RawRefs,
};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row};
use std::collections::BTreeMap;

/// Event 仓库
///
/// 负责 Event 的增删改查，是三个仓库中最复杂的。
///
/// # 生命周期
///
/// `conn: &'a Connection` - 借用数据库连接。
pub struct EventRepository<'a> {
    conn: &'a Connection,
}

impl<'a> EventRepository<'a> {
    /// 创建 EventRepository
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// 插入或更新事件
    ///
    /// # SQL 策略
    ///
    /// 使用 `INSERT OR REPLACE`，相当于：
    /// - 如果 ID 不存在：INSERT
    /// - 如果 ID 已存在：DELETE 旧记录 + INSERT 新记录
    ///
    /// # 副作用
    ///
    /// 同时更新：
    /// - `events` 主表
    /// - `events_fts` 全文搜索索引
    /// - `event_entities` 实体关联
    /// - `tags` 标签索引
    ///
    /// # 性能考虑
    ///
    /// upsert 是最重的写操作，涉及 5 次数据库写入。
    /// 批量导入时建议使用事务包裹多个 upsert。
    pub fn upsert(&self, event: &Event) -> Result<(), Error> {
        let time_start = event.time.start.timestamp();
        let time_end = event.time.end.map(|t| t.timestamp());

        // 1. 写入 events 主表
        self.conn.execute(
            r#"INSERT OR REPLACE INTO events
               (id, schema_version, time_start, time_end, timezone, type, subtype,
                source_device, source_channel, source_capture_agent, confidence,
                ai_summary, ai_topics, ai_sentiment, extraction_version,
                importance, recurrence, created_at, ingested_at, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)"#,
            params![
                event.id,
                event.schema_version,
                time_start,
                time_end,
                event.time.timezone,
                event.type_.clone(),
                event.subtype,
                event.source.device,
                event.source.channel,
                event.source.capture_agent,
                event.confidence,
                event.ai.summary,
                serde_json::to_string(&event.ai.topics).ok(),  // Vec → JSON
                event.ai.sentiment,
                event.ai.extraction_version,
                event.graph_hints.importance,
                event.graph_hints.recurrence as i32,  // bool → 0/1
                event.created_at.map(|t| t.timestamp()),
                event.ingested_at.map(|t| t.timestamp()),
                chrono::Utc::now().timestamp(),
            ],
        )?;

        // 2. 更新 FTS 索引
        self.update_fts(event)?;

        // 3. 更新事件-实体关联
        self.update_entities(event)?;

        // 4. 更新标签
        self.update_tags(event)?;

        Ok(())
    }

    /// 更新 FTS 全文搜索索引
    ///
    /// # 实现
    ///
    /// FTS5 virtual table 不支持 `INSERT OR REPLACE`，
    /// 必须先删除旧记录再插入新记录。
    ///
    /// # content 字段构造
    ///
    /// `content` 拼接了 `id + ai_summary + tags`，
    /// 使得搜索时可以同时匹配这三种内容。
    fn update_fts(&self, event: &Event) -> Result<(), Error> {
        // 先删旧记录（FTS 不支持 INSERT OR REPLACE）
        self.conn
            .execute("DELETE FROM events_fts WHERE id = ?1", params![event.id])?;

        // 构造 content 字段
        let content = format!(
            "{} {} {}",
            event.id,
            event.ai.summary.as_deref().unwrap_or(""),
            event.tags.join(" ")
        );

        // 插入新记录
        self.conn.execute(
            "INSERT INTO events_fts (id, ai_summary, content) VALUES (?1, ?2, ?3)",
            params![event.id, event.ai.summary, content],
        )?;

        Ok(())
    }

    /// 更新事件-实体关联
    ///
    /// # 外键策略
    ///
    /// 插入前关闭 `PRAGMA foreign_keys`，允许 entity_id 尚未存在于 entities 表。
    /// 这是必要的，因为 AI 提取的实体可能还没入库。
    ///
    /// # 关联结构
    ///
    /// `EventEntities` 是 `BTreeMap<EntityType, Vec<String>>` 的 wrapper。
    /// 遍历时按 entity_type 分组插入。
    fn update_entities(&self, event: &Event) -> Result<(), Error> {
        // 临时关闭外键检查，允许 entity_id 尚未存在
        self.conn.execute_batch("PRAGMA foreign_keys = OFF;")?;

        // 删除旧关联
        self.conn.execute(
            "DELETE FROM event_entities WHERE event_id = ?1",
            params![event.id],
        )?;

        // 批量插入新关联
        let mut insert_stmt = self.conn.prepare(
            "INSERT INTO event_entities (event_id, entity_id, entity_type) VALUES (?1, ?2, ?3)",
        )?;

        for (entity_type, ids) in &event.entities.0 {
            for id in ids {
                insert_stmt.execute(params![event.id, id, entity_type.to_string()])?;
            }
        }

        // 重新开启外键检查
        self.conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        Ok(())
    }

    /// 更新事件标签
    ///
    /// # 实现
    ///
    /// 简单的先删后插：
    /// 1. DELETE 所有该事件的旧标签
    /// 2. INSERT 所有新标签
    ///
    /// 注意：confidence 字段固定为 1.0，未使用。
    fn update_tags(&self, event: &Event) -> Result<(), Error> {
        // 删除旧标签
        self.conn
            .execute("DELETE FROM tags WHERE event_id = ?1", params![event.id])?;

        // 插入新标签
        let mut insert_stmt = self
            .conn
            .prepare("INSERT INTO tags (event_id, tag, confidence) VALUES (?1, ?2, 1.0)")?;

        for tag in &event.tags {
            insert_stmt.execute(params![event.id, tag])?;
        }

        Ok(())
    }

    /// 删除事件
    ///
    /// # 级联清理
    ///
    /// 手动清理 4 张关联表：
    /// - `tags` - 事件的所有标签
    /// - `event_entities` - 事件的实体关联
    /// - `event_relations` - 事件间关系
    /// - `events_fts` - 全文搜索索引
    ///
    /// 注意：不会删除关联的实体本身（entities 表），
    /// 也不会删除关联的事件（其他事件的 event_relations）。
    ///
    /// # SQL
    ///
    /// ```sql
    /// DELETE FROM tags WHERE event_id = ?1;
    /// DELETE FROM event_entities WHERE event_id = ?1;
    /// DELETE FROM event_relations WHERE event_id = ?1;
    /// DELETE FROM events_fts WHERE id = ?1;
    /// DELETE FROM events WHERE id = ?1;
    /// ```
    pub fn delete(&self, id: &str) -> Result<(), Error> {
        self.conn
            .execute("DELETE FROM tags WHERE event_id = ?1", params![id])?;
        self.conn.execute(
            "DELETE FROM event_entities WHERE event_id = ?1",
            params![id],
        )?;
        self.conn.execute(
            "DELETE FROM event_relations WHERE event_id = ?1",
            params![id],
        )?;
        self.conn
            .execute("DELETE FROM events_fts WHERE id = ?1", params![id])?;
        self.conn
            .execute("DELETE FROM events WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// 按 ID 查询事件
    ///
    /// # 返回值
    ///
    /// - `Ok(Some(Event))` - 找到事件
    /// - `Ok(None)` - 事件不存在
    ///
    /// # 实现细节
    ///
    /// 1. 查询 events 主表（SELECT 指定列，避免 `SELECT *` 顺序问题）
    /// 2. 调用 `find_entities_by_event_id()` 加载关联实体
    /// 3. 调用 `row_to_event()` 转换
    ///
    /// # 注意
    ///
    /// tags 不从 DB 加载，事件返回时 tags 字段为空 Vec。
    pub fn find_by_id(&self, id: &str) -> Result<Option<Event>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, schema_version, time_start, time_end, timezone, type, subtype,
             source_device, source_channel, source_capture_agent, confidence,
             ai_summary, ai_topics, ai_sentiment, extraction_version,
             importance, recurrence, created_at, ingested_at, updated_at
             FROM events WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            let event_id: String = row.get(0)?;
            let entities = self.find_entities_by_event_id(&event_id)?;
            Ok(Some(self.row_to_event(row, entities)?))
        } else {
            Ok(None)
        }
    }

    /// FTS 全文搜索
    ///
    /// # SQL
    ///
    /// ```sql
    /// SELECT e.* FROM events e
    /// JOIN events_fts fts ON e.id = fts.id
    /// WHERE events_fts MATCH ?1
    /// ORDER BY rank
    /// ```
    ///
    /// # MATCH 语法
    ///
    /// FTS5 支持：
    /// - 单词搜索：`meeting`
    /// - 前缀搜索：`meeting*`
    /// - 短语搜索：`"meeting room"`
    /// - 布尔搜索：`meeting OR conference`
    ///
    /// # rank
    ///
    /// FTS5 的 `rank` 是相关性评分，`ORDER BY rank` 让最相关的排在前面。
    ///
    /// # 限制
    ///
    /// FTS5 索引的是 `ai_summary` 和拼接的 `id + tags`，
    /// 不是原始 markdown 内容。
    pub fn search(&self, keyword: &str) -> Result<Vec<Event>, Error> {
        let mut stmt = self.conn.prepare(
            r#"SELECT e.id, e.schema_version, e.time_start, e.time_end, e.timezone, e.type, e.subtype,
               e.source_device, e.source_channel, e.source_capture_agent, e.confidence,
               e.ai_summary, e.ai_topics, e.ai_sentiment, e.extraction_version,
               e.importance, e.recurrence, e.created_at, e.ingested_at, e.updated_at
               FROM events e
               JOIN events_fts fts ON e.id = fts.id
               WHERE events_fts MATCH ?1
               ORDER BY rank"#,
        )?;

        let mut rows = stmt.query(params![keyword])?;
        let mut events = Vec::new();

        while let Some(row) = rows.next()? {
            let event_id: String = row.get(0)?;
            let entities = self.find_entities_by_event_id(&event_id)?;
            events.push(self.row_to_event(row, entities)?);
        }

        Ok(events)
    }

    /// 按时间范围查询事件
    ///
    /// # 参数
    ///
    /// * `start` - 范围起始时间（闭区间）
    /// * `end` - 范围结束时间（闭区间）
    ///
    /// # SQL
    ///
    /// ```sql
    /// WHERE time_start >= ?1 AND time_start <= ?2 ORDER BY time_start
    /// ```
    ///
    /// # 索引
    ///
    /// `idx_events_time_start` 索引加速这个查询。
    ///
    /// # 注意
    ///
    /// 使用 `time_start` 而非 `time_end` 判断范围。
    /// 对于区间事件（time_start != time_end），如果起始时间在范围内就能被查到。
    pub fn find_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Event>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, schema_version, time_start, time_end, timezone, type, subtype,
             source_device, source_channel, source_capture_agent, confidence,
             ai_summary, ai_topics, ai_sentiment, extraction_version,
             importance, recurrence, created_at, ingested_at, updated_at
             FROM events WHERE time_start >= ?1 AND time_start <= ?2 ORDER BY time_start",
        )?;

        let mut rows = stmt.query(params![start.timestamp(), end.timestamp()])?;
        let mut events = Vec::new();

        while let Some(row) = rows.next()? {
            let event_id: String = row.get(0)?;
            let entities = self.find_entities_by_event_id(&event_id)?;
            events.push(self.row_to_event(row, entities)?);
        }

        Ok(events)
    }

    /// 查询所有事件
    ///
    /// # 返回值
    ///
    /// 按时间倒序排列（最新的在前）的所有事件。
    ///
    /// # 性能
    ///
    /// 对于大量数据，这可能会很慢。建议使用 `find_by_time_range` 限制范围。
    pub fn all(&self) -> Result<Vec<Event>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, schema_version, time_start, time_end, timezone, type, subtype,
             source_device, source_channel, source_capture_agent, confidence,
             ai_summary, ai_topics, ai_sentiment, extraction_version,
             importance, recurrence, created_at, ingested_at, updated_at
             FROM events ORDER BY time_start DESC",
        )?;
        let mut rows = stmt.query([])?;
        let mut events = Vec::new();

        while let Some(row) = rows.next()? {
            let event_id: String = row.get(0)?;
            let entities = self.find_entities_by_event_id(&event_id)?;
            events.push(self.row_to_event(row, entities)?);
        }

        Ok(events)
    }

    /// 将数据库 Row 转换为 Event 模型
    ///
    /// # 参数
    ///
    /// * `row` - 数据库行（从 SELECT 结果）
    /// * `entities` - 已经加载好的事件-实体关联
    ///
    /// # 转换规则
    ///
    /// - Unix timestamp → `DateTime<Utc>`
    /// - `None` timestamp → `Utc::now()` 或 `None`
    /// - `ai_topics` JSON → `Vec<String>`
    /// - `recurrence` INTEGER → bool（非 0 = true）
    ///
    /// # 不从 DB 加载的字段
    ///
    /// - `tags` - 返回空 Vec（DB 只写不读）
    /// - `raw_refs` - 返回默认空结构
    /// - `derived_refs` - 返回默认空结构
    /// - `relations` - 返回默认空结构
    ///
    /// # 列索引映射
    ///
    /// 注意：`SELECT` 列顺序必须与这里一致！
    /// 0=id, 1=schema_version, 2=time_start, 3=time_end, 4=timezone, 5=type,
    /// 6=subtype, 7=source_device, 8=source_channel, 9=source_capture_agent,
    /// 10=confidence, 11=ai_summary, 12=ai_topics, 13=ai_sentiment,
    /// 14=extraction_version, 15=importance, 16=recurrence,
    /// 17=created_at, 18=ingested_at, 19=updated_at
    fn row_to_event(&self, row: &Row, entities: EventEntities) -> Result<Event, Error> {
        let time_start_ts: i64 = row.get(2)?;
        let time_end_ts: Option<i64> = row.get(3)?;
        let timezone: String = row.get(4)?;
        let type_: String = row.get(5)?;

        let ai_topics_str: Option<String> = row.get(12)?;
        let ai_topics: Vec<String> = ai_topics_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        Ok(Event {
            schema: "event/v1".to_string(),
            id: row.get(0)?,
            type_,
            subtype: row.get(6)?,
            time: EventTime {
                start: DateTime::from_timestamp(time_start_ts, 0).unwrap_or_else(Utc::now),
                end: time_end_ts.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                timezone,
            },
            created_at: row
                .get::<_, Option<i64>>(17)?
                .and_then(|ts| DateTime::from_timestamp(ts, 0)),
            ingested_at: row
                .get::<_, Option<i64>>(18)?
                .and_then(|ts| DateTime::from_timestamp(ts, 0)),
            source: EventSource {
                device: row.get(7)?,
                channel: row.get(8)?,
                capture_agent: row.get(9)?,
            },
            confidence: row.get(10)?,
            entities,
            tags: Vec::new(),                     // DB 不加载
            raw_refs: RawRefs::default(),         // DB 不存储
            derived_refs: DerivedRefs::default(), // DB 不存储
            ai: EventAi {
                summary: row.get(11)?,
                extended: None,
                topics: ai_topics,
                sentiment: row.get(13)?,
                extraction_version: row.get(14)?,
            },
            relations: EventRelations::default(), // DB 不加载
            graph_hints: GraphHints {
                importance: row.get(15)?,
                recurrence: row.get::<_, i32>(16)? != 0, // 0/1 → bool
            },
            schema_version: row.get(1)?,
        })
    }

    /// 加载事件关联的实体
    ///
    /// # 返回值
    ///
    /// `EventEntities` - 按类型分组的 entity_id 列表
    ///
    /// # 实现
    ///
    /// 从 `event_entities` 表查询，
    /// 按 `entity_type` 分组构建 `BTreeMap`。
    /// 未知类型会被跳过（`continue`）。
    ///
    /// # EntityType 解析
    ///
    /// 字符串到枚举的转换，未知类型默认跳过。
    /// 这意味着有损坏数据的 events 仍然可以加载。
    pub fn find_entities_by_event_id(&self, event_id: &str) -> Result<EventEntities, Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT entity_id, entity_type FROM event_entities WHERE event_id = ?1")?;

        let mut map: BTreeMap<EntityType, Vec<String>> = BTreeMap::new();
        let mut rows = stmt.query(params![event_id])?;

        while let Some(row) = rows.next()? {
            let entity_id: String = row.get(0)?;
            let entity_type_str: String = row.get(1)?;
            let entity_type = match entity_type_str.as_str() {
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
                _ => continue, // 未知类型跳过
            };
            map.entry(entity_type).or_default().push(entity_id);
        }

        Ok(EventEntities(map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::run_migrations;
    use crate::models::{
        DerivedRefs, EventEntities, EventRelations, EventSource, EventTime, GraphHints, RawRefs,
    };
    use chrono::Utc;

    fn create_test_db() -> Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    /// 创建测试用 Event
    fn create_test_event(id: &str, type_: &str) -> Event {
        Event {
            schema: "event/v1".to_string(),
            id: id.to_string(),
            type_: type_.to_string(),
            subtype: None,
            time: EventTime {
                start: Utc::now(),
                end: None,
                timezone: "UTC".to_string(),
            },
            created_at: Some(Utc::now()),
            ingested_at: Some(Utc::now()),
            source: EventSource {
                device: Some("test".to_string()),
                channel: None,
                capture_agent: None,
            },
            confidence: 0.9,
            entities: EventEntities(BTreeMap::new()),
            tags: vec!["test".to_string(), "unit".to_string()],
            raw_refs: RawRefs::default(),
            derived_refs: DerivedRefs::default(),
            ai: crate::models::EventAi {
                summary: Some("Test summary".to_string()),
                extended: None,
                topics: vec!["topic1".to_string()],
                sentiment: None,
                extraction_version: Some(1),
            },
            relations: EventRelations::default(),
            graph_hints: GraphHints {
                importance: Some(0.5),
                recurrence: false,
            },
            schema_version: 1,
        }
    }

    #[test]
    fn test_upsert_new_event() {
        let conn = create_test_db();
        let repo = EventRepository::new(&conn);

        let event = create_test_event("evt-1", "note");
        let result = repo.upsert(&event);
        assert!(result.is_ok());

        // 验证插入成功
        let found = repo.find_by_id("evt-1").unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.type_, "note");
    }

    #[test]
    fn test_upsert_updates_existing() {
        let conn = create_test_db();
        let repo = EventRepository::new(&conn);

        // 第一次插入
        let event = create_test_event("evt-1", "note");
        repo.upsert(&event).unwrap();

        // 更新（修改 type）
        let event_updated = create_test_event("evt-1", "task");
        repo.upsert(&event_updated).unwrap();

        // 验证已更新
        let found = repo.find_by_id("evt-1").unwrap().unwrap();
        assert_eq!(found.type_, "task");
    }

    #[test]
    fn test_upsert_updates_fts() {
        let conn = create_test_db();
        let repo = EventRepository::new(&conn);

        let event = create_test_event("evt-fts", "note");
        repo.upsert(&event).unwrap();

        // 验证 FTS 有数据
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM events_fts WHERE id = 'evt-fts'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_upsert_updates_tags() {
        let conn = create_test_db();
        let repo = EventRepository::new(&conn);

        let event = create_test_event("evt-tags", "note");
        repo.upsert(&event).unwrap();

        // 验证 tags 插入
        let tags: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT tag FROM tags WHERE event_id = 'evt-tags' ORDER BY tag")
                .unwrap();
            let rows = stmt.query([]).unwrap();
            rows.mapped(|row| row.get(0))
                .filter_map(|r| r.ok())
                .collect()
        };
        assert_eq!(tags, vec!["test", "unit"]);
    }

    #[test]
    fn test_delete_event() {
        let conn = create_test_db();
        let repo = EventRepository::new(&conn);

        let event = create_test_event("evt-del", "note");
        repo.upsert(&event).unwrap();

        // 验证存在
        assert!(repo.find_by_id("evt-del").unwrap().is_some());

        // 删除
        repo.delete("evt-del").unwrap();

        // 验证不存在
        assert!(repo.find_by_id("evt-del").unwrap().is_none());
    }

    #[test]
    fn test_delete_cascades_to_fts() {
        let conn = create_test_db();
        let repo = EventRepository::new(&conn);

        let event = create_test_event("evt-cascade", "note");
        repo.upsert(&event).unwrap();

        repo.delete("evt-cascade").unwrap();

        // 验证 FTS 记录也被删除
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM events_fts WHERE id = 'evt-cascade'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_delete_cascades_to_tags() {
        let conn = create_test_db();
        let repo = EventRepository::new(&conn);

        let event = create_test_event("evt-tag-cascade", "note");
        repo.upsert(&event).unwrap();

        repo.delete("evt-tag-cascade").unwrap();

        // 验证 tags 也被删除
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tags WHERE event_id = 'evt-tag-cascade'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_find_by_id_not_found() {
        let conn = create_test_db();
        let repo = EventRepository::new(&conn);

        let result = repo.find_by_id("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_find_by_id_loads_entities() {
        let conn = create_test_db();
        let repo = EventRepository::new(&conn);

        // 创建事件
        let mut event = create_test_event("evt-with-entity", "meeting");
        // 添加实体关联
        let mut entities_map: BTreeMap<EntityType, Vec<String>> = BTreeMap::new();
        entities_map.insert(EntityType::Person, vec!["ent-person-1".to_string()]);
        entities_map.insert(EntityType::Place, vec!["ent-place-1".to_string()]);
        event.entities = EventEntities(entities_map);

        repo.upsert(&event).unwrap();

        // 验证实体关联被加载
        let found = repo.find_by_id("evt-with-entity").unwrap().unwrap();
        assert_eq!(found.entities.0.len(), 2);
        assert!(found.entities.0.contains_key(&EntityType::Person));
    }

    #[test]
    fn test_find_by_time_range() {
        let conn = create_test_db();
        let repo = EventRepository::new(&conn);

        let now = Utc::now();

        // 创建多个事件
        for i in 0..5 {
            let mut event = Event {
                id: format!("evt-range-{}", i),
                type_: "note".to_string(),
                time: EventTime {
                    start: now + chrono::Duration::hours(i),
                    end: None,
                    timezone: "UTC".to_string(),
                },
                ..create_test_event("dummy", "note")
            };
            event.id = format!("evt-range-{}", i);
            repo.upsert(&event).unwrap();
        }

        // 查询范围
        let events = repo
            .find_by_time_range(
                now + chrono::Duration::hours(1),
                now + chrono::Duration::hours(3),
            )
            .unwrap();

        assert_eq!(events.len(), 3);
    }

    #[test]
    fn test_all_events() {
        let conn = create_test_db();
        let repo = EventRepository::new(&conn);

        // 插入多个事件
        for i in 0..3 {
            let event = create_test_event(&format!("evt-all-{}", i), "note");
            repo.upsert(&event).unwrap();
        }

        let all = repo.all().unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_all_events_ordered_by_time_desc() {
        let conn = create_test_db();
        let repo = EventRepository::new(&conn);

        let base = Utc::now();

        // 按时间顺序插入
        for (i, time_offset) in [(2, 2000), (0, 0), (1, 1000)].iter() {
            let event = Event {
                id: format!("evt-order-{}", i),
                type_: "note".to_string(),
                time: EventTime {
                    start: base + chrono::Duration::seconds(*time_offset),
                    end: None,
                    timezone: "UTC".to_string(),
                },
                ..create_test_event("dummy", "note")
            };
            repo.upsert(&event).unwrap();
        }

        let all = repo.all().unwrap();
        // 最新的（evt-order-2）应该在最前面
        assert_eq!(all[0].id, "evt-order-2");
    }

    #[test]
    fn test_fts_search() {
        let conn = create_test_db();
        let repo = EventRepository::new(&conn);

        // 创建包含特定关键词的事件
        let event = Event {
            id: "evt-search".to_string(),
            type_: "meeting".to_string(),
            ai: crate::models::EventAi {
                summary: Some("Discussed the quarterly planning meeting".to_string()),
                extended: None,
                topics: vec!["planning".to_string()],
                sentiment: None,
                extraction_version: Some(1),
            },
            ..create_test_event("dummy", "meeting")
        };
        repo.upsert(&event).unwrap();

        // FTS 搜索摘要中的词
        let results = repo.search("quarterly").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "evt-search");

        // 搜索标签
        let results = repo.search("test").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_fts_search_no_results() {
        let conn = create_test_db();
        let repo = EventRepository::new(&conn);

        let event = create_test_event("evt-empty", "note");
        repo.upsert(&event).unwrap();

        let results = repo.search("nonexistent_keyword_xyz").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_find_entities_by_event_id() {
        let conn = create_test_db();
        let repo = EventRepository::new(&conn);

        // 关闭外键检查（因为 entity 可能还不存在）
        conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();

        // 插入实体
        conn.execute(
            "INSERT INTO entities (id, type, label) VALUES ('ent-p1', 'person', 'Person1')",
            [],
        )
        .unwrap();

        // 建立关联
        conn.execute(
            "INSERT INTO event_entities (event_id, entity_id, entity_type) VALUES ('evt-e1', 'ent-p1', 'person')",
            [],
        )
        .unwrap();

        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();

        let entities = repo.find_entities_by_event_id("evt-e1").unwrap();
        assert_eq!(entities.0.len(), 1);
        assert!(entities.0.contains_key(&EntityType::Person));
        assert_eq!(
            entities.0.get(&EntityType::Person),
            Some(&vec!["ent-p1".to_string()])
        );
    }

    #[test]
    fn test_find_entities_skips_unknown_type() {
        let conn = create_test_db();
        let repo = EventRepository::new(&conn);

        // 关闭外键检查
        conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();

        // 插入关联但类型未知
        conn.execute(
            "INSERT INTO event_entities (event_id, entity_id, entity_type) VALUES ('evt-unknown', 'ent-u1', 'unknown_type')",
            [],
        )
        .unwrap();

        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();

        let entities = repo.find_entities_by_event_id("evt-unknown").unwrap();
        // 未知类型被跳过，所以 map 为空
        assert!(entities.0.is_empty());
    }

    #[test]
    fn test_row_to_event_tags_empty() {
        let conn = create_test_db();
        let repo = EventRepository::new(&conn);

        let event = create_test_event("evt-no-tags", "note");
        repo.upsert(&event).unwrap();

        // 向 tags 表添加标签（模拟直接 SQL 插入）
        conn.execute(
            "INSERT INTO tags (event_id, tag) VALUES ('evt-no-tags', 'sql-tag')",
            [],
        )
        .unwrap();

        // 但从 repo 读取时 tags 仍然为空
        let found = repo.find_by_id("evt-no-tags").unwrap().unwrap();
        assert!(found.tags.is_empty()); // DB 不加载 tags
    }

    #[test]
    fn test_time_range_handles_null_end() {
        let conn = create_test_db();
        let repo = EventRepository::new(&conn);

        let event = create_test_event("evt-no-end", "note");
        repo.upsert(&event).unwrap();

        let found = repo.find_by_id("evt-no-end").unwrap().unwrap();
        assert!(found.time.end.is_none());
    }

    #[test]
    fn test_recurrence_conversion() {
        let conn = create_test_db();
        let repo = EventRepository::new(&conn);

        // 直接插入 recurrence = 1
        conn.execute(
            "INSERT INTO events (id, time_start, type, recurrence) VALUES ('evt-rec', 0, 'note', 1)",
            [],
        )
        .unwrap();

        let found = repo.find_by_id("evt-rec").unwrap().unwrap();
        assert!(found.graph_hints.recurrence);

        // 直接插入 recurrence = 0
        conn.execute(
            "INSERT INTO events (id, time_start, type, recurrence) VALUES ('evt-not-rec', 0, 'note', 0)",
            [],
        )
        .unwrap();

        let found = repo.find_by_id("evt-not-rec").unwrap().unwrap();
        assert!(!found.graph_hints.recurrence);
    }

    #[test]
    fn test_ai_topics_json_roundtrip() {
        let conn = create_test_db();
        let repo = EventRepository::new(&conn);

        let event = create_test_event("evt-topics", "note");
        repo.upsert(&event).unwrap();

        // 验证 JSON 存储
        let topics_raw: String = conn
            .query_row(
                "SELECT ai_topics FROM events WHERE id = 'evt-topics'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let parsed: Vec<String> = serde_json::from_str(&topics_raw).unwrap();
        assert_eq!(parsed, vec!["topic1"]);
    }

    #[test]
    fn test_foreign_keys_off_allows_missing_entity() {
        let conn = create_test_db();
        let repo = EventRepository::new(&conn);

        // 创建一个关联到不存在实体的 event
        let mut event = create_test_event("evt-orphan", "note");
        let mut entities_map: BTreeMap<EntityType, Vec<String>> = BTreeMap::new();
        entities_map.insert(EntityType::Person, vec!["nonexistent-entity".to_string()]);
        event.entities = EventEntities(entities_map);

        // 应该成功（外键检查已关闭）
        let result = repo.upsert(&event);
        assert!(result.is_ok());

        // 验证关联被插入
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM event_entities WHERE event_id = 'evt-orphan'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_upsert_event_with_entity() {
        let conn = create_test_db();
        let repo = EventRepository::new(&conn);

        // 先创建实体
        conn.execute(
            "INSERT INTO entities (id, type, label) VALUES ('ent-for-event', 'person', 'Person')",
            [],
        )
        .unwrap();

        // 创建关联实体的事件
        let mut event = create_test_event("evt-linked", "meeting");
        let mut entities_map: BTreeMap<EntityType, Vec<String>> = BTreeMap::new();
        entities_map.insert(EntityType::Person, vec!["ent-for-event".to_string()]);
        event.entities = EventEntities(entities_map);

        repo.upsert(&event).unwrap();

        // 验证关联
        let found = repo.find_by_id("evt-linked").unwrap().unwrap();
        assert_eq!(
            found.entities.0.get(&EntityType::Person),
            Some(&vec!["ent-for-event".to_string()])
        );
    }

    #[test]
    fn test_multiple_entities_same_type() {
        let conn = create_test_db();
        let repo = EventRepository::new(&conn);

        let mut event = create_test_event("evt-multi-entity", "meeting");
        let mut entities_map: BTreeMap<EntityType, Vec<String>> = BTreeMap::new();
        entities_map.insert(
            EntityType::Person,
            vec!["person-1".to_string(), "person-2".to_string()],
        );
        event.entities = EventEntities(entities_map);

        repo.upsert(&event).unwrap();

        let found = repo.find_by_id("evt-multi-entity").unwrap().unwrap();
        assert_eq!(
            found.entities.0.get(&EntityType::Person),
            Some(&vec!["person-1".to_string(), "person-2".to_string()])
        );
    }
}
