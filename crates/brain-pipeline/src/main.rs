//! brain-pipeline - AI processing pipeline binary

use brain_pipeline::{processor, queue};
use brain_core::BrainConfig;
use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "brain-pipeline")]
#[command(about = "AI Pipeline for Second Brain", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Process pending tasks
    Process {
        /// Number of tasks to process (default: all)
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Show queue status
    Status,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    // Load configuration
    let config = BrainConfig::load()?;

    match cli.command {
        Commands::Process { limit } => {
            if let Err(e) = processor::process_queue(&config, limit).await {
                eprintln!("Processing failed: {}", e);
            }
        }
        Commands::Status => {
            if let Err(e) = queue::show_status(&config) {
                eprintln!("Show status failed: {}", e);
            }
        }
    }

    Ok(())
}
