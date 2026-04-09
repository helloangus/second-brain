//! Shared prompt templates for analysis pipeline

use crate::adapters::{Step1Output, Step2Input};

/// Step 1 prompt template - free analysis without dictionary constraints
/// Used by all three adapters (Ollama, OpenAI, MiniMax)
/// Placeholders: {data_type}, {content}
pub const STEP1_PROMPT_TEMPLATE: &str = r#"分析这个{data_type}并提供:
1. 简短摘要（2-3句话）
2. 扩展内容 - 当内容复杂、有多个要点或细节无法用2-3句话概括时使用此字段。此字段没有长度限制。
3. 事件类型（根据内容含义自由选择）
4. 事件子类型（根据内容含义自由选择）
5. 关键标签（需要时创建新的 - 要有创意且具体）
6. 关键主题（需要时创建新的 - 要有创意且具体）
7. 提到的任何实体

重要：完全根据内容选择最描述性的类型、子类型、标签和主题。不要试图匹配现有值。如果没有完全合适的就创建新值。

内容:
{content}

请以JSON格式回复:
{{
    "summary": "2-3句话的简短摘要",
    "extended": "详细内容，如果没有合适内容则填null",
    "type": "自由选择的事件类型",
    "subtype": "自由选择的子类型",
    "tags": ["标签1", "标签2"],
    "topics": ["主题1", "主题2"],
    "entities": ["实体1"],
    "confidence": 0.0-1.0
}}"#;

/// Step 2 prompt template - dictionary-aligned analysis
/// Placeholders: {analysis}, {dict_context}
pub const STEP2_PROMPT_TEMPLATE: &str = r#"回顾你的初步分析，并与现有字典进行对齐（如果适用）。

初步分析:
{analysis}

现有字典:
{dict_context}

任务:
对于每个字段（类型、子类型、标签、主题）:
- 如果初步值匹配现有字典条目 → 使用现有条目（使用精确的key）
- 如果初步值是新的（不在字典中）→ 保留它作为新值，它将被添加到字典中

重要：当现有字典值合适时优先使用。但不要强制匹配，如果初步值确实不同或更准确的话。

请以JSON格式回复:
{{
    "final": {{
        "summary": "简短摘要",
        "extended": "详细内容或null",
        "type": "现有或新的事件类型",
        "subtype": "现有或新的子类型",
        "tags": ["标签1", "标签2"],
        "topics": ["主题1", "主题2"],
        "entities": ["实体1"],
        "confidence": 0.0-1.0
    }},
    "new_entries": {{
        "event_types": [{{"key": "新类型", "zh": null, "description": null}}],
        "event_subtypes": [],
        "tags": [{{"key": "新标签", "zh": null, "description": null}}],
        "topics": [{{"key": "新主题", "zh": null, "description": null}}]
    }}
}}"#;

/// Reasoning Step 1 prompt template - deep reasoning analysis
/// Used by MiniMax adapter only (for TaskType::Reasoning)
/// Placeholders: {data_type}, {content}
pub const REASONING_STEP1_PROMPT_TEMPLATE: &str = r#"对以下内容进行深度推理分析:

内容类型: {data_type}
内容: {content}

请提供:
1. 简短摘要（2-3句话）
2. 扩展内容 - 详细分析
3. 事件类型
4. 事件子类型
5. 关键标签
6. 关键主题
7. 提到的任何实体

请以JSON格式回复:
{{
    "summary": "简短摘要",
    "extended": "详细内容",
    "type": "事件类型",
    "subtype": "子类型",
    "tags": ["标签"],
    "topics": ["主题"],
    "entities": ["实体"],
    "confidence": 0.0-1.0
}}"#;

/// Build Step 1 prompt for summarize task
pub fn build_step1_prompt(data_type: &str, content: &str) -> String {
    STEP1_PROMPT_TEMPLATE
        .replace("{data_type}", data_type)
        .replace("{content}", content)
}

/// Build Step 2 prompt for summarize task.
/// Extracts only type_, subtype, tags, topics from step1 for the analysis section.
pub fn build_step2_prompt(step1: &Step1Output, dict_context: &str) -> String {
    let step2_input = Step2Input {
        type_: step1.type_.clone(),
        subtype: step1.subtype.clone(),
        tags: step1.tags.clone(),
        topics: step1.topics.clone(),
    };
    let analysis_json = serde_json::to_string(&step2_input).unwrap_or_default();
    STEP2_PROMPT_TEMPLATE
        .replace("{analysis}", &analysis_json)
        .replace("{dict_context}", dict_context)
}

/// Build Step 1 prompt for reasoning task
pub fn build_reasoning_step1_prompt(data_type: &str, content: &str) -> String {
    REASONING_STEP1_PROMPT_TEMPLATE
        .replace("{data_type}", data_type)
        .replace("{content}", content)
}

/// Build Step 2 prompt for reasoning task.
/// Uses the same template as STEP2 but with "推理" in the header for clarity.
/// Extracts only type_, subtype, tags, topics from step1 for the analysis section.
pub fn build_reasoning_step2_prompt(step1: &Step1Output, dict_context: &str) -> String {
    let step2_input = Step2Input {
        type_: step1.type_.clone(),
        subtype: step1.subtype.clone(),
        tags: step1.tags.clone(),
        topics: step1.topics.clone(),
    };
    let analysis_json = serde_json::to_string(&step2_input).unwrap_or_default();
    STEP2_PROMPT_TEMPLATE
        .replace("{analysis}", &analysis_json)
        .replace("{dict_context}", dict_context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::Step1Output;

    #[test]
    fn test_build_step1_prompt_contains_data_type_and_content() {
        let prompt = build_step1_prompt("text", "Hello world");
        assert!(prompt.contains("text"));
        assert!(prompt.contains("Hello world"));
        assert!(prompt.contains("summary"));
        assert!(prompt.contains("tags"));
    }

    #[test]
    fn test_build_step2_prompt_contains_step1_output() {
        let step1 = Step1Output {
            summary: Some("Test summary".to_string()),
            extended: None,
            type_: Some("note".to_string()),
            subtype: Some("summary".to_string()),
            tags: vec!["test".to_string()],
            topics: vec!["tech".to_string()],
            entities: vec![],
            confidence: Some(0.9),
        };
        let dict_context = "事件类型:\n  - note";
        let prompt = build_step2_prompt(&step1, dict_context);

        assert!(prompt.contains("note"));
        assert!(prompt.contains("test"));
        assert!(prompt.contains("tech"));
        assert!(prompt.contains(dict_context));
    }

    #[test]
    fn test_build_reasoning_step1_prompt_differs_from_regular() {
        let regular = build_step1_prompt("text", "content");
        let reasoning = build_reasoning_step1_prompt("text", "content");
        assert_ne!(regular, reasoning);
        assert!(reasoning.contains("推理"));
    }

    #[test]
    fn test_build_reasoning_step2_prompt_uses_same_template() {
        let step1 = Step1Output {
            summary: Some("Test".to_string()),
            extended: None,
            type_: Some("note".to_string()),
            subtype: None,
            tags: vec![],
            topics: vec![],
            entities: vec![],
            confidence: Some(0.8),
        };
        let ctx = "事件类型:\n  - note";
        let prompt = build_reasoning_step2_prompt(&step1, ctx);
        assert!(prompt.contains("note"));
    }
}
