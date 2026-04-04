//! Process command - run AI pipeline on pending tasks

use brain_core::BrainConfig;
use std::process::Command;

/// Execute the process command
///
/// Processes all pending tasks in the AI pipeline queue.
/// This invokes the brain-pipeline binary directly to avoid tokio runtime conflicts.
pub fn execute(
    config: &BrainConfig,
    limit: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if AI adapter is configured
    if config.adapters.is_empty() {
        return Err(
            "AI adapter not configured. Please configure adapters in config/brain.yaml".into(),
        );
    }

    // Find the brain-pipeline binary
    let pipeline_binary = std::env::current_exe()?
        .parent()
        .ok_or("Cannot find binary directory")?
        .join("brain-pipeline");

    // Build the command
    let mut cmd = Command::new(&pipeline_binary);
    cmd.arg("process");

    if let Some(l) = limit {
        cmd.arg("--limit").arg(l.to_string());
    }

    // Run and wait for completion
    let status = cmd.status()?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "Pipeline process failed with exit code: {:?}",
            status.code()
        )
        .into())
    }
}
