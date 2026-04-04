//! Stats command

use brain_core::{BrainConfig, Database, DictSet, EntityRepository, EventRepository};

pub fn execute(config: &BrainConfig) -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::open(&config.db_path)?;
    let conn = db.connection();

    let event_repo = EventRepository::new(&conn);
    let entity_repo = EntityRepository::new(&conn);

    // Load dictionaries for Chinese display
    let dicts = DictSet::load(&config.dicts_path).unwrap_or_else(|_| DictSet::default_dicts());

    let events = event_repo.all()?;
    let entities = entity_repo.all()?;

    println!("Second Brain Statistics");
    println!("{}", "=".repeat(50));

    // Event stats
    println!();
    println!("事件: {}", events.len());

    // Event type distribution
    let mut type_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for event in &events {
        *type_counts
            .entry(event.type_display_zh(&dicts))
            .or_insert(0) += 1;
    }

    println!("  按类型:");
    let mut type_vec: Vec<_> = type_counts.iter().collect();
    type_vec.sort_by(|a, b| b.1.cmp(a.1));
    for (type_, count) in type_vec {
        println!("    {}: {}", type_, count);
    }

    // Entity stats
    println!();
    println!("实体: {}", entities.len());

    // Entity type distribution
    let mut entity_type_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for entity in &entities {
        *entity_type_counts
            .entry(entity.type_.display_zh().to_string())
            .or_insert(0) += 1;
    }

    println!("  按类型:");
    let mut entity_type_vec: Vec<_> = entity_type_counts.iter().collect();
    entity_type_vec.sort_by(|a, b| b.1.cmp(a.1));
    for (type_, count) in entity_type_vec {
        println!("    {}: {}", type_, count);
    }

    // Tag stats
    let mut tag_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for event in &events {
        for tag in &event.tags {
            *tag_counts.entry(tag.clone()).or_insert(0) += 1;
        }
    }

    println!();
    println!("热门标签:");
    let mut tag_vec: Vec<_> = tag_counts.iter().collect();
    tag_vec.sort_by(|a, b| b.1.cmp(a.1));
    for (tag, count) in tag_vec.into_iter().take(10) {
        println!("  {}: {}", tag, count);
    }

    Ok(())
}
