//! Entity commands

use brain_core::{BrainConfig, Database, EntityRepository, EntityType};

pub fn list(
    config: &BrainConfig,
    type_filter: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::open(&config.db_path)?;
    let conn = db.connection();
    let repo = EntityRepository::new(&conn);

    println!("Entities");
    println!("{}", "=".repeat(50));

    let entities = if let Some(type_str) = type_filter {
        let entity_type = match type_str.to_lowercase().as_str() {
            "person" => EntityType::Person,
            "organization" => EntityType::Organization,
            "project" => EntityType::Project,
            "artifact" => EntityType::Artifact,
            "concept" => EntityType::Concept,
            "topic" => EntityType::Topic,
            "activity" => EntityType::Activity,
            "goal" => EntityType::Goal,
            "skill" => EntityType::Skill,
            "place" => EntityType::Place,
            "device" => EntityType::Device,
            "resource" => EntityType::Resource,
            "memory_cluster" => EntityType::MemoryCluster,
            "state" => EntityType::State,
            _ => return Err(format!("Unknown entity type: {}", type_str).into()),
        };
        repo.find_by_type(&entity_type)?
    } else {
        repo.all()?
    };

    if entities.is_empty() {
        println!("未找到实体。");
        return Ok(());
    }

    // Group by type
    let mut by_type: std::collections::BTreeMap<String, Vec<_>> = std::collections::BTreeMap::new();

    for entity in &entities {
        let type_str = entity.type_.display_zh().to_string();
        by_type.entry(type_str).or_default().push(entity);
    }

    for (type_str, type_entities) in by_type {
        println!();
        println!("{} ({})", type_str, type_entities.len());
        println!("{}", "-".repeat(30));

        for entity in type_entities {
            let status = match entity.status {
                brain_core::EntityStatus::Active => "",
                brain_core::EntityStatus::Archived => " [已归档]",
                brain_core::EntityStatus::Merged => " [已合并]",
            };
            println!("  {} - {}{}", entity.id, entity.label, status);
        }
    }

    println!();
    println!("共 {} 个实体", entities.len());

    Ok(())
}

pub fn show(config: &BrainConfig, id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::open(&config.db_path)?;
    let conn = db.connection();
    let repo = EntityRepository::new(&conn);

    let entity = repo.find_by_id(id)?;

    match entity {
        Some(entity) => {
            println!("Entity: {}", entity.id);
            println!("{}", "=".repeat(50));
            println!("类型: {}", entity.type_.display_zh());
            println!("标签: {}", entity.label);

            if !entity.aliases.is_empty() {
                println!("别名: {}", entity.aliases.join(", "));
            }

            let status = match entity.status {
                brain_core::EntityStatus::Active => "活跃",
                brain_core::EntityStatus::Archived => "已归档",
                brain_core::EntityStatus::Merged => "已合并",
            };
            println!("状态: {}", status);
            println!("置信度: {:.2}", entity.confidence);

            if let Some(ref domain) = entity.classification.domain {
                println!("领域: {}", domain);
            }

            if !entity.classification.parent.is_empty() {
                println!("父级: {}", entity.classification.parent.join(", "));
            }

            if let Some(ref desc) = entity.identity.description {
                println!("描述: {}", desc);
            }

            if let Some(ref summary) = entity.identity.summary {
                println!("摘要: {}", summary);
            }

            println!("指标:");
            println!("  事件数量: {}", entity.metrics.event_count);
            if let Some(ref last_seen) = entity.metrics.last_seen {
                println!("  最近出现: {}", last_seen.format("%Y-%m-%d %H:%M"));
            }
            if let Some(ref score) = entity.metrics.activity_score {
                println!("  活跃度: {:.2}", score);
            }
        }
        None => {
            println!("未找到实体: {}", id);
        }
    }

    Ok(())
}
