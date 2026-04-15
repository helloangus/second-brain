//! Tag repository
//!
//! # 职责
//!
//! [`TagRepository`] 负责标签的**只读查询**。
//! 标签的写入（upsert）是通过 [`EventRepository::upsert`] 间接完成的，
//! 事件删除时也会级联删除关联的标签。
//!
//! # 与 EventRepository 的关系
//!
//! ```text
//! upsert event → EventRepository.update_tags() → 写入 tags 表
//! delete event → EventRepository.delete() → 级联删除 tags 表
//! get_for_event → TagRepository.get_for_event() → 读取 tags 表
//! ```
//!
//! TagRepository 不负责写入，只负责查询。
//!
//! # 中英文标签翻译
//!
//! [`find_by_tag`] 方法支持中文标签到英文 key 的自动翻译。
//! 当 `DictSet`（字典集）可用时，会尝试将中文标签翻译成英文再查询。
//!
//! 例如：
//! - 用户搜索 `"会议"` → DictSet 翻译成 `"meeting"` → 查询 `tag = 'meeting'`
//! - 用户搜索 `"meeting"` → 直接查询 `tag = 'meeting'`
//!
//! 这使得 CLI 可以接受中文输入，同时内部统一用英文存储。

use crate::error::Error;
use crate::DictSet;
use rusqlite::{params, Connection};

/// 标签仓库
///
/// 只负责查询，不负责写入。
///
/// # 生命周期
///
/// `conn: &'a Connection` - 借用数据库连接，生命周期由调用方管理。
pub struct TagRepository<'a> {
    conn: &'a Connection,
}

impl<'a> TagRepository<'a> {
    /// 创建 TagRepository
    ///
    /// # 参数
    ///
    /// * `conn` - 数据库连接的引用（不是所有权）
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let conn = db.connection();
    /// let tag_repo = TagRepository::new(&conn);
    /// ```
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// 获取指定事件的所有标签
    ///
    /// # 参数
    ///
    /// * `event_id` - 事件 ID
    ///
    /// # 返回值
    ///
    /// 标签列表（按插入顺序），如果没有标签则返回空 Vec
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let tags = tag_repo.get_for_event("evt-20260414-001")?;
    /// assert!(tags.contains(&"work".to_string()));
    /// ```
    pub fn get_for_event(&self, event_id: &str) -> Result<Vec<String>, Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT tag FROM tags WHERE event_id = ?1")?;

        let mut rows = stmt.query(params![event_id])?;
        let mut tags = Vec::new();

        while let Some(row) = rows.next()? {
            tags.push(row.get(0)?);
        }

        Ok(tags)
    }

    /// 获取所有唯一的标签
    ///
    /// # 返回值
    ///
    /// 按字母顺序排序的去重标签列表
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let all_tags = tag_repo.all()?;
    /// ```
    pub fn all(&self) -> Result<Vec<String>, Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT tag FROM tags ORDER BY tag")?;

        let mut rows = stmt.query([])?;
        let mut tags = Vec::new();

        while let Some(row) = rows.next()? {
            tags.push(row.get(0)?);
        }

        Ok(tags)
    }

    /// 根据标签搜索关联的事件 ID
    ///
    /// # 参数
    ///
    /// * `tag` - 标签名（可以是中文或英文）
    /// * `dict_set` - 可选的字典集，用于中文到英文的翻译
    ///
    /// # 搜索逻辑
    ///
    /// 1. 先尝试直接匹配（英文 key）
    /// 2. 如果没有匹配到，且提供了 `dict_set`，则尝试翻译后再匹配
    ///
    /// # 返回值
    ///
    /// 关联的事件 ID 列表
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 直接搜英文
    /// let ids = tag_repo.find_by_tag("meeting", None)?;
    ///
    /// // 搜中文，自动翻译
    /// let ids = tag_repo.find_by_tag("会议", Some(&dict_set))?;
    /// ```
    pub fn find_by_tag(&self, tag: &str, dict_set: Option<&DictSet>) -> Result<Vec<String>, Error> {
        // 第一步：尝试直接匹配（英文 key）
        let mut event_ids = self.find_by_tag_direct(tag)?;

        // 第二步：如果没匹配到，尝试中文翻译
        if event_ids.is_empty() {
            if let Some(dict) = dict_set {
                if let Some(entry) = dict.find_entry("tags", tag) {
                    event_ids = self.find_by_tag_direct(&entry.key)?;
                }
            }
        }

        Ok(event_ids)
    }

    /// 内部方法：直接按标签名查询事件 ID
    ///
    /// 不做任何翻译，直接执行 SQL 查询。
    ///
    /// # 参数
    ///
    /// * `tag` - 标签名（精确匹配）
    fn find_by_tag_direct(&self, tag: &str) -> Result<Vec<String>, Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT event_id FROM tags WHERE tag = ?1")?;

        let mut rows = stmt.query(params![tag])?;
        let mut event_ids = Vec::new();

        while let Some(row) = rows.next()? {
            event_ids.push(row.get(0)?);
        }

        Ok(event_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::run_migrations;

    #[test]
    fn test_get_for_event() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        // 插入事件
        conn.execute(
            "INSERT INTO events (id, time_start, type) VALUES ('evt-1', 0, 'note')",
            [],
        )
        .unwrap();

        // 插入标签
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

        let tag_repo = TagRepository::new(&conn);
        let tags = tag_repo.get_for_event("evt-1").unwrap();

        assert_eq!(tags.len(), 2);
        assert!(tags.contains(&"work".to_string()));
        assert!(tags.contains(&"meeting".to_string()));
    }

    /// 测试 get_for_event 返回空 Vec 当事件无标签
    #[test]
    fn test_get_for_event_empty() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();

        conn.execute(
            "INSERT INTO events (id, time_start, type) VALUES ('evt-no-tags', 0, 'note')",
            [],
        )
        .unwrap();

        let tag_repo = TagRepository::new(&conn);
        let tags = tag_repo.get_for_event("evt-no-tags").unwrap();

        assert!(tags.is_empty());
    }

    /// 测试 all() 返回所有唯一标签
    #[test]
    fn test_all_tags() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();

        // 插入多个事件的标签
        conn.execute(
            "INSERT INTO events (id, time_start, type) VALUES ('e1', 0, 'note')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO events (id, time_start, type) VALUES ('e2', 0, 'note')",
            [],
        )
        .unwrap();

        conn.execute("INSERT INTO tags (event_id, tag) VALUES ('e1', 'work')", [])
            .unwrap();
        conn.execute(
            "INSERT INTO tags (event_id, tag) VALUES ('e1', 'meeting')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tags (event_id, tag) VALUES ('e2', 'work')", // 重复
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tags (event_id, tag) VALUES ('e2', 'personal')",
            [],
        )
        .unwrap();

        let tag_repo = TagRepository::new(&conn);
        let all_tags = tag_repo.all().unwrap();

        // 应该去重并排序
        assert_eq!(all_tags.len(), 3);
        assert_eq!(all_tags[0], "meeting"); // 字母序
        assert_eq!(all_tags[1], "personal");
        assert_eq!(all_tags[2], "work");
    }

    /// 测试 find_by_tag_direct 精确匹配
    #[test]
    fn test_find_by_tag_direct() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();

        conn.execute(
            "INSERT INTO events (id, time_start, type) VALUES ('e1', 0, 'note')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tags (event_id, tag) VALUES ('e1', 'meeting')",
            [],
        )
        .unwrap();

        let tag_repo = TagRepository::new(&conn);
        let ids = tag_repo.find_by_tag_direct("meeting").unwrap();

        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], "e1");
    }

    /// 测试 find_by_tag_direct 无匹配时返回空 Vec
    #[test]
    fn test_find_by_tag_direct_no_match() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();

        let tag_repo = TagRepository::new(&conn);
        let ids = tag_repo.find_by_tag_direct("nonexistent").unwrap();

        assert!(ids.is_empty());
    }

    /// 测试 find_by_tag 不传 dict_set（None）
    #[test]
    fn test_find_by_tag_without_dict() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();

        conn.execute(
            "INSERT INTO events (id, time_start, type) VALUES ('e1', 0, 'note')",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO tags (event_id, tag) VALUES ('e1', 'work')", [])
            .unwrap();

        let tag_repo = TagRepository::new(&conn);

        // 不传 dict_set，直接搜英文
        let ids = tag_repo.find_by_tag("work", None).unwrap();
        assert_eq!(ids.len(), 1);

        // 搜不存在的
        let ids = tag_repo.find_by_tag("nonexistent", None).unwrap();
        assert!(ids.is_empty());
    }

    /// 测试 TagRepository 不修改数据（只读验证）
    #[test]
    fn test_tag_repository_read_only() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();

        conn.execute(
            "INSERT INTO events (id, time_start, type) VALUES ('e1', 0, 'note')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tags (event_id, tag) VALUES ('e1', 'original')",
            [],
        )
        .unwrap();

        let tag_repo = TagRepository::new(&conn);

        // 多次读取结果一致
        let tags1 = tag_repo.get_for_event("e1").unwrap();
        let tags2 = tag_repo.get_for_event("e1").unwrap();
        assert_eq!(tags1, tags2);

        // all() 结果一致
        let all1 = tag_repo.all().unwrap();
        let all2 = tag_repo.all().unwrap();
        assert_eq!(all1, all2);
    }

    /// 测试 TagRepository::new 可以用同一个 conn 创建多个实例
    #[test]
    fn test_multiple_tag_repo_instances() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();

        conn.execute(
            "INSERT INTO events (id, time_start, type) VALUES ('e1', 0, 'note')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tags (event_id, tag) VALUES ('e1', 'shared')",
            [],
        )
        .unwrap();

        // 多个 TagRepository 共享同一个 conn
        let repo1 = TagRepository::new(&conn);
        let repo2 = TagRepository::new(&conn);

        // 两个实例都能正常查询
        let tags1 = repo1.get_for_event("e1").unwrap();
        let tags2 = repo2.get_for_event("e1").unwrap();

        assert_eq!(tags1, tags2);
    }

    /// 测试标签名大小写敏感
    #[test]
    fn test_tag_case_sensitive() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();

        conn.execute(
            "INSERT INTO events (id, time_start, type) VALUES ('e1', 0, 'note')",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO tags (event_id, tag) VALUES ('e1', 'Work')", [])
            .unwrap();

        let tag_repo = TagRepository::new(&conn);

        // 大小写敏感，"work" 搜不到 "Work"
        let ids_lower = tag_repo.find_by_tag_direct("work").unwrap();
        assert!(ids_lower.is_empty());

        let ids_exact = tag_repo.find_by_tag_direct("Work").unwrap();
        assert_eq!(ids_exact.len(), 1);
    }

    /// 测试多个事件有相同标签
    #[test]
    fn test_multiple_events_same_tag() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();

        conn.execute(
            "INSERT INTO events (id, time_start, type) VALUES ('e1', 0, 'note')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO events (id, time_start, type) VALUES ('e2', 0, 'note')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tags (event_id, tag) VALUES ('e1', 'common')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tags (event_id, tag) VALUES ('e2', 'common')",
            [],
        )
        .unwrap();

        let tag_repo = TagRepository::new(&conn);
        let ids = tag_repo.find_by_tag_direct("common").unwrap();

        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"e1".to_string()));
        assert!(ids.contains(&"e2".to_string()));
    }
}
