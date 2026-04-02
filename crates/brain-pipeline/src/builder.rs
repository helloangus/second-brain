//! Event builder - creates events from AI analysis
//!
//! IMPORTANT: AI never writes markdown directly. This builder
//! validates and transforms AI JSON output into proper events.

use brain_core::{
    DerivedRefs, Event, EventAi, EventEntities, EventRelations, EventSource, EventTime, EventType,
    GraphHints, PipelineOutput, RawRefs, TaskType,
};
use chrono::Utc;
use std::path::Path;

/// Event builder for creating events from AI analysis
pub struct EventBuilder;

impl EventBuilder {
    /// Build an event from AI analysis output
    ///
    /// Parameters:
    /// - input_path: path to the input file
    /// - task_type: the task type (e.g., summarize, reasoning)
    /// - output: AI analysis output containing summary, tags, etc.
    /// - channel: input channel (e.g., CLI, API)
    /// - device: device that created the data (e.g., PC, iPhone)
    /// - capture_agent: how data was captured (e.g., manual_entry, pipeline)
    pub fn build_from_analysis(
        input_path: &str,
        task_type: &TaskType,
        output: &PipelineOutput,
        channel: &Option<String>,
        device: &Option<String>,
        capture_agent: &Option<String>,
    ) -> Result<Event, Box<dyn std::error::Error>> {
        let now = Utc::now();
        let id = Event::generate_id();

        // Determine event type from task type, or use AI-provided type
        let event_type = if let Some(ref type_str) = output.type_ {
            EventType::try_from_str(type_str).unwrap_or_else(|| Self::event_type_from_task(task_type))
        } else {
            Self::event_type_from_task(task_type)
        };

        // Use AI-provided subtype or fall back to task type
        let subtype = output
            .subtype
            .clone()
            .or_else(|| Some(task_type.to_string()));

        // Extract filename for summary
        let filename = Path::new(input_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let summary = output
            .summary
            .clone()
            .unwrap_or_else(|| format!("Processed {} from {}", task_type, filename));

        let event = Event {
            schema: "event/v1".to_string(),
            id,
            type_: event_type,
            subtype,
            time: EventTime {
                start: now,
                end: None,
                timezone: "UTC".to_string(),
            },
            created_at: Some(now),
            ingested_at: Some(now),
            source: EventSource {
                device: device.clone(),
                channel: channel.clone(),
                capture_agent: capture_agent.clone(),
            },
            confidence: output.confidence.unwrap_or(0.75),
            entities: EventEntities::default(),
            tags: output.tags.clone(),
            raw_refs: RawRefs {
                files: vec![input_path.to_string()],
            },
            derived_refs: DerivedRefs::default(),
            ai: EventAi {
                summary: Some(summary),
                extended: output.extended.clone(),
                topics: output.topics.clone(),
                sentiment: None,
                extraction_version: Some(1),
            },
            relations: EventRelations::default(),
            graph_hints: GraphHints::default(),
            schema_version: 1,
        };

        Ok(event)
    }

    /// Determine event type from task type
    fn event_type_from_task(task_type: &TaskType) -> EventType {
        match task_type {
            TaskType::ImageCaption | TaskType::FaceDetection | TaskType::Ocr => EventType::Photo,
            TaskType::Asr | TaskType::SpeakerDiarization => EventType::Activity,
            TaskType::Embedding | TaskType::Reasoning | TaskType::Summarize | TaskType::Tagging => {
                EventType::Research
            }
            _ => EventType::Other,
        }
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
                device: Some("PC".to_string()),
                channel: Some("CLI".to_string()),
                capture_agent: Some("manual_entry".to_string()),
            },
            confidence: 0.8,
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
        })
    }
}
