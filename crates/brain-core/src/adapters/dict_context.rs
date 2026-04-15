//! Dictionary context string builder

use crate::dicts::{DictEntry, DictSet};

/// Format a single dictionary entry for the context string.
/// Formats as: "  - key" or "  - key (zh)" or "  - key: description" or "  - key (zh): description"
pub(crate) fn format_entry(entry: &DictEntry) -> String {
    let zh = entry.zh.as_deref().unwrap_or("");
    let desc = entry.description.as_deref().unwrap_or("");
    if zh.is_empty() && desc.is_empty() {
        format!("  - {}", entry.key)
    } else if zh.is_empty() {
        format!("  - {}: {}", entry.key, desc)
    } else if desc.is_empty() {
        format!("  - {} ({})", entry.key, zh)
    } else {
        format!("  - {} ({}): {}", entry.key, zh, desc)
    }
}

/// Format entries by applying format_entry to each, collecting into a Vec<String>.
fn format_entries(entries: &[&DictEntry]) -> Vec<String> {
    entries.iter().copied().map(format_entry).collect()
}

/// Build dictionary context string for Step 2 prompt alignment.
/// Iterates over all dict types and formats them for inclusion in the prompt.
pub(crate) fn build_dict_context(dict_set: &DictSet) -> String {
    let mut parts = Vec::new();

    // Event Types
    let entries = format_entries(&dict_set.event_type.list());
    if !entries.is_empty() {
        parts.push(format!("事件类型:\n{}", entries.join("\n")));
    }

    // Event Subtypes
    let entries = format_entries(&dict_set.event_subtype.list());
    if !entries.is_empty() {
        parts.push(format!("事件子类型:\n{}", entries.join("\n")));
    }

    // Tags
    let entries = format_entries(&dict_set.tags.list());
    if !entries.is_empty() {
        parts.push(format!("标签:\n{}", entries.join("\n")));
    }

    // Topics
    let entries = format_entries(&dict_set.topics.list());
    if !entries.is_empty() {
        parts.push(format!("主题:\n{}", entries.join("\n")));
    }

    if parts.is_empty() {
        "暂无现有字典条目。".to_string()
    } else {
        parts.join("\n\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dicts::{Dict, DictEntry, DictSet};

    #[test]
    fn test_build_dict_context_with_default_dicts() {
        let dict_set = DictSet::default_dicts();
        let context = build_dict_context(&dict_set);
        assert!(!context.is_empty());
        assert!(context.contains("事件类型:") || context.contains("标签:"));
    }

    #[test]
    fn test_build_dict_context_empty_when_no_entries() {
        let dict_set = DictSet {
            device: Dict::default(),
            channel: Dict::default(),
            capture_agent: Dict::default(),
            event_type: Dict::default(),
            event_subtype: Dict::default(),
            tags: Dict::default(),
            topics: Dict::default(),
        };
        let context = build_dict_context(&dict_set);
        assert_eq!(context, "暂无现有字典条目。");
    }

    #[test]
    fn test_build_dict_context_with_zh_and_description() {
        let mut dict = Dict::default();
        dict.add(
            DictEntry::new("test_type")
                .with_zh("测试类型")
                .with_description("A test type"),
        );

        let dict_set = DictSet {
            device: Dict::default(),
            channel: Dict::default(),
            capture_agent: Dict::default(),
            event_type: dict,
            event_subtype: Dict::default(),
            tags: Dict::default(),
            topics: Dict::default(),
        };

        let context = build_dict_context(&dict_set);
        assert!(context.contains("test_type"));
        assert!(context.contains("测试类型"));
        assert!(context.contains("A test type"));
    }

    #[test]
    fn test_format_entry_key_only() {
        let entry = DictEntry::new("desktop");
        let formatted = format_entry(&entry);
        assert_eq!(formatted, "  - desktop");
    }

    #[test]
    fn test_format_entry_with_zh_only() {
        let entry = DictEntry::new("desktop").with_zh("台式机");
        let formatted = format_entry(&entry);
        assert!(formatted.contains("desktop"));
        assert!(formatted.contains("台式机"));
    }

    #[test]
    fn test_format_entry_with_description_only() {
        let entry = DictEntry::new("desktop").with_description("桌面设备");
        let formatted = format_entry(&entry);
        assert!(formatted.contains("desktop"));
        assert!(formatted.contains("桌面设备"));
    }

    #[test]
    fn test_format_entry_with_both_zh_and_description() {
        let entry = DictEntry::new("desktop")
            .with_zh("台式机")
            .with_description("桌面设备");
        let formatted = format_entry(&entry);
        assert!(formatted.contains("desktop"));
        assert!(formatted.contains("台式机"));
        assert!(formatted.contains("桌面设备"));
    }
}
