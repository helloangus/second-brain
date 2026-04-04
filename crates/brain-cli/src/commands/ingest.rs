//! Ingest command - add files to the AI processing pipeline

use brain_core::dicts::{prompt_selection, DictEntry};
use brain_core::{BrainConfig, DictSet, RawDataType};
use brain_pipeline::processor;
use brain_pipeline::queue;
use chrono::Utc;
use std::fs;
use std::path::Path;

/// Execute the ingest command
///
/// Adds a file to the raw data lake and queues it for AI processing.
/// If --process is specified, also runs the AI pipeline immediately.
pub async fn execute(
    config: &BrainConfig,
    file_path: &str,
    source: &str,
    device: &str,
    agent: &str,
    data_type: &str,
    process: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(file_path);

    // Validate input file exists
    if !path.exists() {
        return Err(format!("File not found: {}", file_path).into());
    }

    // Read file content to verify it's readable
    let _content = fs::read(path)?;
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // Initialize dictionaries
    let dicts_path = config.dicts_path.clone();
    DictSet::init_if_missing(&dicts_path)?;
    let mut dicts = DictSet::load(&dicts_path)?;

    // Resolve values using dictionaries - with interactive selection if not found
    let (resolved_device, device_is_new) = if dicts.device.exists(device) {
        (device.to_string(), false)
    } else {
        println!("Device '{}' not found.", device);
        prompt_selection(&dicts.device, device, "device")?
    };
    if device_is_new {
        dicts.device.add(DictEntry::new(&resolved_device));
        println!("Added '{}' to device dictionary.", resolved_device);
    }

    let (resolved_agent, agent_is_new) = if dicts.capture_agent.exists(agent) {
        (agent.to_string(), false)
    } else {
        println!("Capture agent '{}' not found.", agent);
        prompt_selection(&dicts.capture_agent, agent, "capture_agent")?
    };
    if agent_is_new {
        dicts.capture_agent.add(DictEntry::new(&resolved_agent));
        println!("Added '{}' to capture_agent dictionary.", resolved_agent);
    }

    // Save dictionaries if new values were added
    if device_is_new || agent_is_new {
        dicts.save(&dicts_path)?;
        println!("Dictionaries updated.");
    }

    // Parse data type (default to text)
    let raw_data_type = match data_type.to_lowercase().as_str() {
        "image" => RawDataType::Image,
        "audio" => RawDataType::Audio,
        "video" => RawDataType::Video,
        "document" => RawDataType::Document,
        _ => RawDataType::Text,
    };

    // Create destination path in raw data lake by data type: data/raw/{data_type}/YYYY/MM/DD/
    let now = Utc::now();
    let year = now.format("%Y");
    let month = now.format("%m");
    let day = now.format("%d");
    let data_type_str = match raw_data_type {
        RawDataType::Text => "text",
        RawDataType::Image => "image",
        RawDataType::Audio => "audio",
        RawDataType::Video => "video",
        RawDataType::Document => "document",
    };

    let dest_dir = config
        .raw_data_path
        .join(data_type_str)
        .join(year.to_string())
        .join(month.to_string())
        .join(day.to_string());

    // Create directory structure
    fs::create_dir_all(&dest_dir)?;

    // Generate unique filename: {timestamp}_{source}_{original_name}
    let timestamp = now.format("%Y%m%dT%H%M%S");
    let safe_source = source.replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
    let dest_filename = format!("{}_{}_{}", timestamp, safe_source, file_name);
    let dest_path = dest_dir.join(&dest_filename);

    // Copy file to raw data lake
    let file_size = fs::metadata(path).map(|m| m.len()).ok();
    fs::copy(path, &dest_path)?;
    println!("Copied to: {}", dest_path.display());

    // Log ingest file
    let logger = brain_core::Logger::new(config);
    let _ = logger.log_ingest_file(
        file_path,
        &dest_path.to_string_lossy(),
        data_type_str,
        file_size,
    );

    // Add task to pipeline queue
    // Path relative to raw_data_path, includes data_type prefix
    let relative_path = dest_path
        .strip_prefix(&config.raw_data_path)
        .unwrap_or(&dest_path);

    let task_id = queue::add_task(
        config,
        "summarize",
        relative_path.to_str().unwrap(),
        Some(source),
        Some(&resolved_device),
        Some(&resolved_agent),
        raw_data_type,
    )
    .await?;

    // Log queue add
    let _ = logger.log_queue_add(&task_id, "summarize", relative_path.to_str().unwrap());

    // If --process flag set, run AI processing immediately
    if process {
        if config.adapters.is_empty() {
            return Err(
                "AI adapter not configured. Please configure adapters in config/brain.yaml".into(),
            );
        }
        println!("Processing with AI...");
        processor::process_queue(config, None).await?;
    }

    Ok(())
}
