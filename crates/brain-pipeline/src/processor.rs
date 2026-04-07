//! Task processor

use crate::builder::EventBuilder;
use brain_core::adapters::{create_adapter, AdapterConfig, ModelAdapter, RawDataInput};
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
        println!("无待处理任务。");
        return Ok(());
    }

    println!("正在处理 {} 个任务...", tasks.len());

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
                        "  任务 {} 完成，耗时 {:.2}秒",
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
        "已处理: {} ({:.2}秒总计, {:.2}秒平均)",
        processed,
        total_duration.as_secs_f64(),
        avg_time
    );
    println!("失败: {}", failed);

    Ok(())
}

/// Load dictionary set for AI prompts
fn load_dict_context(dicts_path: &std::path::Path) -> DictSet {
    DictSet::load(dicts_path).unwrap_or_else(|_| DictSet::default_dicts())
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

    // Load dictionaries for AI context (includes full DictSet for Step 2)
    let dicts = load_dict_context(&config.dicts_path);

    // Create input for the adapter
    let input = RawDataInput {
        data_type: task.data_type(),
        path: input_path.to_string_lossy().to_string(),
        metadata: task.input.metadata.clone(),
        dict_set: Some(dicts.clone()),
    };

    // Analyze the data
    let file_size_bytes = fs::metadata(&input_path).map(|m| m.len()).ok();
    let ai_start = std::time::Instant::now();

    let analysis_result: Result<
        (brain_core::adapters::AnalysisOutputWithNewEntries, DictSet),
        PipelineError,
    > = if adapter.supports(&task.data_type()) {
        match adapter.analyze(&input) {
            Ok(result) => {
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
                    "  AI分析: {:.2}秒 ({})",
                    ai_elapsed.as_secs_f64(),
                    data_type_str
                );

                // dicts may have been modified, pass it back
                Ok((result, dicts))
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
                Err(PipelineError(format!("Analysis error: {}", e)))
            }
        }
    } else {
        warn!(
            "Adapter {} does not support {:?}",
            adapter.name(),
            task.data_type()
        );
        Err(PipelineError(format!(
            "Adapter {} does not support {:?}",
            adapter.name(),
            task.data_type()
        )))
    };

    let (analysis_output, mut dicts) = match analysis_result {
        Ok((result, d)) => (result, d),
        Err(e) => return Err(e),
    };

    let output = PipelineOutput {
        summary: analysis_output.analysis.summary,
        extended: analysis_output.analysis.extended,
        type_: analysis_output.analysis.type_,
        subtype: analysis_output.analysis.subtype,
        tags: analysis_output.analysis.tags,
        topics: analysis_output.analysis.topics,
        entities: analysis_output.analysis.entities,
        confidence: analysis_output.analysis.confidence,
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

    // Add new dictionary entries discovered during analysis
    let new_entries = &analysis_output.new_entries;
    if !new_entries.event_types.is_empty()
        || !new_entries.event_subtypes.is_empty()
        || !new_entries.tags.is_empty()
        || !new_entries.topics.is_empty()
    {
        let mut changed = false;

        for entry in &new_entries.event_types {
            if !dicts.event_type.exists(&entry.key) {
                dicts.event_type.add(entry.clone());
                changed = true;
            }
        }
        for entry in &new_entries.event_subtypes {
            if !dicts.event_subtype.exists(&entry.key) {
                dicts.event_subtype.add(entry.clone());
                changed = true;
            }
        }
        for entry in &new_entries.tags {
            if !dicts.tags.exists(&entry.key) {
                dicts.tags.add(entry.clone());
                changed = true;
            }
        }
        for entry in &new_entries.topics {
            if !dicts.topics.exists(&entry.key) {
                dicts.topics.add(entry.clone());
                changed = true;
            }
        }

        if changed {
            dicts
                .save(&config.dicts_path)
                .map_err(|e| PipelineError(format!("Dict save error: {}", e)))?;
            info!("Updated dictionaries with new entries");
        }
    }

    Ok(())
}
