//! Logs command - view and query system logs

use brain_core::{BrainConfig, LogEntry, LogType, TargetType};

/// Execute the logs command
///
/// View and query system logs with filtering options.
pub fn execute(
    config: &BrainConfig,
    log_type: Option<&str>,
    target_type: Option<&str>,
    target_id: Option<&str>,
    limit: Option<usize>,
    stats: bool,
    days: Option<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    let logger = brain_core::Logger::new(config);

    let limit = limit.unwrap_or(50);
    let days = days.unwrap_or(7);

    if stats {
        // Show AI processing statistics
        let ai_stats = logger.get_ai_stats(days)?;

        println!("AI Processing Statistics (last {} days)", days);
        println!("==========================================");
        println!("Total operations:  {}", ai_stats.total_operations);
        println!("Successful:         {}", ai_stats.successful_operations);
        println!("Failed:             {}", ai_stats.failed_operations);
        if ai_stats.avg_duration_ms > 0.0 {
            println!("Average duration:   {:.2} ms", ai_stats.avg_duration_ms);
        }

        let success_rate = if ai_stats.total_operations > 0 {
            (ai_stats.successful_operations as f64 / ai_stats.total_operations as f64) * 100.0
        } else {
            0.0
        };
        println!("Success rate:       {:.1}%", success_rate);

        return Ok(());
    }

    // Query logs
    #[allow(clippy::needless_borrow)]
    let entries: Vec<LogEntry> = if let Some(tt) = target_type {
        if let Some(tid) = target_id {
            let tt = parse_target_type(&tt);
            logger.get_for_target(tt, &tid, limit)?
        } else {
            logger.get_recent(limit)?
        }
    } else if let Some(lt) = log_type {
        let lt = parse_log_type(&lt);
        logger.get_by_type(lt, limit)?
    } else {
        logger.get_recent(limit)?
    };

    if entries.is_empty() {
        println!("No logs found.");
        return Ok(());
    }

    let count = entries.len();

    // Print header
    println!(
        "{:<8} {:<20} {:<12} {:<12} {:<40} DURATION",
        "LEVEL", "TIMESTAMP", "TYPE", "OPERATION", "TARGET"
    );
    println!("{}", "=".repeat(100));

    for entry in &entries {
        let timestamp = entry.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
        let target = match &entry.target_id {
            Some(id) => format!("{}:{}", entry.target_type, id),
            None => entry.target_type.to_string(),
        };
        let duration = match entry.duration_ms {
            Some(ms) => format!("{:.2}ms", ms as f64),
            None => "-".to_string(),
        };

        println!(
            "{:<8} {:<20} {:<12} {:<12} {:<40} {}",
            entry.level,
            timestamp,
            entry.log_type,
            entry.operation,
            truncate(&target, 38),
            duration
        );

        if !entry.success {
            if let Some(ref err) = entry.error_message {
                println!("         ERROR: {}", truncate(err, 90));
            }
        }
    }

    println!();
    println!("Total: {} entries", count);

    Ok(())
}

fn parse_log_type(s: &str) -> LogType {
    match s.to_lowercase().as_str() {
        "crud" => LogType::Crud,
        "ai" | "ai_processing" => LogType::AiProcessing,
        "pipeline" => LogType::Pipeline,
        "system" => LogType::System,
        "tag" => LogType::Tag,
        "cognition" => LogType::Cognition,
        "evaluation" => LogType::Evaluation,
        _ => LogType::Custom,
    }
}

fn parse_target_type(s: &str) -> TargetType {
    match s.to_lowercase().as_str() {
        "event" => TargetType::Event,
        "entity" => TargetType::Entity,
        "tag" => TargetType::Tag,
        "pipeline_task" | "pipeline" => TargetType::PipelineTask,
        "config" => TargetType::Config,
        "system" => TargetType::System,
        _ => TargetType::System,
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
