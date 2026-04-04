//! brain-indexerd - Filesystem watcher daemon for indexing events
//!
//! Monitors the events/ directory and updates the SQLite index in real-time.

use brain_core::{BrainConfig, Database};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod processor;

use processor::EventProcessor;

/// Shared state for the indexer
struct IndexerState {
    db: Database,
    events_path: PathBuf,
    entities_path: PathBuf,
}

impl IndexerState {
    fn new(config: &BrainConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let db = Database::open(&config.db_path)?;
        Ok(Self {
            db,
            events_path: config.events_path.clone(),
            entities_path: config.entities_path.clone(),
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting brain-indexerd...");

    // Load configuration
    let config = BrainConfig::load()?;
    info!("Loaded configuration");

    // Initialize state
    let state = Arc::new(Mutex::new(IndexerState::new(&config)?));
    info!("数据库已初始化于 {:?}", config.db_path);

    // Process existing files first
    {
        let state_guard = state.lock().await;
        processor::index_existing_files(
            &state_guard.db,
            &state_guard.events_path,
            &state_guard.entities_path,
        )?;
    }

    // Start filesystem watcher
    let events_path = config.events_path.clone();
    let entities_path = config.entities_path.clone();
    let watcher_state = state.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            if let Err(e) = run_watcher(events_path, entities_path, watcher_state).await {
                error!("监视器错误: {}", e);
            }
        });
    });

    // Keep the main thread alive
    info!("索引服务运行中。按 Ctrl+C 停止。");
    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

async fn run_watcher(
    events_path: PathBuf,
    entities_path: PathBuf,
    state: Arc<Mutex<IndexerState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        },
        Config::default().with_poll_interval(Duration::from_secs(2)),
    )?;

    watcher.watch(&events_path, RecursiveMode::Recursive)?;
    watcher.watch(&entities_path, RecursiveMode::Recursive)?;

    info!(
        "正在监视 {} 和 {}",
        events_path.display(),
        entities_path.display()
    );

    for event in rx {
        handle_event(&event, &state).await;
    }

    Ok(())
}

async fn handle_event(event: &Event, state: &Arc<Mutex<IndexerState>>) {
    let paths: Vec<PathBuf> = event.paths.to_vec();

    for path in paths {
        // Only process .md files
        if path.extension().map(|e| e != "md").unwrap_or(true) {
            continue;
        }

        let state_guard = state.lock().await;
        let processor = EventProcessor::new(&state_guard.db);

        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) => {
                info!("正在处理: {:?}", path);
                if let Err(e) = processor.process_file(&path) {
                    error!("处理 {:?} 失败: {}", path, e);
                }
            }
            EventKind::Remove(_) => {
                info!("正在删除: {:?}", path);
                if let Err(e) = processor.remove_file(&path) {
                    error!("删除 {:?} 失败: {}", path, e);
                }
            }
            _ => {}
        }
    }
}
