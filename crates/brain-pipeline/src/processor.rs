//! Task processor

use crate::builder::EventBuilder;
use brain_core::adapters::{
    create_adapter, AdapterConfig, DictContext, ModelAdapter, RawDataInput,
};
use brain_core::{BrainConfig, DictSet, PipelineOutput, PipelineTask};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info, warn};

/// A wrapper error that is Send + Sync
#[derive(Debug)]
pub struct PipelineError(String);

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PipelineError {}

impl From<std::io::Error> for PipelineError {
    fn from(e: std::io::Error) -> Self {
        PipelineError(format!("IO error: {}", e))
    }
}

impl From<serde_yaml::Error> for PipelineError {
    fn from(e: serde_yaml::Error) -> Self {
        PipelineError(format!("YAML error: {}", e))
    }
}

impl From<brain_core::Error> for PipelineError {
    fn from(e: brain_core::Error) -> Self {
        PipelineError(format!("Brain error: {}", e))
    }
}

/// Process all pending tasks in the queue
pub async fn process_queue(
    config: &BrainConfig,
    limit: Option<usize>,
) -> Result<(), PipelineError> {
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
    let adapter_config =
        config.adapters.first().cloned().unwrap_or_else(|| {
            AdapterConfig::ollama("http://localhost:11434", "qwen3.5:9b-q4_K_M")
        });

    let adapter = Arc::new(
        create_adapter(&adapter_config)
            .map_err(|e| PipelineError(format!("Adapter error: {}", e)))?,
    );

    // Clone config for each task since we'll move it into closures
    let config_clone = config.clone();

    let mut processed = 0;
    let mut failed = 0;
    let mut total_duration = std::time::Duration::ZERO;

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

        let task_id = task.id.clone();
        info!("Processing task: {} ({})", task_id, task.task);

        // Move to processing
        let processing_path = processing_dir.join(task_path.file_name().unwrap());
        if let Err(e) = fs::rename(&task_path, &processing_path) {
            error!("Failed to move task file: {}", e);
            continue;
        }

        // Clone Arc for this task
        let adapter_clone = Arc::clone(&adapter);
        let config_for_task = config_clone.clone();

        // Time the task processing
        let start = std::time::Instant::now();

        // Process the task in a blocking thread to avoid tokio runtime conflicts
        // with the blocking HTTP client
        let task_result = tokio::task::spawn_blocking(move || {
            process_task_sync(&task, adapter_clone.as_ref().as_ref(), &config_for_task)
        })
        .await;

        let elapsed = start.elapsed();
        total_duration += elapsed;

        match task_result {
            Ok(Ok(_)) => {
                // Move to done
                let done_path = done_dir.join(task_path.file_name().unwrap());
                if fs::rename(&processing_path, &done_path).is_ok() {
                    processed += 1;
                    println!(
                        "  Task {} completed in {:.2}s",
                        task_id,
                        elapsed.as_secs_f64()
                    );
                    info!("Task {} completed", task_id);
                }
            }
            Ok(Err(e)) => {
                error!("Task {} failed: {}", task_id, e);
                failed += 1;
            }
            Err(e) => {
                error!("Task {} panicked: {}", task_id, e);
                failed += 1;
            }
        }
    }

    println!();
    let avg_time = if processed > 0 {
        total_duration.as_secs_f64() / processed as f64
    } else {
        0.0
    };
    println!(
        "Processed: {} ({:.2}s total, {:.2}s avg)",
        processed,
        total_duration.as_secs_f64(),
        avg_time
    );
    println!("Failed: {}", failed);

    Ok(())
}

/// Load dictionary context for AI prompts
fn load_dict_context(dicts_path: &std::path::Path) -> DictContext {
    match DictSet::load(dicts_path) {
        Ok(dicts) => DictContext {
            event_types: dicts.event_type.keys().into_iter().cloned().collect(),
            event_subtypes: dicts.event_subtype.keys().into_iter().cloned().collect(),
            tags: dicts.tags.keys().into_iter().cloned().collect(),
            topics: dicts.topics.keys().into_iter().cloned().collect(),
        },
        Err(_) => DictContext::default(),
    }
}

fn process_task_sync(
    task: &PipelineTask,
    adapter: &dyn ModelAdapter,
    config: &BrainConfig,
) -> Result<(), PipelineError> {
    use std::path::Path;

    // Resolve the input path - if relative, prepend raw_data_path
    let input_path = if Path::new(&task.input.path).is_absolute() {
        Path::new(&task.input.path).to_path_buf()
    } else {
        config.raw_data_path.join(&task.input.path)
    };

    // Load dictionaries for AI context
    let dict_context = load_dict_context(&config.dicts_path);

    // Create input for the adapter
    let input = RawDataInput {
        data_type: task.data_type(),
        path: input_path.to_string_lossy().to_string(),
        metadata: task.input.metadata.clone(),
        dict_context: Some(dict_context),
    };

    // Analyze the data
    let file_size_bytes = fs::metadata(&input_path).map(|m| m.len()).ok();
    let ai_start = std::time::Instant::now();

    let output: PipelineOutput = if adapter.supports(&task.data_type()) {
        match adapter.analyze(&input) {
            Ok(analysis) => {
                let ai_elapsed = ai_start.elapsed();
                let ai_duration_ms = ai_elapsed.as_millis() as u64;

                // Log AI processing
                let logger = brain_core::Logger::new(config);
                let data_type_str = match task.data_type() {
                    brain_core::RawDataType::Image => "image",
                    brain_core::RawDataType::Audio => "audio",
                    brain_core::RawDataType::Video => "video",
                    brain_core::RawDataType::Document => "document",
                    _ => "text",
                };
                let _ = logger.log_ai_processing(
                    &input_path.to_string_lossy(),
                    data_type_str,
                    adapter.name(),
                    ai_duration_ms,
                    true,
                    file_size_bytes,
                    None,
                );
                println!(
                    "  AI analysis: {:.2}s ({})",
                    ai_elapsed.as_secs_f64(),
                    data_type_str
                );

                PipelineOutput {
                    summary: analysis.summary,
                    extended: analysis.extended,
                    type_: analysis.type_,
                    subtype: analysis.subtype,
                    tags: analysis.tags,
                    topics: analysis.topics,
                    entities: analysis.entities,
                    confidence: analysis.confidence,
                }
            }
            Err(e) => {
                let ai_elapsed = ai_start.elapsed();
                let ai_duration_ms = ai_elapsed.as_millis() as u64;

                // Log AI processing failure
                let logger = brain_core::Logger::new(config);
                let data_type_str = match task.data_type() {
                    brain_core::RawDataType::Image => "image",
                    brain_core::RawDataType::Audio => "audio",
                    brain_core::RawDataType::Video => "video",
                    brain_core::RawDataType::Document => "document",
                    _ => "text",
                };
                let _ = logger.log_ai_processing(
                    &input_path.to_string_lossy(),
                    data_type_str,
                    adapter.name(),
                    ai_duration_ms,
                    false,
                    file_size_bytes,
                    Some(&e.to_string()),
                );

                warn!("Analysis failed: {}", e);
                PipelineOutput::default()
            }
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
        &task.input.channel,
        &task.input.device,
        &task.input.capture_agent,
    )
    .map_err(|e| PipelineError(format!("Event build error: {}", e)))?;

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
    let markdown = serializer
        .serialize(&event)
        .map_err(|e| PipelineError(format!("Serialize error: {}", e)))?;
    fs::write(&event_path, &markdown).map_err(|e| PipelineError(format!("Write error: {}", e)))?;

    info!("Created event: {} at {:?}", event.id, event_path);

    // Log event create
    let logger = brain_core::Logger::new(config);
    let source = brain_core::LogSource {
        device: task.input.device.clone(),
        channel: task.input.channel.clone(),
        agent: task.input.capture_agent.clone(),
    };
    let _ = logger.log_event_crud(
        brain_core::CrudOperation::Create,
        &event.id,
        &source,
        0, // no separate duration for create
    );

    // Also index in database
    let db = brain_core::Database::open(&config.db_path)
        .map_err(|e| PipelineError(format!("DB open error: {}", e)))?;
    let conn = db.connection();
    let repo = brain_core::EventRepository::new(&conn);
    repo.upsert(&event)
        .map_err(|e| PipelineError(format!("Repo error: {}", e)))?;

    Ok(())
}
