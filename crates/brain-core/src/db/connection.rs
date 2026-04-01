//! Database connection management

use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;
use crate::Error;

use super::run_migrations;

/// Thread-safe database wrapper
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open or create database at the given path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let conn = Connection::open(path.as_ref())?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init()?;
        Ok(db)
    }

    /// Initialize database with migrations
    fn init(&self) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        run_migrations(&conn)?;
        Ok(())
    }

    /// Get a reference to the connection (for repositories)
    pub fn connection(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_open() {
        let tmp_dir = std::env::temp_dir().join("brain_test_db");
        std::fs::create_dir_all(&tmp_dir).unwrap();
        let db_path = &tmp_dir.join("test.db");
        let _db = Database::open(db_path).unwrap();
        assert!(db_path.exists());
        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
