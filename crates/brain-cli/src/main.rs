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

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
        Commands::Entity { command } => {
            match command {
                EntityCommands::List { type_ } => {
                    commands::entity::list(&config, type_.as_deref())?;
                }
                EntityCommands::Show { id } => {
                    commands::entity::show(&config, &id)?;
                }
            }
        }
        Commands::Stats => {
            commands::stats::execute(&config)?;
        }
    }

    Ok(())
}
