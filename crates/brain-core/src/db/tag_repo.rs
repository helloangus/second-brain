//! Tag repository

use crate::error::Error;
use rusqlite::{params, Connection};

pub struct TagRepository<'a> {
    conn: &'a Connection,
}

impl<'a> TagRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Get all tags for an event
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

    /// Get all unique tags
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

    /// Search events by tag
    pub fn find_by_tag(&self, tag: &str) -> Result<Vec<String>, Error> {
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
