//! Shared response parsing structures

use crate::adapters::NewDictEntries;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

/// Deserialize a number that might be a string or a number
/// Used by MiniMax adapter because some models return confidence as string
pub fn deserialize_number_or_string<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(opt.and_then(|v| match v {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    }))
}

/// Input for Step 2 - only the fields that need dictionary alignment.
/// Step 2 doesn't need summary, extended, entities, or confidence from Step 1.
#[derive(Debug, Serialize)]
pub struct Step2Input {
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub subtype: Option<String>,
    pub tags: Vec<String>,
    pub topics: Vec<String>,
}

/// Step 1 output - freely analyzed without dictionary constraints
/// Used by Ollama and MiniMax adapters
#[derive(Debug, Deserialize, Serialize)]
pub struct Step1Output {
    pub summary: Option<String>,
    pub extended: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub subtype: Option<String>,
    pub tags: Vec<String>,
    pub topics: Vec<String>,
    pub entities: Vec<String>,
    #[serde(deserialize_with = "deserialize_number_or_string", default)]
    pub confidence: Option<f64>,
}

/// Step 2 final output - aligned with dictionary
/// Used by Ollama and MiniMax adapters
#[derive(Debug, Deserialize)]
pub struct Step2Final {
    pub summary: Option<String>,
    pub extended: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub subtype: Option<String>,
    pub tags: Vec<String>,
    pub topics: Vec<String>,
    pub entities: Vec<String>,
    #[serde(deserialize_with = "deserialize_number_or_string", default)]
    pub confidence: Option<f64>,
}

/// Step 2 output structure - includes final aligned output and new dictionary entries
/// Used by Ollama and MiniMax adapters
#[derive(Debug, Deserialize)]
pub struct Step2Output {
    #[serde(rename = "final")]
    pub final_: Step2Final,
    pub new_entries: NewDictEntries,
}

/// Extract JSON from content that may be wrapped in markdown code blocks
/// Used by MiniMax adapter (and potentially others if they return markdown-wrapped JSON)
pub fn extract_json_from_content(content: &str) -> &str {
    // Try to find JSON inside markdown code blocks first
    if let Some(start) = content.find("```json") {
        let after_start = &content[start + 7..];
        let search_content = after_start.trim();
        if let Some(json_start) = search_content.find('{') {
            let potential = &search_content[json_start..];
            if let Some(end) = find_matching_brace(potential) {
                return &potential[..=end];
            }
        }
    }
    // Try generic code blocks (without json specifier)
    if let Some(start) = content.find("```") {
        let after_code = &content[start + 3..];
        let search_content = after_code.trim();
        if let Some(code_start) = search_content.find('{') {
            let potential = &search_content[code_start..];
            if let Some(end) = find_matching_brace(potential) {
                return &potential[..=end];
            }
        }
    }
    // Try to find raw JSON object
    if let Some(start) = content.find('{') {
        let potential = &content[start..];
        if let Some(end) = find_matching_brace(potential) {
            return &potential[..=end];
        }
    }
    // Fallback: return original content
    content
}

/// Find the matching closing brace for an opening brace
/// Returns the position of the closing brace that completes the JSON object
fn find_matching_brace(s: &str) -> Option<usize> {
    // First, validate that we start with {
    let first_char = s.chars().next()?;
    if first_char != '{' {
        return None;
    }

    // Find the position of the last } in the string
    let last_brace = s.rfind('}')?;

    // Return the position of the last }
    // This works because the JSON structure should end with the outermost }
    Some(last_brace)
}

/// Parse JSON from content that may be wrapped in markdown code blocks
/// label: "Step1" or "Step2" for error messages
pub fn parse_json<T: for<'de> Deserialize<'de>>(content: &str, label: &str) -> Result<T> {
    let extracted = extract_json_from_content(content);
    serde_json::from_str(extracted)
        .map_err(|e| Error::Config(format!("{} parse error: {}", label, e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_from_plain_json() {
        let json = r#"{"key": "value"}"#;
        let extracted = extract_json_from_content(json);
        assert!(extracted.contains("key"));
        assert!(extracted.contains("value"));
    }

    #[test]
    fn test_extract_json_from_markdown_json_block() {
        let content = "Some text\n```json\n{\"key\": \"value\"}\n```\nMore text";
        let extracted = extract_json_from_content(content);
        assert!(extracted.contains("key"));
        assert!(extracted.contains("value"));
    }

    #[test]
    fn test_extract_json_from_markdown_code_block() {
        let content = "```\n{\"key\": \"value\"}\n```";
        let extracted = extract_json_from_content(content);
        assert!(extracted.contains("key"));
    }

    #[test]
    fn test_extract_json_from_raw_json_object() {
        let content = "prefix{\"a\":1}suffix";
        let extracted = extract_json_from_content(content);
        assert!(extracted.contains("a"));
    }

    #[test]
    fn test_parse_json_valid_step1_output() {
        let json = r#"{"summary": "test", "tags": ["a", "b"], "topics": [], "entities": [], "type": "note"}"#;
        let parsed: Step1Output = parse_json(json, "test").unwrap();
        assert_eq!(parsed.summary, Some("test".to_string()));
        assert_eq!(parsed.tags, vec!["a", "b"]);
    }

    #[test]
    fn test_parse_json_invalid_returns_error() {
        let json = "not json";
        let result: std::result::Result<Step1Output, _> = parse_json(json, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_confidence_can_be_string_or_number() {
        #[derive(Debug, Deserialize)]
        struct TestStruct {
            #[serde(deserialize_with = "deserialize_number_or_string", default)]
            confidence: Option<f64>,
        }

        // Test with number
        let json = r#"{"confidence": 0.95}"#;
        let parsed: TestStruct = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.confidence, Some(0.95));

        // Test with string
        let json = r#"{"confidence": "0.95"}"#;
        let parsed: TestStruct = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.confidence, Some(0.95));

        // Test with null
        let json = r#"{"confidence": null}"#;
        let parsed: TestStruct = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.confidence, None);
    }

    #[test]
    fn test_step1_output_serde_roundtrip() {
        let step1 = Step1Output {
            summary: Some("Test summary".to_string()),
            extended: Some("Extended content".to_string()),
            type_: Some("note".to_string()),
            subtype: Some("test".to_string()),
            tags: vec!["tag1".to_string()],
            topics: vec!["topic1".to_string()],
            entities: vec!["entity1".to_string()],
            confidence: Some(0.85),
        };

        let json = serde_json::to_string(&step1).unwrap();
        let parsed: Step1Output = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.summary, step1.summary);
        assert_eq!(parsed.type_, step1.type_);
        assert_eq!(parsed.tags, step1.tags);
        assert_eq!(parsed.confidence, step1.confidence);
    }
}
