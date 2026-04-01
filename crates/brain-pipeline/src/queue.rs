//! Queue management

use brain_core::{BrainConfig, PipelineInput, PipelineTask, TaskStatus, TaskType};
use std::fs;
use std::path::Path;
use tracing::info;
use uuid::Uuid;

/// Add a task to the queue
pub async fn add_task(
    config: &BrainConfig,
    task_type: &str,
    input_path: &str,
    source: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let task =
        TaskType::from_str(task_type).ok_or_else(|| format!("Unknown task type: {}", task_type))?;

    let task_id = Uuid::new_v4().to_string()[..8].to_string();

    let pipeline_task = PipelineTask {
        id: task_id.clone(),
        task,
        input: PipelineInput {
            path: input_path.to_string(),
            source: source.map(|s| s.to_string()),
            metadata: std::collections::HashMap::new(),
        },
        output: None,
        status: TaskStatus::Pending,
    };

    let queue_dir = config.pipeline_queue_path.join("pending");
    fs::create_dir_all(&queue_dir)?;

    let task_path = queue_dir.join(format!("{}.yaml", task_id));

    let content = serde_yaml::to_string(&pipeline_task)?;
    fs::write(&task_path, content)?;

    info!("Added task {} to queue", task_id);
    println!("Task {} added: {} -> {}", task_id, task_type, input_path);

    Ok(())
}

/// Show queue status
pub fn show_status(config: &BrainConfig) -> Result<(), Box<dyn std::error::Error>> {
    let pending = count_files(&config.pipeline_queue_path.join("pending"))?;
    let processing = count_files(&config.pipeline_queue_path.join("processing"))?;
    let done = count_files(&config.pipeline_queue_path.join("done"))?;

    println!("Pipeline Queue Status");
    println!("{}", "=".repeat(50));
    println!("Pending:    {}", pending);
    println!("Processing: {}", processing);
    println!("Done:       {}", done);

    Ok(())
}

fn count_files(dir: &Path) -> Result<usize, Box<dyn std::error::Error>> {
    if !dir.exists() {
        return Ok(0);
    }

    let count = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "yaml")
                .unwrap_or(false)
        })
        .count();

    Ok(count)
}
