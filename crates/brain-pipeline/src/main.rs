//! brain-pipeline - AI processing pipeline
//!
//! Processes raw data through AI models and creates events.

mod builder;
mod processor;
mod queue;

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
    /// Add a task to the queue
    Add {
        /// Task type (image.analyze, text.analyze, etc.)
        #[arg(long)]
        task: String,
        /// Input file path
        #[arg(long)]
        input: String,
        /// Source description
        #[arg(long)]
        source: Option<String>,
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
            processor::process_queue(&config, limit).await?;
        }
        Commands::Add {
            task,
            input,
            source,
        } => {
            queue::add_task(&config, &task, &input, source.as_deref()).await?;
        }
        Commands::Status => {
            queue::show_status(&config)?;
        }
    }

    Ok(())
}
