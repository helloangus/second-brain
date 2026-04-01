//! Task processor

use crate::builder::EventBuilder;
use brain_core::adapters::{create_adapter, AdapterConfig, ModelAdapter, RawDataInput};
use brain_core::{BrainConfig, PipelineOutput, PipelineTask};
use std::fs;
use std::path::PathBuf;
use tracing::{error, info, warn};

/// Process all pending tasks in the queue
pub async fn process_queue(
    config: &BrainConfig,
    limit: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    let pending_dir = config.pipeline_queue_path.join("pending");
    let processing_dir = config.pipeline_queue_path.join("processing");
    let done_dir = config.pipeline_queue_path.join("done");

    fs::create_dir_all(&pending_dir)?;
    fs::create_dir_all(&processing_dir)?;
    fs::create_dir_all(&done_dir)?;

    // Get all pending tasks
    let mut tasks: Vec<PathBuf> = fs::read_dir(&pending_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "yaml")
                .unwrap_or(false)
        })
        .map(|e| e.path())
        .collect();

    // Sort by name (oldest first)
    tasks.sort();

    // Apply limit if specified
    if let Some(l) = limit {
        tasks.truncate(l);
    }

    if tasks.is_empty() {
        println!("No pending tasks.");
        return Ok(());
    }

    println!("Processing {} task(s)...", tasks.len());

    // Get adapter configuration
    let adapter_config = config
        .adapters
        .first()
        .cloned()
        .unwrap_or_else(|| AdapterConfig::ollama("http://localhost:11434", "llama3"));

    let adapter = create_adapter(&adapter_config)?;

    let mut processed = 0;
    let mut failed = 0;

    for task_path in tasks {
        let task: PipelineTask = match fs::read_to_string(&task_path) {
            Ok(s) => match serde_yaml::from_str(&s) {
                Ok(t) => t,
                Err(e) => {
                    error!("Failed to parse task file {:?}: {}", task_path, e);
                    continue;
                }
            },
            Err(e) => {
                error!("Failed to read task file {:?}: {}", task_path, e);
                continue;
            }
        };

        info!("Processing task: {} ({})", task.id, task.task);

        // Move to processing
        let processing_path = processing_dir.join(task_path.file_name().unwrap());
        fs::rename(&task_path, &processing_path)?;

        // Process the task
        match process_task(&task, adapter.as_ref(), config).await {
            Ok(_) => {
                // Move to done
                let done_path = done_dir.join(task_path.file_name().unwrap());
                fs::rename(&processing_path, &done_path)?;
                processed += 1;
                info!("Task {} completed", task.id);
            }
            Err(e) => {
                error!("Task {} failed: {}", task.id, e);
                // Keep in processing for retry
                failed += 1;
            }
        }
    }

    println!();
    println!("Processed: {}", processed);
    println!("Failed: {}", failed);

    Ok(())
}

async fn process_task(
    task: &PipelineTask,
    adapter: &dyn ModelAdapter,
    config: &BrainConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create input for the adapter
    let input = RawDataInput {
        data_type: task.data_type(),
        path: task.input.path.clone(),
        metadata: task.input.metadata.clone(),
    };

    // Analyze the data
    let output: PipelineOutput = if adapter.supports(&task.data_type()) {
        let analysis = adapter.analyze(&input)?;
        PipelineOutput {
            summary: analysis.summary,
            tags: analysis.tags,
            entities: analysis.entities,
            confidence: analysis.confidence,
        }
    } else {
        warn!(
            "Adapter {} does not support {:?}",
            adapter.name(),
            task.data_type()
        );
        PipelineOutput::default()
    };

    // Build event from output
    let event = EventBuilder::build_from_analysis(
        &task.input.path,
        &task.task,
        &output,
        &task.input.source,
    )?;

    // Save event to file
    let year = event.time.start.format("%Y").to_string();
    let month = event.time.start.format("%m").to_string();
    let event_path = config
        .events_path
        .join(&year)
        .join(&month)
        .join(format!("{}.md", event.id));

    fs::create_dir_all(event_path.parent().unwrap())?;
    let serializer = brain_core::markdown::EventSerializer;
    let markdown = serializer.serialize(&event)?;
    fs::write(&event_path, &markdown)?;

    info!("Created event: {} at {:?}", event.id, event_path);

    // Also index in database
    let db = brain_core::Database::open(&config.db_path)?;
    let conn = db.connection();
    let repo = brain_core::EventRepository::new(&conn);
    repo.upsert(&event)?;

    Ok(())
}
