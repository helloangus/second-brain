//! Timeline command

use brain_core::{BrainConfig, Database, DictSet, EventRepository};
use chrono::NaiveDate;

pub fn execute(config: &BrainConfig, month: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Parse month (YYYY-MM)
    let parts: Vec<&str> = month.split('-').collect();
    if parts.len() != 2 {
        return Err("Invalid month format. Use YYYY-MM".into());
    }

    let year: i32 = parts[0].parse()?;
    let month_num: u32 = parts[1].parse()?;

    let start = NaiveDate::from_ymd_opt(year, month_num, 1).ok_or("Invalid date")?;
    let end = if month_num == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month_num + 1, 1).unwrap()
    };

    let start_dt = start.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let end_dt = end.and_hms_opt(0, 0, 0).unwrap().and_utc();

    let db = Database::open(&config.db_path)?;
    let conn = db.connection();
    let repo = EventRepository::new(&conn);

    // Load dictionaries for Chinese display
    let dicts = DictSet::load(&config.dicts_path).unwrap_or_else(|_| DictSet::default_dicts());

    println!("Timeline: {}", month);
    println!("{}", "=".repeat(50));

    let events = repo.find_by_time_range(start_dt, end_dt)?;

    if events.is_empty() {
        println!("本月无事件。");
        return Ok(());
    }

    // Group by day
    let mut by_day: std::collections::BTreeMap<NaiveDate, Vec<_>> =
        std::collections::BTreeMap::new();

    for event in &events {
        let date = event.time.start.date_naive();
        by_day.entry(date).or_default().push(event);
    }

    for (date, day_events) in by_day {
        println!();
        println!("{}", date.format("%Y-%m-%d"));
        println!("{}", "-".repeat(30));

        for event in day_events {
            let time = event.time.start.format("%H:%M");
            let type_str = event.type_display_zh(&dicts);
            let summary = event.ai.summary.as_deref().unwrap_or("(无摘要)");
            println!("  {} [{}] {}", time, type_str, summary);
        }
    }

    println!();
    println!("共 {} 个事件", events.len());

    Ok(())
}
