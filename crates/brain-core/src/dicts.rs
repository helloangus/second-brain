//! Dictionary system for reusable field values
//!
//! # Overview
//! Dictionaries provide controlled vocabularies for event metadata fields.
//! Each dictionary maps unique keys to optional Chinese translations and descriptions.
//!
//! # Dictionary Types
//! - **device**: Hardware devices (desktop, laptop, mobile, tablet, server, cloud)
//! - **channel**: Input channels (cli, api, web, mobile, desktop)
//! - **capture_agent**: How data was captured (manual, automated, imported, synced)
//! - **event_type**: Event categories (note, task, idea, meeting, communication, media, document)
//! - **event_subtype**: Event subcategories (summary, analysis, followup, reference, caption)
//! - **tags**: Categorization labels (personal, work, learning, health, finance, social)
//! - **topics**: Subject domains (technology, science, art, business, life, philosophy)
//!
//! # Storage
//! Each dictionary is stored as a YAML file in `brain-data/dicts/`:
//! ```yaml
//! desktop:
//!   zh: 台式机
//!   description: 桌面电脑设备
//! laptop:
//!   zh: 笔记本
//!   description: 笔记本电脑设备
//! ```
//!
//! # Usage
//! - Keys are unique identifiers used in event frontmatter
//! - Chinese translations enable bilingual UI display
//! - Descriptions provide human-readable explanations

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A single entry in a dictionary.
///
/// Contains a unique key, optional Chinese translation, and optional description.
/// The key serves as both the identifier and the default display value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictEntry {
    /// The unique key identifier (used in event frontmatter as field values)
    #[serde(default)]
    pub key: String,

    /// Optional Chinese translation for bilingual display
    #[serde(default)]
    pub zh: Option<String>,

    /// Optional human-readable description explaining the entry's meaning
    #[serde(default)]
    pub description: Option<String>,
}

impl DictEntry {
    /// Create a new entry with the given key.
    ///
    /// # Examples
    /// ```
    /// use brain_core::DictEntry;
    /// let entry = DictEntry::new("desktop");
    /// assert_eq!(entry.key, "desktop");
    /// assert!(entry.zh.is_none());
    /// assert!(entry.description.is_none());
    /// ```
    pub fn new(key: &str) -> Self {
        Self {
            key: key.to_string(),
            zh: None,
            description: None,
        }
    }

    /// Set the Chinese translation.
    ///
    /// # Examples
    /// ```
    /// use brain_core::DictEntry;
    /// let entry = DictEntry::new("desktop").with_zh("台式机");
    /// assert_eq!(entry.zh, Some("台式机".to_string()));
    /// ```
    pub fn with_zh(mut self, zh: &str) -> Self {
        self.zh = Some(zh.to_string());
        self
    }

    /// Set the description.
    ///
    /// # Examples
    /// ```
    /// use brain_core::DictEntry;
    /// let entry = DictEntry::new("desktop").with_description("桌面电脑设备");
    /// assert_eq!(entry.description, Some("桌面电脑设备".to_string()));
    /// ```
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }
}

/// A dictionary mapping unique keys to `DictEntry` values.
///
/// Internally stored as a HashMap for O(1) lookups.
/// Supports YAML serialization/deserialization with a flat key format.
#[derive(Debug, Clone, Default)]
pub struct Dict {
    entries: HashMap<String, DictEntry>,
}

// ---------------------------------------------------------------------------
// Serialization
// ---------------------------------------------------------------------------

/// Custom serializer for Dict.
///
/// Serializes as a flat YAML map where the YAML key is the entry key,
/// and the value is a nested object containing `zh` and `description`.
/// The entry's own `key` field is omitted since it's redundant.
///
/// # YAML Format
/// ```yaml
/// desktop:
///   zh: 台式机
///   description: 桌面电脑设备
/// ```
impl Serialize for Dict {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.entries.len()))?;
        for (key, entry) in &self.entries {
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

/// Custom deserializer for Dict.
///
/// Uses a flat map visitor that captures both the YAML key and value.
/// The YAML key becomes the entry's `key` field, while `zh` and `description`
/// come from the nested value object.
impl<'de> Deserialize<'de> for Dict {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(DictFlatVisitor)
    }
}

/// Visitor for deserializing flat dictionary format.
///
/// Handles: `key: { zh: "...", description: "..." }`
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
            let entry: DictEntry = map.next_value()?;
            let mut entry = entry;
            entry.key = yaml_key;
            dict.add(entry);
        }
        Ok(dict)
    }
}

// ---------------------------------------------------------------------------
// Dict Operations
// ---------------------------------------------------------------------------

impl Dict {
    /// Load a dictionary from a YAML file.
    ///
    /// Returns an empty dictionary if the file does not exist.
    /// Returns an error if the file exists but cannot be parsed.
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

    /// Save a dictionary to a YAML file.
    ///
    /// Creates parent directories if they do not exist.
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_yaml::to_string(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Check if a key exists in the dictionary.
    pub fn exists(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// Look up an entry by key. Returns None if not found.
    pub fn lookup(&self, key: &str) -> Option<&DictEntry> {
        self.entries.get(key)
    }

    /// List all entries as a vector. Order is unspecified.
    pub fn list(&self) -> Vec<&DictEntry> {
        self.entries.values().collect()
    }

    /// Add a new entry. Overwrites existing entry with the same key.
    pub fn add(&mut self, entry: DictEntry) {
        self.entries.insert(entry.key.clone(), entry);
    }

    /// Remove an entry by key. Returns true if an entry was removed.
    pub fn remove(&mut self, key: &str) -> bool {
        self.entries.remove(key).is_some()
    }

    /// Get all keys as a vector. Order is unspecified.
    pub fn keys(&self) -> Vec<&String> {
        self.entries.keys().collect()
    }
}

// ---------------------------------------------------------------------------
// DictSet
// ---------------------------------------------------------------------------

/// A collection of all dictionaries loaded together.
///
/// Provides convenient access to all seven dictionary types:
/// device, channel, capture_agent, event_type, event_subtype, tags, topics.
#[derive(Debug, Clone)]
pub struct DictSet {
    /// Hardware devices (desktop, laptop, mobile, tablet, server, cloud)
    pub device: Dict,

    /// Input channels (cli, api, web, mobile, desktop)
    pub channel: Dict,

    /// How data was captured (manual, automated, imported, synced)
    pub capture_agent: Dict,

    /// Event categories (note, task, idea, meeting, communication, media, document)
    pub event_type: Dict,

    /// Event subcategories (summary, analysis, followup, reference, caption)
    pub event_subtype: Dict,

    /// Categorization labels (personal, work, learning, health, finance, social)
    pub tags: Dict,

    /// Subject domains (technology, science, art, business, life, philosophy)
    pub topics: Dict,
}

impl DictSet {
    /// Load all dictionaries from a directory.
    ///
    /// Each dictionary is loaded from a separate YAML file:
    /// `device.yaml`, `channel.yaml`, `capture_agent.yaml`, `event_type.yaml`,
    /// `event_subtype.yaml`, `tags.yaml`, `topics.yaml`.
    ///
    /// Missing files are treated as empty dictionaries.
    pub fn load(dicts_path: &Path) -> Result<Self> {
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

    /// Save all dictionaries to a directory.
    ///
    /// Creates the directory if it does not exist.
    /// Each dictionary is saved as a separate YAML file.
    pub fn save(&self, dicts_path: &Path) -> Result<()> {
        self.device.save(&dicts_path.join("device.yaml"))?;
        self.channel.save(&dicts_path.join("channel.yaml"))?;
        self.capture_agent
            .save(&dicts_path.join("capture_agent.yaml"))?;
        self.event_type.save(&dicts_path.join("event_type.yaml"))?;
        self.event_subtype
            .save(&dicts_path.join("event_subtype.yaml"))?;
        self.tags.save(&dicts_path.join("tags.yaml"))?;
        self.topics.save(&dicts_path.join("topics.yaml"))?;
        Ok(())
    }

    /// Find an entry by key or Chinese translation.
    ///
    /// First attempts direct key lookup. If not found, searches Chinese translations.
    /// Returns the matching entry, or None if not found in either.
    ///
    /// # Arguments
    /// * `dict_name` - One of: "device", "channel", "capture_agent", "event_type",
    ///   "event_subtype", "tags", "topics"
    /// * `key_or_zh` - The key string or Chinese translation to search for
    ///
    /// # Examples
    /// ```
    /// use brain_core::DictSet;
    /// let dict_set = DictSet::default_dicts();
    /// dict_set.find_entry("device", "desktop");   // finds by key
    /// dict_set.find_entry("device", "台式机");      // finds by zh
    /// dict_set.find_entry("device", "unknown");    // returns None
    /// ```
    pub fn find_entry(&self, dict_name: &str, key_or_zh: &str) -> Option<&DictEntry> {
        let dict = match dict_name {
            "device" => &self.device,
            "channel" => &self.channel,
            "capture_agent" => &self.capture_agent,
            "event_type" => &self.event_type,
            "event_subtype" => &self.event_subtype,
            "tags" => &self.tags,
            "topics" => &self.topics,
            _ => return None,
        };

        // Try direct key match first
        if let Some(entry) = dict.lookup(key_or_zh) {
            return Some(entry);
        }

        // Try Chinese translation match
        for entry in dict.list() {
            if let Some(zh) = &entry.zh {
                if zh == key_or_zh {
                    return Some(entry);
                }
            }
        }
        None
    }

    /// Create default dictionaries with generic, universally applicable values.
    pub fn default_dicts() -> Self {
        let mut device = Dict::default();
        device.add(
            DictEntry::new("desktop")
                .with_zh("台式机")
                .with_description("桌面电脑设备"),
        );
        device.add(
            DictEntry::new("laptop")
                .with_zh("笔记本")
                .with_description("笔记本电脑设备"),
        );
        device.add(
            DictEntry::new("mobile")
                .with_zh("手机")
                .with_description("移动设备"),
        );
        device.add(
            DictEntry::new("tablet")
                .with_zh("平板")
                .with_description("平板电脑设备"),
        );
        device.add(
            DictEntry::new("server")
                .with_zh("服务器")
                .with_description("远程服务器"),
        );
        device.add(
            DictEntry::new("cloud")
                .with_zh("云端")
                .with_description("云服务"),
        );

        let mut channel = Dict::default();
        channel.add(
            DictEntry::new("cli")
                .with_zh("命令行")
                .with_description("命令行界面输入"),
        );
        channel.add(
            DictEntry::new("api")
                .with_zh("API")
                .with_description("编程接口输入"),
        );
        channel.add(
            DictEntry::new("web")
                .with_zh("网页")
                .with_description("网页界面输入"),
        );
        channel.add(
            DictEntry::new("mobile")
                .with_zh("移动端")
                .with_description("移动应用输入"),
        );
        channel.add(
            DictEntry::new("desktop")
                .with_zh("桌面端")
                .with_description("桌面应用输入"),
        );

        let mut capture_agent = Dict::default();
        capture_agent.add(
            DictEntry::new("manual")
                .with_zh("手动")
                .with_description("用户手动输入"),
        );
        capture_agent.add(
            DictEntry::new("automated")
                .with_zh("自动")
                .with_description("系统自动生成"),
        );
        capture_agent.add(
            DictEntry::new("imported")
                .with_zh("导入")
                .with_description("从外部导入"),
        );
        capture_agent.add(
            DictEntry::new("synced")
                .with_zh("同步")
                .with_description("第三方服务同步"),
        );

        let mut event_type = Dict::default();
        // Event type categories based on "time-anchored cognitive fact" principle:
        // describes WHAT KIND OF COGNITIVE OCCURRENCE happened
        event_type.add(
            DictEntry::new("observation")
                .with_zh("观察")
                .with_description("看到、注意到、感知到某事（拍照、截图、视觉输入）"),
        );
        event_type.add(
            DictEntry::new("communication")
                .with_zh("沟通")
                .with_description("与他人交流（会议、消息、讨论、邮件）"),
        );
        event_type.add(
            DictEntry::new("learning")
                .with_zh("学习")
                .with_description("获取知识（阅读、研究、课程）"),
        );
        event_type.add(
            DictEntry::new("creation")
                .with_zh("创作")
                .with_description("产生新内容（写作、编程、制图）"),
        );
        event_type.add(
            DictEntry::new("idea")
                .with_zh("想法")
                .with_description("思考、反思、灵感、结论"),
        );
        event_type.add(
            DictEntry::new("action")
                .with_zh("行为")
                .with_description("执行具体行动（锻炼、用餐、工作、任务）"),
        );
        event_type.add(
            DictEntry::new("transaction")
                .with_zh("交易")
                .with_description("交换、获取、失去某物（购买、接收、丢失）"),
        );
        event_type.add(
            DictEntry::new("state_change")
                .with_zh("状态变化")
                .with_description("开始、结束、到达、离开、转变"),
        );

        let mut event_subtype = Dict::default();
        event_subtype.add(
            DictEntry::new("summary")
                .with_zh("摘要")
                .with_description("内容摘要"),
        );
        event_subtype.add(
            DictEntry::new("analysis")
                .with_zh("分析")
                .with_description("分析内容"),
        );
        event_subtype.add(
            DictEntry::new("followup")
                .with_zh("跟进")
                .with_description("后续跟进事项"),
        );
        event_subtype.add(
            DictEntry::new("reference")
                .with_zh("参考")
                .with_description("参考资料"),
        );
        event_subtype.add(
            DictEntry::new("caption")
                .with_zh("说明")
                .with_description("描述或说明"),
        );

        // Tags: labels for categorizing events (distinct from event_type)
        let mut tags = Dict::default();
        tags.add(DictEntry::new("personal").with_zh("个人"));
        tags.add(DictEntry::new("work").with_zh("工作"));
        tags.add(DictEntry::new("learning").with_zh("学习"));
        tags.add(DictEntry::new("health").with_zh("健康"));
        tags.add(DictEntry::new("finance").with_zh("财务"));
        tags.add(DictEntry::new("social").with_zh("社交"));

        // Topics: subject domains or themes
        let mut topics = Dict::default();
        topics.add(DictEntry::new("technology").with_zh("技术"));
        topics.add(DictEntry::new("science").with_zh("科学"));
        topics.add(DictEntry::new("art").with_zh("艺术"));
        topics.add(DictEntry::new("business").with_zh("商业"));
        topics.add(DictEntry::new("life").with_zh("生活"));
        topics.add(DictEntry::new("philosophy").with_zh("哲学"));

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

    /// Initialize default dictionaries if they don't exist.
    ///
    /// If `dicts_path` does not exist, creates the directory and saves all
    /// default dictionaries. If the directory already exists, does nothing.
    pub fn init_if_missing(dicts_path: &PathBuf) -> Result<()> {
        if !dicts_path.exists() {
            fs::create_dir_all(dicts_path)?;
            let defaults = Self::default_dicts();
            defaults.save(dicts_path)?;
        }
        Ok(())
    }
}

/// Prompt user to select an existing value or create a new one.
///
/// Returns `(selected_value, is_new)` where:
/// - `is_new` is `true` if the user chose to create a new value
/// - `is_new` is `false` if the user selected an existing value
///
/// # Arguments
/// * `dict` - The dictionary to search in
/// * `input` - The value the user provided that was not found
/// * `dict_name` - The name of the dictionary (for display purposes)
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
    println!(
        "\nEnter 'new' to create '{}' as a new value, or enter an existing value to use it:",
        input
    );

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
    use tempfile::TempDir;

    // ---------------------------------------------------------------------------
    // DictEntry Tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_dict_entry_new() {
        let entry = DictEntry::new("desktop");
        assert_eq!(entry.key, "desktop");
        assert!(entry.zh.is_none());
        assert!(entry.description.is_none());
    }

    #[test]
    fn test_dict_entry_with_zh() {
        let entry = DictEntry::new("desktop").with_zh("台式机");
        assert_eq!(entry.key, "desktop");
        assert_eq!(entry.zh, Some("台式机".to_string()));
    }

    #[test]
    fn test_dict_entry_with_description() {
        let entry = DictEntry::new("desktop").with_description("桌面电脑设备");
        assert_eq!(entry.description, Some("桌面电脑设备".to_string()));
    }

    #[test]
    fn test_dict_entry_builder_pattern() {
        let entry = DictEntry::new("desktop")
            .with_zh("台式机")
            .with_description("桌面电脑设备");
        assert_eq!(entry.key, "desktop");
        assert_eq!(entry.zh, Some("台式机".to_string()));
        assert_eq!(entry.description, Some("桌面电脑设备".to_string()));
    }

    // ---------------------------------------------------------------------------
    // Dict Basic Operations Tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_dict_default_is_empty() {
        let dict = Dict::default();
        assert!(!dict.exists("any_key"));
        assert!(dict.lookup("any_key").is_none());
        assert!(dict.list().is_empty());
        assert!(dict.keys().is_empty());
    }

    #[test]
    fn test_dict_add_and_exists() {
        let mut dict = Dict::default();
        assert!(!dict.exists("desktop"));

        dict.add(DictEntry::new("desktop").with_zh("台式机"));
        assert!(dict.exists("desktop"));
        assert!(!dict.exists("laptop")); // still false
    }

    #[test]
    fn test_dict_lookup() {
        let mut dict = Dict::default();
        dict.add(DictEntry::new("desktop").with_zh("台式机"));
        dict.add(DictEntry::new("laptop").with_zh("笔记本"));

        let entry = dict.lookup("desktop");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().zh.as_deref(), Some("台式机"));

        assert!(dict.lookup("nonexistent").is_none());
    }

    #[test]
    fn test_dict_list() {
        let mut dict = Dict::default();
        dict.add(DictEntry::new("a"));
        dict.add(DictEntry::new("b"));
        dict.add(DictEntry::new("c"));

        let entries = dict.list();
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn test_dict_keys() {
        let mut dict = Dict::default();
        dict.add(DictEntry::new("desktop"));
        dict.add(DictEntry::new("laptop"));

        let keys = dict.keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.iter().any(|k| *k == "desktop"));
        assert!(keys.iter().any(|k| *k == "laptop"));
    }

    #[test]
    fn test_dict_remove() {
        let mut dict = Dict::default();
        dict.add(DictEntry::new("desktop"));

        assert!(dict.remove("desktop"));
        assert!(!dict.exists("desktop"));
        assert!(!dict.remove("nonexistent")); // returns false
    }

    #[test]
    fn test_dict_add_overwrites() {
        let mut dict = Dict::default();
        dict.add(
            DictEntry::new("desktop")
                .with_zh("台式机1")
                .with_description("desc1"),
        );
        dict.add(
            DictEntry::new("desktop")
                .with_zh("台式机2")
                .with_description("desc2"),
        );

        // Should have only one entry, with latest values
        assert_eq!(dict.keys().len(), 1);
        let entry = dict.lookup("desktop").unwrap();
        assert_eq!(entry.zh.as_deref(), Some("台式机2"));
        assert_eq!(entry.description.as_deref(), Some("desc2"));
    }

    // ---------------------------------------------------------------------------
    // Dict Serialization/Deserialization Tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_dict_serialize_deserialize() {
        let mut dict = Dict::default();
        dict.add(
            DictEntry::new("desktop")
                .with_zh("台式机")
                .with_description("桌面电脑设备"),
        );
        dict.add(DictEntry::new("laptop").with_zh("笔记本"));

        let yaml = serde_yaml::to_string(&dict).unwrap();
        let deserialized: Dict = serde_yaml::from_str(&yaml).unwrap();

        assert!(deserialized.exists("desktop"));
        assert!(deserialized.exists("laptop"));

        let desktop = deserialized.lookup("desktop").unwrap();
        assert_eq!(desktop.key, "desktop");
        assert_eq!(desktop.zh.as_deref(), Some("台式机"));
        assert_eq!(desktop.description.as_deref(), Some("桌面电脑设备"));

        let laptop = deserialized.lookup("laptop").unwrap();
        assert_eq!(laptop.zh.as_deref(), Some("笔记本"));
        assert!(laptop.description.is_none());
    }

    #[test]
    fn test_dict_roundtrip_empty() {
        let dict = Dict::default();
        let yaml = serde_yaml::to_string(&dict).unwrap();
        let deserialized: Dict = serde_yaml::from_str(&yaml).unwrap();
        assert!(deserialized.list().is_empty());
    }

    #[test]
    fn test_dict_load_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("nonexistent.yaml");

        let dict = Dict::load(&path).unwrap();
        assert!(dict.list().is_empty());
    }

    #[test]
    fn test_dict_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test_dict.yaml");

        let mut dict = Dict::default();
        dict.add(
            DictEntry::new("server")
                .with_zh("服务器")
                .with_description("远程服务器"),
        );

        dict.save(&path).unwrap();
        let loaded = Dict::load(&path).unwrap();

        assert!(loaded.exists("server"));
        let entry = loaded.lookup("server").unwrap();
        assert_eq!(entry.zh.as_deref(), Some("服务器"));
        assert_eq!(entry.description.as_deref(), Some("远程服务器"));
    }

    // ---------------------------------------------------------------------------
    // DictSet Tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_default_dicts_all_have_entries() {
        let dicts = DictSet::default_dicts();

        // Each dict should have at least one entry
        assert!(!dicts.device.list().is_empty());
        assert!(!dicts.channel.list().is_empty());
        assert!(!dicts.capture_agent.list().is_empty());
        assert!(!dicts.event_type.list().is_empty());
        assert!(!dicts.event_subtype.list().is_empty());
        assert!(!dicts.tags.list().is_empty());
        assert!(!dicts.topics.list().is_empty());
    }

    #[test]
    fn test_default_dicts_specific_entries() {
        let dicts = DictSet::default_dicts();

        // device
        assert!(dicts.device.exists("desktop"));
        assert!(dicts.device.exists("mobile"));
        assert!(dicts.device.exists("cloud"));

        // channel
        assert!(dicts.channel.exists("cli"));
        assert!(dicts.channel.exists("web"));

        // event_type
        assert!(dicts.event_type.exists("observation"));
        assert!(dicts.event_type.exists("communication"));

        // tags
        assert!(dicts.tags.exists("personal"));
        assert!(dicts.tags.exists("work"));

        // topics
        assert!(dicts.topics.exists("technology"));
        assert!(dicts.topics.exists("life"));
    }

    #[test]
    fn test_dictset_find_entry_by_key() {
        let dicts = DictSet::default_dicts();

        let entry = dicts.find_entry("device", "desktop");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().key, "desktop");
    }

    #[test]
    fn test_dictset_find_entry_by_zh() {
        let dicts = DictSet::default_dicts();

        // Find by Chinese translation
        let entry = dicts.find_entry("device", "台式机");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().key, "desktop");
    }

    #[test]
    fn test_dictset_find_entry_not_found() {
        let dicts = DictSet::default_dicts();

        assert!(dicts.find_entry("device", "nonexistent").is_none());
        assert!(dicts.find_entry("invalid_dict", "desktop").is_none());
    }

    #[test]
    fn test_dictset_find_entry_in_tags() {
        let dicts = DictSet::default_dicts();

        let entry = dicts.find_entry("tags", "personal");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().zh.as_deref(), Some("个人"));

        let entry_zh = dicts.find_entry("tags", "工作");
        assert!(entry_zh.is_some());
        assert_eq!(entry_zh.unwrap().key, "work");
    }

    #[test]
    fn test_dictset_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let dicts_path = temp_dir.path();

        let dicts = DictSet::default_dicts();
        dicts.save(dicts_path).unwrap();

        let loaded = DictSet::load(dicts_path).unwrap();

        // Verify some entries
        assert!(loaded.device.exists("desktop"));
        assert!(loaded.channel.exists("cli"));
        assert!(loaded.event_type.exists("observation"));
    }

    #[test]
    fn test_dictset_init_if_missing_creates_files() {
        let temp_dir = TempDir::new().unwrap();
        let dicts_path = temp_dir.path().join("dicts");

        // Should not exist initially
        assert!(!dicts_path.exists());

        DictSet::init_if_missing(&dicts_path).unwrap();

        // Now should exist
        assert!(dicts_path.exists());

        // Should have all the YAML files
        assert!(dicts_path.join("device.yaml").exists());
        assert!(dicts_path.join("channel.yaml").exists());
        assert!(dicts_path.join("event_type.yaml").exists());

        // And they should have content
        let dicts = DictSet::load(&dicts_path).unwrap();
        assert!(!dicts.device.list().is_empty());
    }

    #[test]
    fn test_dictset_init_if_missing_does_nothing_if_exists() {
        let temp_dir = TempDir::new().unwrap();
        let dicts_path = temp_dir.path().join("dicts");

        // Create directory first
        std::fs::create_dir_all(&dicts_path).unwrap();

        // Put a custom file
        std::fs::write(
            dicts_path.join("device.yaml"),
            "custom_key: { zh: '自定义' }",
        )
        .unwrap();

        // init_if_missing should not overwrite
        DictSet::init_if_missing(&dicts_path).unwrap();

        let dicts = DictSet::load(&dicts_path).unwrap();
        // Should have our custom entry, not defaults
        assert!(dicts.device.exists("custom_key"));
        assert!(!dicts.device.exists("desktop")); // default not added
    }
}
