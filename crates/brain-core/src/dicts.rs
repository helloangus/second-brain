//! Dictionary system for reusable field values
//!
//! Manages device, channel, capture_agent, event_type, event_subtype, tags, and topics
//! stored in brain-data/dicts/

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// A single dictionary entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictEntry {
    /// The key (value itself) - defaults to empty, filled by Dict visitor
    #[serde(default)]
    pub key: String,
    /// Optional Chinese translation
    #[serde(default)]
    pub zh: Option<String>,
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
}

impl DictEntry {
    pub fn new(key: &str) -> Self {
        Self {
            key: key.to_string(),
            zh: None,
            description: None,
        }
    }

    pub fn with_zh(mut self, zh: &str) -> Self {
        self.zh = Some(zh.to_string());
        self
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }
}

/// A dictionary of entries
#[derive(Debug, Clone, Default)]
pub struct Dict {
    entries: HashMap<String, DictEntry>,
}

impl Serialize for Dict {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as a map where key is the entry key and value is {zh, description}
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.entries.len()))?;
        for (key, entry) in &self.entries {
            // Skip the key field in serialization, only serialize zh and description
            #[derive(Serialize)]
            struct EntryView<'a> {
                zh: &'a Option<String>,
                description: &'a Option<String>,
            }
            let view = EntryView {
                zh: &entry.zh,
                description: &entry.description,
            };
            map.serialize_entry(key, &view)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for Dict {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Use a flat map visitor that captures both key and value
        deserializer.deserialize_map(DictFlatVisitor)
    }
}

/// Visitor that handles format: PC: { zh: ..., description: ... } where YAML key is the entry key
struct DictFlatVisitor;

impl<'de> serde::de::Visitor<'de> for DictFlatVisitor {
    type Value = Dict;

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "a dictionary with entries")
    }

    fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut dict = Dict::default();
        while let Some(yaml_key) = map.next_key::<String>()? {
            // Try to read the value as DictEntry
            let entry: DictEntry = map.next_value()?;
            // Use the YAML key as the entry key ( DictEntry.key may be empty or different)
            let mut entry = entry;
            entry.key = yaml_key;
            dict.add(entry);
        }
        Ok(dict)
    }
}

impl Dict {
    /// Load a dictionary from a YAML file
    pub fn load(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path)?;
        let dict: Dict = serde_yaml::from_str(&content).map_err(|e| {
            Error::Config(format!("Failed to parse dict {}: {}", path.display(), e))
        })?;
        Ok(dict)
    }

    /// Save a dictionary to a YAML file
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_yaml::to_string(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Check if a key exists
    pub fn exists(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// Look up an entry by key
    pub fn lookup(&self, key: &str) -> Option<&DictEntry> {
        self.entries.get(key)
    }

    /// List all entries
    pub fn list(&self) -> Vec<&DictEntry> {
        self.entries.values().collect()
    }

    /// Add a new entry
    pub fn add(&mut self, entry: DictEntry) {
        self.entries.insert(entry.key.clone(), entry);
    }

    /// Remove an entry
    pub fn remove(&mut self, key: &str) -> bool {
        self.entries.remove(key).is_some()
    }

    /// Get all keys
    pub fn keys(&self) -> Vec<&String> {
        self.entries.keys().collect()
    }
}

/// All dictionaries loaded together
#[derive(Debug, Clone)]
pub struct DictSet {
    pub device: Dict,
    pub channel: Dict,
    pub capture_agent: Dict,
    pub event_type: Dict,
    pub event_subtype: Dict,
    pub tags: Dict,
    pub topics: Dict,
}

impl DictSet {
    /// Load all dictionaries from the dicts directory
    pub fn load(dicts_path: &PathBuf) -> Result<Self> {
        Ok(Self {
            device: Dict::load(&dicts_path.join("device.yaml"))?,
            channel: Dict::load(&dicts_path.join("channel.yaml"))?,
            capture_agent: Dict::load(&dicts_path.join("capture_agent.yaml"))?,
            event_type: Dict::load(&dicts_path.join("event_type.yaml"))?,
            event_subtype: Dict::load(&dicts_path.join("event_subtype.yaml"))?,
            tags: Dict::load(&dicts_path.join("tags.yaml"))?,
            topics: Dict::load(&dicts_path.join("topics.yaml"))?,
        })
    }

    /// Save all dictionaries
    pub fn save(&self, dicts_path: &PathBuf) -> Result<()> {
        self.device.save(&dicts_path.join("device.yaml"))?;
        self.channel.save(&dicts_path.join("channel.yaml"))?;
        self.capture_agent.save(&dicts_path.join("capture_agent.yaml"))?;
        self.event_type.save(&dicts_path.join("event_type.yaml"))?;
        self.event_subtype.save(&dicts_path.join("event_subtype.yaml"))?;
        self.tags.save(&dicts_path.join("tags.yaml"))?;
        self.topics.save(&dicts_path.join("topics.yaml"))?;
        Ok(())
    }

    /// Create default dictionaries with common values
    pub fn default_dicts() -> Self {
        let mut device = Dict::default();
        device.add(DictEntry::new("PC").with_zh("个人电脑").with_description("本地桌面或笔记本设备"));
        device.add(DictEntry::new("iPhone").with_zh("iPhone").with_description("苹果手机"));
        device.add(DictEntry::new("Android").with_zh("安卓手机").with_description("安卓手机设备"));
        device.add(DictEntry::new("Server").with_zh("服务器").with_description("远程服务器"));

        let mut channel = Dict::default();
        channel.add(DictEntry::new("CLI").with_zh("命令行").with_description("通过命令行界面输入"));
        channel.add(DictEntry::new("API").with_zh("API接口").with_description("通过编程接口输入"));
        channel.add(DictEntry::new("Web").with_zh("网页").with_description("通过网页界面输入"));

        let mut capture_agent = Dict::default();
        capture_agent.add(DictEntry::new("manual_entry").with_zh("手动录入").with_description("用户手动输入"));
        capture_agent.add(DictEntry::new("pipeline").with_zh("AI流水线").with_description("AI自动处理生成"));
        capture_agent.add(DictEntry::new("sync_service").with_zh("同步服务").with_description("第三方服务同步"));

        let mut event_type = Dict::default();
        event_type.add(DictEntry::new("note").with_zh("笔记").with_description("普通笔记"));
        event_type.add(DictEntry::new("task").with_zh("任务").with_description("待办或已完成的任务"));
        event_type.add(DictEntry::new("research").with_zh("研究").with_description("研究或学习相关"));
        event_type.add(DictEntry::new("photo").with_zh("照片").with_description("照片或图像记录"));

        let mut event_subtype = Dict::default();
        event_subtype.add(DictEntry::new("summarize").with_zh("摘要").with_description("AI生成的摘要"));
        event_subtype.add(DictEntry::new("reasoning").with_zh("推理").with_description("AI推理分析"));
        event_subtype.add(DictEntry::new("image_caption").with_zh("图片说明").with_description("图片描述生成"));

        let mut tags = Dict::default();
        tags.add(DictEntry::new("AI").with_zh("人工智能"));
        tags.add(DictEntry::new("Rust").with_zh("Rust编程"));
        tags.add(DictEntry::new("project").with_zh("项目"));
        tags.add(DictEntry::new("notes").with_zh("笔记"));

        let mut topics = Dict::default();
        topics.add(DictEntry::new("machine_learning").with_zh("机器学习"));
        topics.add(DictEntry::new("programming").with_zh("编程"));
        topics.add(DictEntry::new("life").with_zh("生活"));

        Self {
            device,
            channel,
            capture_agent,
            event_type,
            event_subtype,
            tags,
            topics,
        }
    }

    /// Initialize default dictionaries if they don't exist
    pub fn init_if_missing(dicts_path: &PathBuf) -> Result<()> {
        if !dicts_path.exists() {
            fs::create_dir_all(dicts_path)?;
            let defaults = Self::default_dicts();
            defaults.save(dicts_path)?;
        }
        Ok(())
    }
}

/// Prompt user to select from existing values or create new
/// Returns (selected_value, is_new) where is_new=true if user chose to create a new value
pub fn prompt_selection(dict: &Dict, input: &str, dict_name: &str) -> Result<(String, bool)> {
    if dict.exists(input) {
        return Ok((input.to_string(), false));
    }

    println!("Value '{}' not found in {} dictionary.", input, dict_name);
    println!("\nExisting {} values:", dict_name);
    for entry in dict.list() {
        let zh = entry.zh.as_deref().unwrap_or("");
        if zh.is_empty() {
            println!("  - {}", entry.key);
        } else {
            println!("  - {} ({})", entry.key, zh);
        }
    }
    println!("\nEnter 'new' to create '{}' as a new value, or enter an existing value to use it:", input);

    // Read from stdin
    let mut choice = String::new();
    std::io::stdin().read_line(&mut choice)?;
    let choice = choice.trim();

    if choice.eq_ignore_ascii_case("new") {
        Ok((input.to_string(), true)) // true = newly created
    } else if dict.exists(choice) {
        Ok((choice.to_string(), false)) // false = existing
    } else {
        Err(Error::Config(format!("Invalid selection: {}", choice)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dict_entry() {
        let entry = DictEntry::new("test").with_zh("测试").with_description("A test entry");
        assert_eq!(entry.key, "test");
        assert_eq!(entry.zh, Some("测试".to_string()));
        assert_eq!(entry.description, Some("A test entry".to_string()));
    }

    #[test]
    fn test_dict_operations() {
        let mut dict = Dict::default();
        assert!(!dict.exists("test"));

        dict.add(DictEntry::new("test").with_zh("测试"));
        assert!(dict.exists("test"));
        assert_eq!(dict.lookup("test").unwrap().zh.as_deref(), Some("测试"));

        let keys = dict.keys();
        assert!(keys.contains(&&"test".to_string()));
    }
}