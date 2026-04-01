//! Today command

use brain_core::{BrainConfig, Database, EventRepository};
use chrono::Utc;

pub fn execute(config: &BrainConfig) -> Result<(), Box<dyn std::error::Error>> {
    let now = Utc::now();
    let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
    let today_end = now.date_naive().and_hms_opt(23, 59, 59).unwrap().and_utc();

    let db = Database::open(&config.db_path)?;
    let conn = db.connection();
    let repo = EventRepository::new(&conn);

    println!("Today's Events: {}", now.format("%Y-%m-%d"));
    println!("{}", "=".repeat(50));

    let events = repo.find_by_time_range(today_start, today_end)?;

    if events.is_empty() {
        println!("No events today.");
        return Ok(());
    }

    for event in &events {
        let time = event.time.start.format("%H:%M");
        let end_time = event.time.end.map(|e| e.format("%H:%M").to_string());
        let type_str = event.type_.to_string();

        println!();
        print!("[{}", time);
        if let Some(ref end) = end_time {
            print!(" - {}", end);
        }
        println!("] ({})", type_str);

        if let Some(ref summary) = event.ai.summary {
            println!("  {}", summary);
        }

        if !event.tags.is_empty() {
            println!("  Tags: {}", event.tags.join(", "));
        }
    }

    println!();
    println!("Total: {} event(s)", events.len());

    Ok(())
}
