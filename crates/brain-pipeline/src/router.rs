//! Model registry for task-type-based routing

use brain_core::adapters::{create_adapter, AdapterConfig, ModelAdapter};
use brain_core::{BrainConfig, PipelineTask, TaskType};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Clone)]
struct ModelEntry {
    config: AdapterConfig,
    adapter: Arc<dyn ModelAdapter>,
}

pub struct ModelRegistry {
    entries: HashMap<TaskType, Vec<ModelEntry>>,
    fallback: Option<ModelEntry>,
}

impl ModelRegistry {
    pub fn new(config: &BrainConfig) -> brain_core::Result<Self> {
        let mut entries: HashMap<TaskType, Vec<ModelEntry>> = HashMap::new();

        for adapter_config in &config.adapters {
            let adapter = create_adapter(adapter_config)?;

            // Health check - if fails, don't register this adapter
            if !adapter.health_check().unwrap_or(false) {
                warn!(
                    "Adapter {} ({}) not healthy, skipping",
                    adapter_config.adapter_type, adapter_config.default_model
                );
                continue;
            }

            let entry = ModelEntry {
                config: adapter_config.clone(),
                adapter,
            };

            for task_type in entry.adapter.supported_task_types() {
                entries.entry(task_type).or_default().push(entry.clone());
            }
        }

        let fallback = entries.values().flat_map(|v| v.iter()).next().cloned();

        info!(
            "ModelRegistry built with {} task types registered",
            entries.len()
        );

        Ok(Self { entries, fallback })
    }

    pub fn select(&self, task: &PipelineTask) -> Option<(Arc<dyn ModelAdapter>, &AdapterConfig)> {
        let required_data_type = task.data_type();

        if let Some(model_entries) = self.entries.get(&task.task) {
            for entry in model_entries {
                if entry
                    .adapter
                    .supported_data_types()
                    .contains(&required_data_type)
                {
                    return Some((Arc::clone(&entry.adapter), &entry.config));
                }
            }
        }

        self.fallback
            .as_ref()
            .map(|entry| (Arc::clone(&entry.adapter), &entry.config))
    }
}
