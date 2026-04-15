//! Database module - SQLite index for the Second Brain markdown-first system
//!
//! # 架构设计
//!
//! 这个模块是 Second Brain 的**数据索引层**，而非数据存储层。
//! 所有数据的真相（Truth）存储在 `.md` 文件中，SQLite 只负责建立索引，
//! 提供快速搜索、关联查询和时间范围查询能力。
//!
//! # 核心组件
//!
//! - [`Database`] - SQLite 连接管理，线程安全包装
//! - [`EventRepository`] - 事件增删改查，含 FTS 全文搜索
//! - [`EntityRepository`] - 实体（人物/地点/项目等）管理
//! - [`TagRepository`] - 标签查询，支持中文标签翻译
//!
//! # 数据流向
//!
//! ```text
//! .md 文件变化 → brain-indexerd 解析 → Repository.upsert() → SQLite
//! CLI 查询    → Repository 查询方法 → 返回 Event/Entity 模型
//! ```

mod connection;
mod entity_repo;
mod event_repo;
mod migrations;
mod tag_repo;

pub use connection::Database;
pub use entity_repo::EntityRepository;
pub use event_repo::EventRepository;
pub use migrations::run_migrations;
pub use tag_repo::TagRepository;

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证模块导出是否完整
    #[test]
    fn test_module_exports() {
        // 验证类型可以构造（编译通过即说明导出完整）
        fn _check_database(_: &Database) {}
        fn _check_event_repo(_: &EventRepository) {}
        fn _check_entity_repo(_: &EntityRepository) {}
        fn _check_tag_repo(_: &TagRepository) {}
    }

    /// 验证 run_migrations 可以调用
    #[test]
    fn test_run_migrations_is_callable() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let result = run_migrations(&conn);
        assert!(result.is_ok());
    }
}
