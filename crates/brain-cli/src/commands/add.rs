//! Add command

use brain_core::markdown::EventSerializer;
use brain_core::{
    BrainConfig, Database, DerivedRefs, Event, EventAi, EventEntities, EventRelations, EventSource,
    EventTime, EventType, GraphHints, RawRefs,
};
use chrono::Utc;
use std::fs;

pub fn execute(
    config: &BrainConfig,
    type_: &str,
    summary: &str,
    tags: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let now = Utc::now();

    // Parse event type
    let event_type = match type_.to_lowercase().as_str() {
        "meeting" => EventType::Meeting,
        "photo" => EventType::Photo,
        "note" => EventType::Note,
        "activity" => EventType::Activity,
        "research" => EventType::Research,
        "reading" => EventType::Reading,
        "exercise" => EventType::Exercise,
        "meal" => EventType::Meal,
        "work" => EventType::Work,
        _ => EventType::Other,
    };

    // Generate event ID
    let id = Event::generate_id();

    // Create event
    let event = Event {
        schema: "event/v1".to_string(),
        id: id.clone(),
        type_: event_type,
        subtype: None,
        time: EventTime {
            start: now,
            end: None,
            timezone: "UTC".to_string(),
        },
        created_at: Some(now),
        ingested_at: Some(now),
        source: EventSource {
            device: Some("PC".to_string()),
            channel: Some("CLI".to_string()),
            capture_agent: Some("manual_entry".to_string()),
        },
        confidence: 0.5,
        entities: EventEntities::default(),
        tags: tags.to_vec(),
        raw_refs: RawRefs::default(),
        derived_refs: DerivedRefs::default(),
        ai: EventAi {
            summary: Some(summary.to_string()),
            extended: None,
            topics: Vec::new(),
            sentiment: None,
            extraction_version: None,
        },
        relations: EventRelations::default(),
        graph_hints: GraphHints::default(),
        schema_version: 1,
    };

    // Serialize to markdown
    let serializer = EventSerializer;
    let markdown = serializer.serialize(&event)?;

    // Determine file path
    let year = now.format("%Y");
    let month = now.format("%m");
    let events_dir = config
        .events_path
        .join(year.to_string())
        .join(month.to_string());
    fs::create_dir_all(&events_dir)?;

    let file_path = events_dir.join(format!("{}.md", id));

    // Write file
    fs::write(&file_path, &markdown)?;

    // Also insert into database
    let db = Database::open(&config.db_path)?;
    let conn = db.connection();
    let repo = brain_core::EventRepository::new(&conn);
    repo.upsert(&event)?;

    println!("Created event: {}", id);
    println!("File: {}", file_path.display());

    Ok(())
}
