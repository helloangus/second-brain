//! Search command

use brain_core::{BrainConfig, Database, EventRepository};

pub fn execute(config: &BrainConfig, keyword: &str) -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::open(&config.db_path)?;
    let conn = db.connection();
    let repo = EventRepository::new(&conn);

    println!("Searching for: {}", keyword);
    println!("{}", "=".repeat(50));

    let events = repo.search(keyword)?;

    if events.is_empty() {
        println!("No events found.");
        return Ok(());
    }

    for event in &events {
        let time = event.time.start.format("%Y-%m-%d %H:%M");
        println!();
        println!("[{}] {}", time, event.id);
        println!("  Type: {}", event.type_);
        if let Some(ref summary) = event.ai.summary {
            println!("  Summary: {}", summary);
        }
        if !event.tags.is_empty() {
            println!("  Tags: {}", event.tags.join(", "));
        }
    }

    println!();
    println!("Found {} event(s)", events.len());

    Ok(())
}
