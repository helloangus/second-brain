//! Queue management

use brain_core::{BrainConfig, PipelineInput, PipelineTask, RawDataType, TaskStatus, TaskType};
use std::fs;
use std::path::Path;
use tracing::info;
use uuid::Uuid;

/// Add a task to the queue
pub async fn add_task(
    config: &BrainConfig,
    task_type: &str,
    input_path: &str,
    channel: Option<&str>,
    device: Option<&str>,
    capture_agent: Option<&str>,
    data_type: RawDataType,
) -> Result<String, Box<dyn std::error::Error>> {
    let task =
        TaskType::from_str(task_type).ok_or_else(|| format!("Unknown task type: {}", task_type))?;

    let task_id = Uuid::new_v4().to_string()[..8].to_string();

    let pipeline_task = PipelineTask {
        id: task_id.clone(),
        task,
        input: PipelineInput {
            path: input_path.to_string(),
            channel: channel.map(|s| s.to_string()),
            device: device.map(|s| s.to_string()),
            capture_agent: capture_agent.map(|s| s.to_string()),
            data_type,
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
    println!("任务 {} 已添加: {} -> {}", task_id, task_type, input_path);

    Ok(task_id)
}

/// Show queue status
pub fn show_status(config: &BrainConfig) -> Result<(), Box<dyn std::error::Error>> {
    let pending = count_files(&config.pipeline_queue_path.join("pending"))?;
    let processing = count_files(&config.pipeline_queue_path.join("processing"))?;
    let done = count_files(&config.pipeline_queue_path.join("done"))?;

    println!("流水线队列状态");
    println!("{}", "=".repeat(50));
    println!("待处理:    {}", pending);
    println!("处理中: {}", processing);
    println!("已完成:       {}", done);

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
