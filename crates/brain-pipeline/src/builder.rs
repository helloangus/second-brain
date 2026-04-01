//! Event builder - creates events from AI analysis
//!
//! IMPORTANT: AI never writes markdown directly. This builder
//! validates and transforms AI JSON output into proper events.

use brain_core::{
    Event, EventAi, EventEntities, EventRelations, EventSource, EventTime, EventType,
    GraphHints, DerivedRefs, RawRefs, PipelineOutput, TaskType,
};
use chrono::Utc;
use std::path::Path;

/// Event builder for creating events from AI analysis
pub struct EventBuilder;

impl EventBuilder {
    /// Build an event from AI analysis output
    pub fn build_from_analysis(
        input_path: &str,
        task_type: &TaskType,
        output: &PipelineOutput,
        source: &Option<String>,
    ) -> Result<Event, Box<dyn std::error::Error>> {
        let now = Utc::now();
        let id = Event::generate_id();

        // Determine event type from task type
        let event_type = match task_type {
            TaskType::ImageCaption | TaskType::FaceDetection | TaskType::Ocr => EventType::Photo,
            TaskType::Asr | TaskType::SpeakerDiarization => EventType::Activity,
            TaskType::Embedding | TaskType::Reasoning | TaskType::Summarize | TaskType::Tagging => {
                EventType::Research
            }
            _ => EventType::Other,
        };

        // Extract filename for summary
        let filename = Path::new(input_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let summary = output.summary.clone()
            .unwrap_or_else(|| format!("Processed {} from {}", task_type, filename));

        let event = Event {
            schema: "event/v1".to_string(),
            id,
            type_: event_type,
            subtype: Some(task_type.to_string()),
            time: EventTime {
                start: now,
                end: None,
                timezone: "UTC".to_string(),
            },
            created_at: Some(now),
            ingested_at: Some(now),
            source: EventSource {
                device: Some("pipeline".to_string()),
                channel: source.clone(),
                capture_agent: Some("brain-pipeline".to_string()),
            },
            status: "auto".to_string(),
            confidence: output.confidence.unwrap_or(0.75),
            entities: EventEntities::default(),
            tags: output.tags.clone(),
            raw_refs: RawRefs {
                files: vec![input_path.to_string()],
            },
            derived_refs: DerivedRefs::default(),
            ai: EventAi {
                summary: Some(summary),
                topics: output.entities.clone(),
                sentiment: None,
                extraction_version: Some(1),
            },
            relations: EventRelations::default(),
            graph_hints: GraphHints::default(),
            schema_version: 1,
        };

        Ok(event)
    }

    /// Create a simple manual event
    #[allow(dead_code)]
    pub fn _create_simple_event(
        summary: &str,
        event_type: EventType,
        tags: &[String],
    ) -> Result<Event, Box<dyn std::error::Error>> {
        let now = Utc::now();
        let id = Event::generate_id();

        Ok(Event {
            schema: "event/v1".to_string(),
            id,
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
                device: Some("cli".to_string()),
                channel: None,
                capture_agent: Some("brain-cli".to_string()),
            },
            status: "manual".to_string(),
            confidence: 0.8,
            entities: EventEntities::default(),
            tags: tags.to_vec(),
            raw_refs: RawRefs::default(),
            derived_refs: DerivedRefs::default(),
            ai: EventAi {
                summary: Some(summary.to_string()),
                topics: Vec::new(),
                sentiment: None,
                extraction_version: None,
            },
            relations: EventRelations::default(),
            graph_hints: GraphHints::default(),
            schema_version: 1,
        })
    }
}
