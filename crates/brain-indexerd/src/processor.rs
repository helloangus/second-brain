//! Event processor for indexing markdown files

use brain_core::{Database, EventRepository, EntityRepository};
use brain_core::markdown::{EventParser, EntityParser};
use rusqlite::Connection;
use std::fs;
use std::path::Path;
use tracing::{info, warn, error};

/// Event processor for handling file changes
pub struct EventProcessor<'a> {
    db: &'a Database,
}

impl<'a> EventProcessor<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Process a single file (create or update)
    pub fn process_file(&self, path: &Path) -> Result<(), brain_core::Error> {
        let content = fs::read_to_string(path)?;
        let conn = self.db.connection();

        // Determine if it's an event or entity file
        if self.is_event_file(path) {
            self.process_event(&content, &conn)?;
        } else if self.is_entity_file(path) {
            self.process_entity(&content, &conn)?;
        }

        Ok(())
    }

    /// Remove a file from the index
    pub fn remove_file(&self, path: &Path) -> Result<(), brain_core::Error> {
        let conn = self.db.connection();

        if let Some(id) = self.extract_id(path) {
            if self.is_event_file(path) {
                let repo = EventRepository::new(&conn);
                info!("Removing event: {}", id);
                repo.delete(&id)?;
            } else if self.is_entity_file(path) {
                let repo = EntityRepository::new(&conn);
                info!("Removing entity: {}", id);
                repo.delete(&id)?;
            }
        }

        Ok(())
    }

    fn process_event(&self, content: &str, conn: &Connection) -> Result<(), brain_core::Error> {
        match EventParser::parse(content) {
            Ok(event) => {
                let repo = EventRepository::new(conn);
                repo.upsert(&event)?;
                info!("Indexed event: {}", event.id);
            }
            Err(e) => {
                warn!("Failed to parse event file: {}", e);
            }
        }
        Ok(())
    }

    fn process_entity(&self, content: &str, conn: &Connection) -> Result<(), brain_core::Error> {
        match EntityParser::parse(content) {
            Ok(entity) => {
                let repo = EntityRepository::new(conn);
                repo.upsert(&entity)?;
                info!("Indexed entity: {}", entity.id);
            }
            Err(e) => {
                warn!("Failed to parse entity file: {}", e);
            }
        }
        Ok(())
    }

    fn is_event_file(&self, path: &Path) -> bool {
        path.components().any(|c| {
            c.as_os_str() == "events"
        })
    }

    fn is_entity_file(&self, path: &Path) -> bool {
        path.components().any(|c| {
            c.as_os_str() == "entities"
        })
    }

    fn extract_id(&self, path: &Path) -> Option<String> {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    }
}

/// Index all existing files in the events and entities directories
pub fn index_existing_files(
    db: &Database,
    events_path: &Path,
    entities_path: &Path,
) -> Result<(), brain_core::Error> {
    let conn = db.connection();

    // Index events
    if events_path.exists() {
        for entry in walkdir::WalkDir::new(events_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().map(|e| e == "md").unwrap_or(false) {
                match fs::read_to_string(path) {
                    Ok(content) => {
                        match EventParser::parse(&content) {
                            Ok(event) => {
                                let repo = EventRepository::new(&conn);
                                if let Err(e) = repo.upsert(&event) {
                                    error!("Failed to index event {}: {}", event.id, e);
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse {}: {}", path.display(), e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to read {}: {}", path.display(), e);
                    }
                }
            }
        }
    }

    // Index entities
    if entities_path.exists() {
        for entry in walkdir::WalkDir::new(entities_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().map(|e| e == "md").unwrap_or(false) {
                match fs::read_to_string(path) {
                    Ok(content) => {
                        match EntityParser::parse(&content) {
                            Ok(entity) => {
                                let repo = EntityRepository::new(&conn);
                                if let Err(e) = repo.upsert(&entity) {
                                    error!("Failed to index entity {}: {}", entity.id, e);
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse {}: {}", path.display(), e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to read {}: {}", path.display(), e);
                    }
                }
            }
        }
    }

    Ok(())
}
