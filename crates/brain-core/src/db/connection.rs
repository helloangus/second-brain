//! Database connection management
//!
//! # 线程安全设计
//!
//! [`Database`] 使用 [`std::sync::Mutex`] 包装 rusqlite 的 [`Connection`]，
//! 确保在多线程环境下同一时刻只有一个线程能执行写操作。
//!
//! SQLite 本身支持并发读（通过共享锁），但写操作需要独占访问。
//! [`Mutex`] 在这里的作用是序列化所有访问，而不是防止并发读。
//!
//! # 生命周期管理
//!
//! Database 的生命周期由调用方控制，通常是 `main` 函数或 `App` 结构体。
//! 当 Database 被 drop 时，其内部的 Connection 也会自动关闭。
//!
//! # 示例
//!
//! ```ignore
//! let db = Database::open("brain.db")?;
//! let conn = db.connection();
//! let event_repo = EventRepository::new(&conn);
//! let entity_repo = EntityRepository::new(&conn);
//! ```

use crate::error::Error;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

use super::run_migrations;

/// 线程安全的数据库包装器
///
/// 使用 `Mutex<Connection>` 实现线程安全，所有对数据库的访问
/// 都必须先获取锁。对于读操作多的场景，这可能会有一定性能影响，
/// 但 SQLite 的读性能很高，且整个 Second Brain 是本地优先架构，
/// 并发压力不大。
///
/// # 类型定义
///
/// `conn: Mutex<Connection>` - 互斥锁保护的 SQLite 连接
///
/// # 线程安全注解
///
/// `conn` 字段是 `Mutex<Connection>`，实现了 `Send + Sync`，
/// 因此 `Database` 本身也是 `Send + Sync`，可以在多线程间共享。
pub struct Database {
    /// 互斥锁保护的 SQLite 连接
    /// 使用 Mutex 而不是 RwLock 是因为：
    /// 1. SQLite 写操作本身就需要独占锁
    /// 2. rusqlite 的 Connection 不是 Send+Sync，直接用锁包装更简单
    conn: Mutex<Connection>,
}

impl Database {
    /// 打开或创建数据库
    ///
    /// # 参数
    ///
    /// * `path` - 数据库文件路径，支持任意实现 [`AsRef<Path>`] 的类型
    ///   如 `&str`, `String`, `&Path`, `PathBuf`
    ///
    /// # 行为
    ///
    /// 1. 调用 [`Connection::open`] 打开或创建 SQLite 文件
    /// 2. 创建 Mutex 包装
    /// 3. 调用 [`Database::init`] 执行 migrations 建表
    ///
    /// # 错误
    ///
    /// 如果文件无法打开或 migrations 失败，返回 [`Error`]
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let db = Database::open("brain.db")?;
    /// ```
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let conn = Connection::open(path.as_ref())?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init()?;
        Ok(db)
    }

    /// 执行数据库初始化，包括运行 migrations
    ///
    /// # 实现细节
    ///
    /// 1. 获取锁（阻塞等待）
    /// 2. 调用 [`run_migrations`] 创建所有表
    ///
    /// 注意：SQLite 的 `CREATE TABLE IF NOT EXISTS` 是幂等的，
    /// 即使表已存在也不会报错。
    fn init(&self) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        run_migrations(&conn)?;
        Ok(())
    }

    /// 获取数据库连接的引用
    ///
    /// # 返回值
    ///
    /// 返回 [`MutexGuard`]`<`[`Connection`]`>`，
    /// 调用方在用完后（MutexGuard 被 drop）锁自动释放。
    ///
    /// # 线程安全
    ///
    /// 同一时刻只有一个线程能持有锁，其他试图获取锁的线程会阻塞。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let guard = db.connection();
    /// let repo = EventRepository::new(&guard);  // &Connection
    /// // guard 超出作用域后，锁自动释放
    /// ```
    pub fn connection(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// 测试 Database::open 能正常创建数据库文件
    #[test]
    fn test_database_open_creates_file() {
        let tmp_dir = std::env::temp_dir().join("brain_test_db_open");
        let _ = fs::remove_dir_all(&tmp_dir);
        fs::create_dir_all(&tmp_dir).unwrap();
        let db_path = tmp_dir.join("test.db");

        let db = Database::open(&db_path);
        assert!(db.is_ok(), "Database::open should succeed");
        assert!(db_path.exists(), "Database file should be created");

        drop(db);
        let _ = fs::remove_dir_all(&tmp_dir);
    }

    /// 测试 Database::open 对已有数据库不会重复建表
    #[test]
    fn test_database_open_existing() {
        let tmp_dir = std::env::temp_dir().join("brain_test_db_existing");
        let _ = fs::remove_dir_all(&tmp_dir);
        fs::create_dir_all(&tmp_dir).unwrap();
        let db_path = tmp_dir.join("test.db");

        // 第一次打开
        {
            let db1 = Database::open(&db_path).unwrap();
            let conn1 = db1.connection();
            conn1
                .execute(
                    "INSERT INTO events (id, time_start, type) VALUES ('test', 0, 'note')",
                    [],
                )
                .unwrap();
        } // db1 和 conn1 在这里被 drop

        // 第二次打开同一个文件
        {
            let db2 = Database::open(&db_path).unwrap();
            let conn2 = db2.connection();

            let count: i64 = conn2
                .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))
                .unwrap();
            assert_eq!(count, 1, "Existing data should persist after reopening");

            let table_exists: i64 = conn2
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='events'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(table_exists, 1, "events table should still exist");
        }

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    /// 测试 connection() 返回的锁在 drop 后自动释放
    #[test]
    fn test_connection_lock_released() {
        let tmp_dir = std::env::temp_dir().join("brain_test_db_lock");
        let _ = fs::remove_dir_all(&tmp_dir);
        fs::create_dir_all(&tmp_dir).unwrap();
        let db_path = tmp_dir.join("test.db");

        let db = Database::open(&db_path).unwrap();

        {
            let guard1 = db.connection();
            assert!(guard1.query_row("SELECT 1", [], |_| Ok(())).is_ok());
        } // guard1 在这里被 drop

        {
            let guard2 = db.connection();
            assert!(guard2.query_row("SELECT 1", [], |_| Ok(())).is_ok());
        }

        drop(db);
        let _ = fs::remove_dir_all(&tmp_dir);
    }

    /// 测试 Mutex 正确工作：同一线程重复获取锁会死锁
    /// 注意：这是理论上会死锁的场景，ignore 防止测试卡死
    #[test]
    #[ignore]
    fn test_mutex_blocks_same_thread() {
        let tmp_dir = std::env::temp_dir().join("brain_test_db_mutex");
        let _ = fs::remove_dir_all(&tmp_dir);
        fs::create_dir_all(&tmp_dir).unwrap();
        let db_path = tmp_dir.join("test.db");

        let db = Database::open(&db_path).unwrap();

        let _guard1 = db.connection();
        let _guard2 = db.connection(); // 死锁

        drop(_guard2);
        drop(_guard1);
        drop(db);
        let _ = fs::remove_dir_all(&tmp_dir);
    }

    /// 测试 Database 实现了 Send + Sync（线程安全标记）
    #[test]
    fn test_database_thread_safe() {
        let tmp_dir = std::env::temp_dir().join("brain_test_db_thread");
        let _ = fs::remove_dir_all(&tmp_dir);
        fs::create_dir_all(&tmp_dir).unwrap();
        let db_path = tmp_dir.join("test.db");

        let db = Database::open(&db_path).unwrap();

        let handle = std::thread::spawn(move || {
            let conn = db.connection();
            let result: Result<i64, _> =
                conn.query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0));
            result
        });

        let result = handle.join().unwrap();
        assert!(result.is_ok());

        // db 在闭包中被 move，所以在这里已经不可用了
        // 当 thread spawn 的线程结束时，db 会被自动 drop
        let _ = fs::remove_dir_all(&tmp_dir);
    }
}
