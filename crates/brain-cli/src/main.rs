//! brain-cli - CLI tool for Second Brain
//!
//! Provides commands for searching, viewing, and managing events and entities.

mod commands;

use brain_core::BrainConfig;
use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "brain")]
#[command(about = "Second Brain CLI - Your personal cognitive system", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Search events by keyword
    Search {
        /// Search keyword
        keyword: String,
    },
    /// Show timeline for a specific month
    Timeline {
        /// Month in YYYY-MM format
        #[arg(value_parser = parse_month)]
        month: String,
    },
    /// Show today's events
    Today,
    /// Add a new event
    Add {
        /// Event type
        #[arg(long, default_value = "note")]
        type_: String,
        /// Event summary
        #[arg(long)]
        summary: String,
        /// Tags (comma-separated)
        #[arg(long)]
        tags: Option<String>,
    },
    /// Ingest a file and queue for AI processing
    Ingest {
        /// Path to file to ingest
        #[arg(short, long)]
        file: String,
        /// Source identifier (channel)
        #[arg(short, long, default_value = "CLI")]
        source: String,
        /// Device type
        #[arg(short, long, default_value = "PC")]
        device: String,
        /// Capture agent
        #[arg(short, long, default_value = "manual_entry")]
        agent: String,
        /// Data type (text, image, audio, video, document)
        #[arg(short, long, default_value = "text")]
        type_: String,
        /// Process immediately with AI
        #[arg(short = 'p', long, default_value = "false")]
        process: bool,
    },
    /// Process pending tasks in AI pipeline
    Process {
        /// Limit number of tasks to process
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// View and query system logs
    Logs {
        /// Filter by log type (crud, ai_processing, pipeline, system)
        #[arg(short, long)]
        log_type: Option<String>,
        /// Filter by target type (event, entity, tag, pipeline_task)
        #[arg(short, long)]
        target_type: Option<String>,
        /// Filter by target ID
        #[arg(short, long)]
        target_id: Option<String>,
        /// Maximum number of entries to show
        #[arg(short, long)]
        limit: Option<usize>,
        /// Show AI processing statistics
        #[arg(long, default_value = "false")]
        stats: bool,
        /// Number of days for statistics (default: 7)
        #[arg(long)]
        days: Option<u32>,
    },
    /// Entity commands
    Entity {
        #[command(subcommand)]
        command: EntityCommands,
    },
    /// Show statistics
    Stats,
}

#[derive(Subcommand)]
enum EntityCommands {
    /// List all entities
    List {
        /// Filter by entity type
        #[arg(short, long)]
        type_: Option<String>,
    },
    /// Show a specific entity
    Show {
        /// Entity ID
        id: String,
    },
}

fn parse_month(s: &str) -> Result<String, String> {
    // Validate YYYY-MM format
    if s.len() == 7 && s.chars().nth(4) == Some('-') {
        Ok(s.to_string())
    } else {
        Err("Month must be in YYYY-MM format".to_string())
    }
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
        Commands::Search { keyword } => {
            commands::search::execute(&config, &keyword)?;
        }
        Commands::Timeline { month } => {
            commands::timeline::execute(&config, &month)?;
        }
        Commands::Today => {
            commands::today::execute(&config)?;
        }
        Commands::Add {
            type_,
            summary,
            tags,
        } => {
            let tags: Vec<String> = tags
                .map(|s| s.split(',').map(|t| t.trim().to_string()).collect())
                .unwrap_or_default();
            commands::add::execute(&config, &type_, &summary, &tags)?;
        }
        Commands::Ingest {
            file,
            source,
            device,
            agent,
            type_,
            process,
        } => {
            commands::ingest::execute(&config, &file, &source, &device, &agent, &type_, process)
                .await?;
        }
        Commands::Process { limit } => {
            commands::process::execute(&config, limit)?;
        }
        Commands::Logs {
            log_type,
            target_type,
            target_id,
            limit,
            stats,
            days,
        } => {
            commands::logs::execute(
                &config,
                log_type.as_deref(),
                target_type.as_deref(),
                target_id.as_deref(),
                limit,
                stats,
                days,
            )?;
        }
        Commands::Entity { command } => match command {
            EntityCommands::List { type_ } => {
                commands::entity::list(&config, type_.as_deref())?;
            }
            EntityCommands::Show { id } => {
                commands::entity::show(&config, &id)?;
            }
        },
        Commands::Stats => {
            commands::stats::execute(&config)?;
        }
    }

    Ok(())
}
