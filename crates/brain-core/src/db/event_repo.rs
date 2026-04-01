//! Event repository

use crate::error::Error;
use crate::models::{
    DerivedRefs, Event, EventAi, EventEntities, EventRelations, EventSource, EventTime, GraphHints,
    RawRefs,
};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row};

pub struct EventRepository<'a> {
    conn: &'a Connection,
}

impl<'a> EventRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Insert or update an event
    pub fn upsert(&self, event: &Event) -> Result<(), Error> {
        let time_start = event.time.start.timestamp();
        let time_end = event.time.end.map(|t| t.timestamp());

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
                event.type_.to_string(),
                event.subtype,
                event.source.device,
                event.source.channel,
                event.source.capture_agent,
                event.confidence,
                event.ai.summary,
                serde_json::to_string(&event.ai.topics).ok(),
                event.ai.sentiment,
                event.ai.extraction_version,
                event.graph_hints.importance,
                event.graph_hints.recurrence as i32,
                event.created_at.map(|t| t.timestamp()),
                event.ingested_at.map(|t| t.timestamp()),
                chrono::Utc::now().timestamp(),
            ],
        )?;

        // Update FTS
        self.update_fts(event)?;

        // Update entity associations
        self.update_entities(event)?;

        // Update tags
        self.update_tags(event)?;

        Ok(())
    }

    /// Update FTS index for an event
    fn update_fts(&self, event: &Event) -> Result<(), Error> {
        // Delete existing FTS entry
        self.conn
            .execute("DELETE FROM events_fts WHERE id = ?1", params![event.id])?;

        // Insert new FTS entry
        let content = format!(
            "{} {} {}",
            event.id,
            event.ai.summary.as_deref().unwrap_or(""),
            event.tags.join(" ")
        );
        self.conn.execute(
            "INSERT INTO events_fts (id, ai_summary, content) VALUES (?1, ?2, ?3)",
            params![event.id, event.ai.summary, content],
        )?;

        Ok(())
    }

    /// Update event-entity associations
    fn update_entities(&self, event: &Event) -> Result<(), Error> {
        // Delete existing associations
        self.conn.execute(
            "DELETE FROM event_entities WHERE event_id = ?1",
            params![event.id],
        )?;

        // Insert new associations
        let mut insert_stmt = self.conn.prepare(
            "INSERT INTO event_entities (event_id, entity_id, entity_type) VALUES (?1, ?2, ?3)",
        )?;

        for person in &event.entities.people {
            insert_stmt.execute(params![event.id, person, "person"])?;
        }
        for org in &event.entities.organizations {
            insert_stmt.execute(params![event.id, org, "organization"])?;
        }
        for proj in &event.entities.projects {
            insert_stmt.execute(params![event.id, proj, "project"])?;
        }
        for artifact in &event.entities.artifacts {
            insert_stmt.execute(params![event.id, artifact, "artifact"])?;
        }
        for concept in &event.entities.concepts {
            insert_stmt.execute(params![event.id, concept, "concept"])?;
        }
        for topic in &event.entities.topics {
            insert_stmt.execute(params![event.id, topic, "topic"])?;
        }
        for activity in &event.entities.activities {
            insert_stmt.execute(params![event.id, activity, "activity"])?;
        }
        for goal in &event.entities.goals {
            insert_stmt.execute(params![event.id, goal, "goal"])?;
        }
        for skill in &event.entities.skills {
            insert_stmt.execute(params![event.id, skill, "skill"])?;
        }
        for place in &event.entities.places {
            insert_stmt.execute(params![event.id, place, "place"])?;
        }
        for device in &event.entities.devices {
            insert_stmt.execute(params![event.id, device, "device"])?;
        }
        for resource in &event.entities.resources {
            insert_stmt.execute(params![event.id, resource, "resource"])?;
        }
        for memory in &event.entities.memory_clusters {
            insert_stmt.execute(params![event.id, memory, "memory_cluster"])?;
        }
        for state in &event.entities.states {
            insert_stmt.execute(params![event.id, state, "state"])?;
        }

        Ok(())
    }

    /// Update tags for an event
    fn update_tags(&self, event: &Event) -> Result<(), Error> {
        // Delete existing tags
        self.conn
            .execute("DELETE FROM tags WHERE event_id = ?1", params![event.id])?;

        // Insert new tags
        let mut insert_stmt = self
            .conn
            .prepare("INSERT INTO tags (event_id, tag, confidence) VALUES (?1, ?2, 1.0)")?;

        for tag in &event.tags {
            insert_stmt.execute(params![event.id, tag])?;
        }

        Ok(())
    }

    /// Delete an event by ID
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

    /// Find event by ID
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
            Ok(Some(self.row_to_event(row)?))
        } else {
            Ok(None)
        }
    }

    /// Search events by keyword using FTS
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
            events.push(self.row_to_event(row)?);
        }

        Ok(events)
    }

    /// Get events for a time range
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
            events.push(self.row_to_event(row)?);
        }

        Ok(events)
    }

    /// Get all events
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
            events.push(self.row_to_event(row)?);
        }

        Ok(events)
    }

    fn row_to_event(&self, row: &Row) -> Result<Event, Error> {
        let time_start_ts: i64 = row.get(2)?;
        let time_end_ts: Option<i64> = row.get(3)?;
        let timezone: String = row.get(4)?;
        let type_str: String = row.get(5)?;

        let event_type = match type_str.as_str() {
            "meeting" => crate::models::EventType::Meeting,
            "photo" => crate::models::EventType::Photo,
            "note" => crate::models::EventType::Note,
            "activity" => crate::models::EventType::Activity,
            "research" => crate::models::EventType::Research,
            "reading" => crate::models::EventType::Reading,
            "exercise" => crate::models::EventType::Exercise,
            "meal" => crate::models::EventType::Meal,
            "work" => crate::models::EventType::Work,
            _ => crate::models::EventType::Other,
        };

        let ai_topics_str: Option<String> = row.get(12)?;
        let ai_topics: Vec<String> = ai_topics_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        Ok(Event {
            schema: "event/v1".to_string(),
            id: row.get(0)?,
            type_: event_type,
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
            entities: EventEntities::default(),
            tags: Vec::new(),
            raw_refs: RawRefs::default(),
            derived_refs: DerivedRefs::default(),
            ai: EventAi {
                summary: row.get(11)?,
                topics: ai_topics,
                sentiment: row.get(13)?,
                extraction_version: row.get(14)?,
            },
            relations: EventRelations::default(),
            graph_hints: GraphHints {
                importance: row.get(15)?,
                recurrence: row.get::<_, i32>(16)? != 0,
            },
            schema_version: row.get(1)?,
        })
    }
}
