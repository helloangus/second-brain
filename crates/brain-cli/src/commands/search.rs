//! Search command

use brain_core::{BrainConfig, Database, DictSet, EventRepository};

pub fn execute(config: &BrainConfig, keyword: &str) -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::open(&config.db_path)?;
    let conn = db.connection();
    let repo = EventRepository::new(&conn);

    // Load dictionaries for Chinese display
    let dicts = DictSet::load(&config.dicts_path).unwrap_or_else(|_| DictSet::default_dicts());

    println!("Searching for: {}", keyword);
    println!("{}", "=".repeat(50));

    let events = repo.search(keyword)?;

    if events.is_empty() {
        println!("未找到事件。");
        return Ok(());
    }

    for event in &events {
        let time = event.time.start.format("%Y-%m-%d %H:%M");
        println!();
        println!("[{}] {}", time, event.id);
        println!("  类型: {}", event.type_display_zh(&dicts));
        if let Some(ref summary) = event.ai.summary {
            println!("  摘要: {}", summary);
        }
        if !event.tags.is_empty() {
            println!("  标签: {}", event.tags.join(", "));
        }
    }

    println!();
    println!("找到 {} 个事件", events.len());

    Ok(())
}
