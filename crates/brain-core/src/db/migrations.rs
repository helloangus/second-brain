//! Database schema migrations
//!
//! # Schema 版本管理
//!
//! **当前版本**: Version 1 (无显式版本号字段用于迁移追踪)
//!
//! 这个模块负责创建 Second Brain 的所有数据库表。
//! 所有 `CREATE TABLE` 语句都使用 `IF NOT EXISTS` 子句，
//! 确保幂等性——重复执行不会报错，也不会重复创建。
//!
//! # 表概览
//!
//! | 表名 | 用途 |
//! |------|------|
//! | `events` | 事件主表，存储时间、类型、AI 分析结果 |
//! | `entities` | 实体主表，存储人物/地点/项目等 |
//! | `event_entities` | 事件-实体多对多关联 |
//! | `tags` | 事件标签索引 |
//! | `event_relations` | 事件间关系（reply_to, duplicate_of 等）|
//! | `events_fts` | FTS5 全文搜索虚拟表 |
//! | `logs` | 结构化操作日志 |
//!
//! # 设计决策
//!
//! ## 为什么用 INTEGER 存储时间而不是 TEXT？
//!
//! Unix timestamp (INTEGER) 存储：
//! - 占用空间小（8 字节）
//! - 比较运算快（整数比较 vs 字符串比较）
//! - 时区无关（只存 UTC 毫秒数）
//!
//! 缺点是人类不可读，需要转换：
//! ```ignore
//! let dt = DateTime::from_timestamp(timestamp, 0);
//! ```
//!
//! ## 为什么有些字段用 JSON 字符串而不是独立表？
//!
//! `aliases`, `images`, `voices`, `papers`, `merged_from`, `split_to` 等字段
//! 使用 JSON 数组存储，因为：
//!
//! 1. **数量不固定** - 一个实体可能有 0 个也可能有一百个别名
//! 2. **查询频率低** - 通常按 exact match 或一次性加载
//! 3. **避免 join 开销** - 独立关联表会有额外的 join 查询
//!
//! 代价是无法对 JSON 内部字段建索引，更新时需要整体替换。
//!
//! ## 为什么没有 migration 版本管理？
//!
//! 当前实现只有 `CREATE TABLE IF NOT EXISTS`，没有：
//! - `schema_migrations` 表记录已执行的 migration
//! - `ALTER TABLE` 语句处理 schema 变更
//!
//! 这意味着未来加字段需要手动实现 version check 和 ALTER 逻辑。

use crate::error::Error;
use rusqlite::Connection;

/// 创建所有数据库表的 SQL 语句
///
/// 注意：
/// - 所有表都使用 `IF NOT EXISTS`，可安全重复执行
/// - `events_fts` 是 FTS5 虚拟表，由 SQLite 引擎管理
/// - 外键约束默认开启，但 `event_entities` 插入时可能临时关闭
///   （因为 AI 提取的 entity_id 可能尚未入库）
const CREATE_TABLES: &str = r#"
-- Events table
-- 事件主表，存储所有 life data 的索引
--
-- 设计原则：
-- - time_start 必填，time_end 可选（支持点事件和区间事件）
-- - ai_summary/ai_topics 由 AI pipeline 填充
-- - schema_version 预留用于未来 schema 演进
CREATE TABLE IF NOT EXISTS events (
    id TEXT PRIMARY KEY,
    schema_version INTEGER DEFAULT 1,

    -- Time information
    -- 使用 Unix timestamp (INTEGER) 而非 ISO 8601 字符串
    time_start INTEGER NOT NULL,
    time_end INTEGER,
    timezone TEXT DEFAULT 'UTC',

    -- Type
    -- type 是保留字，所以列名叫 type（SQLite 允许）
    type TEXT NOT NULL,
    subtype TEXT,

    -- Source tracking
    -- 记录数据来源设备、渠道、采集代理
    source_device TEXT,
    source_channel TEXT,
    source_capture_agent TEXT,

    -- Status and confidence
    -- status: auto/manual/verified
    -- confidence: AI 提取的可信度 [0.0, 1.0]
    status TEXT DEFAULT 'auto',
    confidence REAL DEFAULT 0.5,

    -- AI analysis results
    -- ai_topics 存储 JSON 数组，如 ["meeting", "work", "planning"]
    ai_summary TEXT,
    ai_topics TEXT,
    ai_sentiment TEXT,
    extraction_version INTEGER,

    -- Graph hints
    -- importance: 重要性评分 [0.0, 1.0]，用于图谱可视化
    -- recurrence: 是否重复事件（0/1 整数）
    importance REAL,
    recurrence INTEGER DEFAULT 0,

    -- System timestamps
    -- created_at: 事件原始创建时间
    -- ingested_at: 进入 Second Brain 系统的时间
    -- updated_at: 最后修改时间
    created_at INTEGER,
    ingested_at INTEGER,
    updated_at INTEGER
);

-- Entity table
-- 实体主表，存储长生命周期对象（人物/组织/项目等）
--
-- 设计原则：
-- - label 是必填的显示名称
-- - aliases 支持同义词/昵称搜索
-- - classification 支持层级分类（parent 是 JSON 数组）
CREATE TABLE IF NOT EXISTS entities (
    id TEXT PRIMARY KEY,
    schema_version INTEGER DEFAULT 1,

    -- Basic info
    type TEXT NOT NULL,
    label TEXT NOT NULL,
    aliases TEXT,  -- JSON array: ["nickname1", "nickname2", ...]

    -- Status
    status TEXT DEFAULT 'active',  -- active/archived/merged
    confidence REAL DEFAULT 0.5,

    -- Classification hierarchy
    -- domain:顶级分类如 "person", "organization", "project"
    -- parent: 上级分类路径，JSON 数组如 ["work", "open_source"]
    classification_domain TEXT,
    classification_parent TEXT,  -- JSON array

    -- Identity description
    -- description: AI 生成的人物/实体描述
    -- summary: 简短摘要
    identity_description TEXT,
    summary TEXT,

    -- Multimedia references
    -- 均存储为 JSON 数组
    images TEXT,              -- JSON: ["path/to/img1.jpg", ...]
    voices TEXT,              -- JSON: ["path/to/voice1.wav", ...]
    embeddings_text TEXT,    -- 文本嵌入向量序列化

    -- External links
    links_wikipedia TEXT,
    links_papers TEXT,       -- JSON: [{"title": "...", "url": "..."}, ...]

    -- Evolution history
    -- 用于实体合并/分裂追踪
    -- merged_from: 被合并前的前身 ID 列表
    -- split_to: 分裂后的新实体 ID 列表
    merged_from TEXT,  -- JSON array
    split_to TEXT,     -- JSON array

    -- Metrics
    event_count INTEGER DEFAULT 0,
    last_seen INTEGER,       -- Unix timestamp，最后关联事件时间
    activity_score REAL,      -- 活跃度评分 [0.0, 1.0]

    -- System timestamps
    created_at INTEGER,
    updated_at INTEGER
);

-- Event-Entity association
-- 事件与实体的多对多关联表
--
-- 注意：
-- - 一个事件可以关联多个实体（多个人、多个地点）
-- - 一个实体可以被多个事件引用
-- - entity_type 冗余存储，方便按类型查询
-- - relation 字段表示关系类型（可选）
CREATE TABLE IF NOT EXISTS event_entities (
    event_id TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_type TEXT,
    relation TEXT,
    PRIMARY KEY (event_id, entity_id, relation),
    FOREIGN KEY (event_id) REFERENCES events(id),
    FOREIGN KEY (entity_id) REFERENCES entities(id)
);

-- Tags table
-- 事件标签索引，支持一个事件多个标签
--
-- 设计选择：
-- - 主键 (event_id, tag) 确保同一事件无重复标签
-- - confidence 字段预留（目前固定 1.0）
-- - 没有独立 tag 表，tag 字符串直接存储
CREATE TABLE IF NOT EXISTS tags (
    event_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    confidence REAL DEFAULT 1.0,
    PRIMARY KEY (event_id, tag),
    FOREIGN KEY (event_id) REFERENCES events(id)
);

-- Event relations
-- 事件间关系表，支持自引用（事件指向其他事件）
--
-- 使用场景：
-- - reply_to: 回复哪个事件
-- - duplicate_of: 重复事件
-- - triggered_by: 被哪个事件触发
CREATE TABLE IF NOT EXISTS event_relations (
    event_id TEXT NOT NULL,
    rel_type TEXT NOT NULL,
    target_event_id TEXT NOT NULL,
    PRIMARY KEY (event_id, rel_type, target_event_id),
    FOREIGN KEY (event_id) REFERENCES events(id),
    FOREIGN KEY (target_event_id) REFERENCES events(id)
);

-- Full-text search virtual table
-- FTS5 全文搜索引擎
--
-- 重要设计：
-- - content 列由程序拼接 id + ai_summary + tags，不是原始 markdown 内容
-- - 原始内容在 .md 文件中，不在 DB 里
-- - ai_summary 单独一列，可以单独搜索 AI 摘要
CREATE VIRTUAL TABLE IF NOT EXISTS events_fts USING fts5(
    id,
    ai_summary,
    content
);

-- Indexes for events
-- events 表的常用查询字段索引
CREATE INDEX IF NOT EXISTS idx_events_time_start ON events(time_start);
CREATE INDEX IF NOT EXISTS idx_events_type ON events(type);

-- Indexes for entities
-- entities 表的常用查询字段索引
CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(type);
CREATE INDEX IF NOT EXISTS idx_entities_label ON entities(label);

-- Logs table
-- 结构化操作日志表
--
-- 用于追踪：
-- - AI pipeline 处理记录
-- - indexer 更新记录
-- - 用户操作历史
--
-- 设计原则：
-- - level: info/warn/error 三级
-- - log_type + operation + target_type 组合分类
-- - success: 0/1 整数（SQLite 无原生 BOOLEAN）
-- - metadata: JSON 格式额外数据
CREATE TABLE IF NOT EXISTS logs (
    id TEXT PRIMARY KEY,
    timestamp INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    -- Classification
    level TEXT NOT NULL DEFAULT 'info',
    log_type TEXT NOT NULL,
    operation TEXT NOT NULL,
    target_type TEXT NOT NULL,
    target_id TEXT,

    -- Source context
    source_device TEXT,
    source_channel TEXT,
    source_agent TEXT,

    -- Result
    success INTEGER NOT NULL DEFAULT 1,
    error_message TEXT,

    -- Timing (milliseconds)
    duration_ms INTEGER,

    -- Type-specific data (JSON)
    metadata TEXT
);

-- Log indexes
-- 日志表常用查询字段索引
CREATE INDEX IF NOT EXISTS idx_logs_timestamp ON logs(timestamp);
CREATE INDEX IF NOT EXISTS idx_logs_log_type ON logs(log_type);
CREATE INDEX IF NOT EXISTS idx_logs_target_type ON logs(target_type);
CREATE INDEX IF NOT EXISTS idx_logs_level ON logs(level);
CREATE INDEX IF NOT EXISTS idx_logs_target_id ON logs(target_id);
"#;

/// 执行数据库 migrations
///
/// # 行为
///
/// 执行 `CREATE_TABLES` 中定义的所有建表语句。
/// 使用 `execute_batch` 一次性执行整个脚本（更高效）。
///
/// # 幂等性
///
/// 所有表都用 `IF NOT EXISTS`，重复执行不会报错。
///
/// # 错误处理
///
/// 如果 SQL 语法错误或数据库权限问题，返回 [`Error`]。
///
/// # 示例
///
/// ```ignore
/// let conn = Connection::open("brain.db")?;
/// run_migrations(&conn)?;
/// ```
pub fn run_migrations(conn: &Connection) -> Result<(), Error> {
    conn.execute_batch(CREATE_TABLES)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// 创建一个临时内存数据库用于测试
    fn create_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    /// 验证 events 表已创建且结构正确
    #[test]
    fn test_events_table_created() {
        let conn = create_test_db();

        // 验证表存在
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='events'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    /// 验证 events 表能正确插入和查询数据
    #[test]
    fn test_events_insert_and_query() {
        let conn = create_test_db();

        // 插入一条事件
        conn.execute(
            "INSERT INTO events (id, time_start, type, status) VALUES ('evt-1', 1234567890, 'note', 'manual')",
            [],
        )
        .unwrap();

        // 按 ID 查询
        let id: String = conn
            .query_row("SELECT id FROM events WHERE id = 'evt-1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(id, "evt-1");

        // 按类型查询
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM events WHERE type = 'note'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    /// 验证时间戳存储和比较
    #[test]
    fn test_time_range_query() {
        let conn = create_test_db();

        // 插入多个事件
        conn.execute(
            "INSERT INTO events (id, time_start, type) VALUES ('e1', 1000, 'note')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO events (id, time_start, type) VALUES ('e2', 2000, 'note')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO events (id, time_start, type) VALUES ('e3', 3000, 'note')",
            [],
        )
        .unwrap();

        // 范围查询
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM events WHERE time_start >= 1500 AND time_start <= 2500",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1); // 只有 e2 在范围内
    }

    /// 验证 entities 表已创建且支持 JSON 列
    #[test]
    fn test_entities_with_json_fields() {
        let conn = create_test_db();

        // 插入带 JSON 字段的实体
        conn.execute(
            "INSERT INTO entities (id, type, label, aliases, images)
             VALUES ('ent-1', 'person', 'Zhang San', '[\"Zhang\",\"San\"]', '[\"img1.jpg\"]')",
            [],
        )
        .unwrap();

        // 查询并验证 JSON 字段
        let aliases: String = conn
            .query_row(
                "SELECT aliases FROM entities WHERE id = 'ent-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(aliases, "[\"Zhang\",\"San\"]");

        // Rust 端会用 serde_json 反序列化
        let parsed: Vec<String> = serde_json::from_str(&aliases).unwrap();
        assert_eq!(parsed, vec!["Zhang", "San"]);
    }

    /// 验证 event_entities 关联表的外键行为
    #[test]
    fn test_event_entities_association() {
        let conn = create_test_db();

        // 插入事件和实体
        conn.execute(
            "INSERT INTO events (id, time_start, type) VALUES ('evt-1', 0, 'note')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO entities (id, type, label) VALUES ('ent-1', 'person', 'Test')",
            [],
        )
        .unwrap();

        // 建立关联（默认外键检查开启）
        conn.execute(
            "INSERT INTO event_entities (event_id, entity_id, entity_type)
             VALUES ('evt-1', 'ent-1', 'person')",
            [],
        )
        .unwrap();

        // 查询关联
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM event_entities WHERE event_id = 'evt-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    /// 验证 tags 表的去重约束
    #[test]
    fn test_tags_unique_constraint() {
        let conn = create_test_db();

        // 插入事件
        conn.execute(
            "INSERT INTO events (id, time_start, type) VALUES ('evt-1', 0, 'note')",
            [],
        )
        .unwrap();

        // 插入同一个事件的多个标签
        conn.execute(
            "INSERT INTO tags (event_id, tag) VALUES ('evt-1', 'work')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tags (event_id, tag) VALUES ('evt-1', 'meeting')",
            [],
        )
        .unwrap();

        // 验证两个标签都存在
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tags WHERE event_id = 'evt-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);

        // 尝试插入重复标签应该失败（或被静默忽略，取决于 SQLite 配置）
        let _result = conn.execute(
            "INSERT INTO tags (event_id, tag) VALUES ('evt-1', 'work')",
            [],
        );
        // PRIMARY KEY 冲突会导致 error 或 replaced
        // SQLite 默认行为是 REPLACE，这里验证只有 2 条
        let count_after: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tags WHERE event_id = 'evt-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count_after, 2); // 不应该有 3 条
    }

    /// 验证 FTS 虚拟表已创建
    #[test]
    fn test_fts_table_created() {
        let conn = create_test_db();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='events_fts'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    /// 验证 FTS 全文搜索功能
    #[test]
    fn test_fts_search() {
        let conn = create_test_db();

        // 向 FTS 表插入数据
        conn.execute(
            "INSERT INTO events_fts (id, ai_summary, content) VALUES ('e1', 'meeting about work', 'meeting work')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO events_fts (id, ai_summary, content) VALUES ('e2', 'dinner with friends', 'dinner friends')",
            [],
        )
        .unwrap();

        // FTS 搜索
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM events_fts WHERE events_fts MATCH 'meeting'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    /// 验证 logs 表已创建
    #[test]
    fn test_logs_table_created() {
        let conn = create_test_db();

        // 插入一条日志
        conn.execute(
            "INSERT INTO logs (id, level, log_type, operation, target_type, target_id)
             VALUES ('log-1', 'info', 'ai', 'extract', 'event', 'evt-1')",
            [],
        )
        .unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM logs", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    /// 验证索引是否被创建
    #[test]
    fn test_indexes_created() {
        let conn = create_test_db();

        let indexes = vec![
            "idx_events_time_start",
            "idx_events_type",
            "idx_entities_type",
            "idx_entities_label",
            "idx_logs_timestamp",
            "idx_logs_log_type",
            "idx_logs_target_type",
            "idx_logs_level",
            "idx_logs_target_id",
        ];

        for idx_name in indexes {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?",
                    [idx_name],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "Index {} should exist", idx_name);
        }
    }

    /// 验证 run_migrations 是幂等的（重复调用不报错）
    #[test]
    fn test_migrations_idempotent() {
        let conn = Connection::open_in_memory().unwrap();

        // 第一次
        let result1 = run_migrations(&conn);
        assert!(result1.is_ok());

        // 第二次（应该成功，不报错）
        let result2 = run_migrations(&conn);
        assert!(result2.is_ok(), "run_migrations should be idempotent");

        // 第三次也成功
        let result3 = run_migrations(&conn);
        assert!(result3.is_ok());
    }

    /// 验证全部 7 张表都已创建
    #[test]
    fn test_all_tables_exist() {
        let conn = create_test_db();

        let expected_tables = vec![
            "events",
            "entities",
            "event_entities",
            "tags",
            "event_relations",
            "events_fts",
            "logs",
        ];

        for table_name in expected_tables {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
                    [table_name],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "Table {} should exist", table_name);
        }
    }
}
