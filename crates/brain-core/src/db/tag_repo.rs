//! Tag repository

use crate::error::Error;
use crate::DictSet;
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

    /// Search events by tag (supports both English key and Chinese translation)
    /// If dict_set is provided, will also try to match Chinese translation
    pub fn find_by_tag(&self, tag: &str, dict_set: Option<&DictSet>) -> Result<Vec<String>, Error> {
        // First try direct match (English key)
        let mut event_ids = self.find_by_tag_direct(tag)?;

        // If no direct match and dict_set provided, try Chinese match
        if event_ids.is_empty() {
            if let Some(dict) = dict_set {
                if let Some(entry) = dict.find_entry("tags", tag) {
                    event_ids = self.find_by_tag_direct(&entry.key)?;
                }
            }
        }

        Ok(event_ids)
    }

    /// Internal: find events by tag directly
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
