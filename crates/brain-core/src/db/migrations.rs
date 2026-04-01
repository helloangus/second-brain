//! Database schema migrations

use rusqlite::Connection;
use crate::Error;

const CREATE_TABLES: &str = r#"
-- Events table
CREATE TABLE IF NOT EXISTS events (
    id TEXT PRIMARY KEY,
    schema_version INTEGER DEFAULT 1,

    -- Time information
    time_start INTEGER NOT NULL,
    time_end INTEGER,
    timezone TEXT DEFAULT 'UTC',

    -- Type
    type TEXT NOT NULL,
    subtype TEXT,

    -- Source
    source_device TEXT,
    source_channel TEXT,
    source_capture_agent TEXT,

    -- Status
    status TEXT DEFAULT 'auto',
    confidence REAL DEFAULT 0.5,

    -- AI analysis
    ai_summary TEXT,
    ai_topics TEXT,
    ai_sentiment TEXT,
    extraction_version INTEGER,

    -- Graph hints
    importance REAL,
    recurrence INTEGER DEFAULT 0,

    -- System
    created_at INTEGER,
    ingested_at INTEGER,
    updated_at INTEGER
);

-- Entity table
CREATE TABLE IF NOT EXISTS entities (
    id TEXT PRIMARY KEY,
    schema_version INTEGER DEFAULT 1,

    -- Basic info
    type TEXT NOT NULL,
    label TEXT NOT NULL,
    aliases TEXT,

    -- Status
    status TEXT DEFAULT 'active',
    confidence REAL DEFAULT 0.5,

    -- Classification
    classification_domain TEXT,
    classification_parent TEXT,

    -- Description
    identity_description TEXT,
    summary TEXT,

    -- Multimedia
    images TEXT,
    voices TEXT,
    embeddings_text TEXT,

    -- Links
    links_wikipedia TEXT,
    links_papers TEXT,

    -- Evolution
    merged_from TEXT,
    split_to TEXT,

    -- Metrics
    event_count INTEGER DEFAULT 0,
    last_seen INTEGER,
    activity_score REAL,

    -- System
    created_at INTEGER,
    updated_at INTEGER
);

-- Event-Entity association
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
CREATE TABLE IF NOT EXISTS tags (
    event_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    confidence REAL DEFAULT 1.0,
    PRIMARY KEY (event_id, tag),
    FOREIGN KEY (event_id) REFERENCES events(id)
);

-- Event relations
CREATE TABLE IF NOT EXISTS event_relations (
    event_id TEXT NOT NULL,
    rel_type TEXT NOT NULL,
    target_event_id TEXT NOT NULL,
    PRIMARY KEY (event_id, rel_type, target_event_id),
    FOREIGN KEY (event_id) REFERENCES events(id),
    FOREIGN KEY (target_event_id) REFERENCES events(id)
);

-- Full-text search virtual table
CREATE VIRTUAL TABLE IF NOT EXISTS events_fts USING fts5(
    id,
    ai_summary,
    content
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_events_time_start ON events(time_start);
CREATE INDEX IF NOT EXISTS idx_events_type ON events(type);
CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(type);
CREATE INDEX IF NOT EXISTS idx_entities_label ON entities(label);
"#;

pub fn run_migrations(conn: &Connection) -> Result<(), Error> {
    conn.execute_batch(CREATE_TABLES)?;
    Ok(())
}
