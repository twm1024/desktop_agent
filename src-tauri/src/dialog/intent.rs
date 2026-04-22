// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Intent recognition and slot (parameter) extraction
//!
//! Provides pattern-based intent matching with slot filling

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Recognized intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub name: String,
    pub confidence: f64,
    pub slots: Vec<Slot>,
    pub raw_input: String,
}

/// A slot (parameter) extracted from user input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slot {
    pub name: String,
    pub value: SlotValue,
    pub required: bool,
    pub confirmed: bool,
}

/// Slot value types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum SlotValue {
    Text(String),
    Number(f64),
    Boolean(bool),
    Path(String),
    Url(String),
    Date(String),
    Time(String),
    Enum(String),
    List(Vec<String>),
    Empty,
}

impl SlotValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            SlotValue::Text(s) | SlotValue::Path(s) | SlotValue::Url(s)
            | SlotValue::Date(s) | SlotValue::Time(s) | SlotValue::Enum(s) => Some(s),
            _ => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, SlotValue::Empty)
    }
}

/// Intent definition for registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentDefinition {
    pub name: String,
    pub description: String,
    pub patterns: Vec<String>,
    pub slots: Vec<SlotDefinition>,
    pub examples: Vec<String>,
}

/// Slot definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotDefinition {
    pub name: String,
    pub slot_type: SlotType,
    pub required: bool,
    pub prompt: String,
    pub validation_pattern: Option<String>,
    pub default_value: Option<SlotValue>,
    pub enum_values: Option<Vec<String>>,
}

/// Slot type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SlotType {
    Text,
    Number,
    Boolean,
    Path,
    Url,
    Date,
    Time,
    Enum,
    List,
}

/// Intent recognizer
pub struct IntentRecognizer {
    definitions: HashMap<String, IntentDefinition>,
}

impl IntentRecognizer {
    pub fn new() -> Self {
        let mut recognizer = Self {
            definitions: HashMap::new(),
        };
        recognizer.register_builtin_intents();
        recognizer
    }

    /// Register a custom intent definition
    pub fn register_intent(&mut self, definition: IntentDefinition) {
        self.definitions.insert(definition.name.clone(), definition);
    }

    /// Unregister an intent
    pub fn unregister_intent(&mut self, name: &str) {
        self.definitions.remove(name);
    }

    /// Recognize intent from user input
    pub fn recognize(&self, input: &str) -> Result<Option<Intent>> {
        let input_lower = input.to_lowercase().trim().to_string();

        let mut best_match: Option<Intent> = None;
        let mut best_score = 0.0;

        for (name, definition) in &self.definitions {
            let score = self.match_score(&input_lower, definition);
            if score > best_score && score > 0.3 {
                best_score = score;

                let slots = self.extract_slots(input, definition);

                best_match = Some(Intent {
                    name: name.clone(),
                    confidence: score,
                    slots,
                    raw_input: input.to_string(),
                });
            }
        }

        Ok(best_match)
    }

    /// Calculate match score between input and intent patterns
    fn match_score(&self, input: &str, definition: &IntentDefinition) -> f64 {
        let mut max_score = 0.0;

        for pattern in &definition.patterns {
            let pattern_lower = pattern.to_lowercase();
            let score = self.pattern_match_score(input, &pattern_lower);
            max_score = max_score.max(score);
        }

        // Also check examples
        for example in &definition.examples {
            let example_lower = example.to_lowercase();
            let score = self.fuzzy_match(input, &example_lower);
            max_score = max_score.max(score * 0.8); // Slightly lower weight for examples
        }

        max_score
    }

    /// Calculate pattern match score
    fn pattern_match_score(&self, input: &str, pattern: &str) -> f64 {
        // Check for exact match
        if input == pattern {
            return 1.0;
        }

        // Check if input contains the pattern
        if input.contains(pattern) {
            return 0.9;
        }

        // Check if pattern contains wildcards
        if pattern.contains('{') && pattern.contains('}') {
            return self.wildcard_match(input, pattern);
        }

        // Word-level matching
        let input_words: Vec<&str> = input.split_whitespace().collect();
        let pattern_words: Vec<&str> = pattern.split_whitespace().collect();

        if pattern_words.is_empty() {
            return 0.0;
        }

        let matched_count = pattern_words.iter()
            .filter(|pw| input_words.iter().any(|iw| iw.contains(*pw) || pw.contains(*iw)))
            .count();

        (matched_count as f64) / (pattern_words.len() as f64)
    }

    /// Wildcard pattern matching (e.g., "打开 {file}")
    fn wildcard_match(&self, input: &str, pattern: &str) -> f64 {
        let parts: Vec<&str> = pattern.split(|c| c == '{' || c == '}')
            .filter(|s| !s.is_empty())
            .collect();

        if parts.is_empty() {
            return 0.0;
        }

        let mut matched = 0;
        for part in &parts {
            if input.contains(part) {
                matched += 1;
            }
        }

        (matched as f64) / (parts.len() as f64)
    }

    /// Simple fuzzy matching using Levenshtein-like approach
    fn fuzzy_match(&self, a: &str, b: &str) -> f64 {
        if a == b {
            return 1.0;
        }

        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();

        let a_len = a_chars.len();
        let b_len = b_chars.len();

        if a_len == 0 || b_len == 0 {
            return 0.0;
        }

        // Use length ratio and common prefix
        let len_ratio = if a_len > b_len {
            b_len as f64 / a_len as f64
        } else {
            a_len as f64 / b_len as f64
        };

        // Count common bigrams
        let common = self.count_common_bigrams(&a_chars, &b_chars);
        let max_bigrams = (a_len - 1).max(b_len - 1) as f64;

        let bigram_score = if max_bigrams > 0.0 {
            common as f64 / max_bigrams
        } else {
            0.0
        };

        len_ratio * 0.4 + bigram_score * 0.6
    }

    fn count_common_bigrams(&self, a: &[char], b: &[char]) -> usize {
        let mut count = 0;
        for i in 0..a.len().saturating_sub(1) {
            let bigram = (a[i], a[i + 1]);
            for j in 0..b.len().saturating_sub(1) {
                if bigram == (b[j], b[j + 1]) {
                    count += 1;
                    break;
                }
            }
        }
        count
    }

    /// Extract slots from user input based on intent definition
    fn extract_slots(&self, input: &str, definition: &IntentDefinition) -> Vec<Slot> {
        let mut slots = Vec::new();

        for slot_def in &definition.slots {
            let value = self.extract_slot_value(input, slot_def);
            slots.push(Slot {
                name: slot_def.name.clone(),
                value,
                required: slot_def.required,
                confirmed: false,
            });
        }

        slots
    }

    /// Extract a single slot value from input
    fn extract_slot_value(&self, input: &str, slot_def: &SlotDefinition) -> SlotValue {
        // Try regex pattern if available
        if let Some(pattern) = &slot_def.validation_pattern {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(caps) = re.captures(input) {
                    if let Some(m) = caps.get(1).or_else(|| caps.get(0)) {
                        return self.parse_slot_value(m.as_str(), slot_def);
                    }
                }
            }
        }

        // Try to extract based on slot type
        match slot_def.slot_type {
            SlotType::Number => {
                // Find first number in input
                for word in input.split_whitespace() {
                    if let Ok(n) = word.parse::<f64>() {
                        return SlotValue::Number(n);
                    }
                }
            }
            SlotType::Boolean => {
                let lower = input.to_lowercase();
                if lower.contains("是") || lower.contains("true") || lower.contains("yes") || lower.contains("确认") {
                    return SlotValue::Boolean(true);
                }
                if lower.contains("否") || lower.contains("false") || lower.contains("no") || lower.contains("取消") {
                    return SlotValue::Boolean(false);
                }
            }
            SlotType::Path => {
                // Find path-like strings (starting with / or drive letter or ~)
                for word in input.split_whitespace() {
                    if word.starts_with('/') || word.starts_with('~')
                        || word.starts_with("C:\\") || word.starts_with("D:\\")
                        || word.starts_with("./") || word.starts_with("../") {
                        return SlotValue::Path(word.to_string());
                    }
                }
            }
            SlotType::Url => {
                for word in input.split_whitespace() {
                    if word.starts_with("http://") || word.starts_with("https://") {
                        return SlotValue::Url(word.to_string());
                    }
                }
            }
            SlotType::Enum => {
                if let Some(values) = &slot_def.enum_values {
                    let lower = input.to_lowercase();
                    for value in values {
                        if lower.contains(&value.to_lowercase()) {
                            return SlotValue::Enum(value.clone());
                        }
                    }
                }
            }
            SlotType::List => {
                // Try to extract comma-separated or space-separated list
                if let Some(idx) = input.find("：").or_else(|| input.find(": ")) {
                    let rest = &input[idx + "：".len()..];
                    let items: Vec<String> = rest
                        .split(&[',', '，', '、'][..])
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    if !items.is_empty() {
                        return SlotValue::List(items);
                    }
                }
            }
            _ => {}
        }

        // Return default if available
        slot_def.default_value.clone().unwrap_or(SlotValue::Empty)
    }

    /// Parse a string into the appropriate slot value type
    fn parse_slot_value(&self, value: &str, slot_def: &SlotDefinition) -> SlotValue {
        match slot_def.slot_type {
            SlotType::Number => value.parse::<f64>()
                .map(SlotValue::Number)
                .unwrap_or(SlotValue::Text(value.to_string())),
            SlotType::Boolean => {
                let lower = value.to_lowercase();
                SlotValue::Boolean(lower == "true" || lower == "yes" || lower == "是")
            }
            SlotType::Path => SlotValue::Path(value.to_string()),
            SlotType::Url => SlotValue::Url(value.to_string()),
            SlotType::Date => SlotValue::Date(value.to_string()),
            SlotType::Time => SlotValue::Time(value.to_string()),
            SlotType::Enum => SlotValue::Enum(value.to_string()),
            SlotType::List => SlotValue::List(
                value.split(&[',', '，', '、'][..])
                    .map(|s| s.trim().to_string())
                    .collect()
            ),
            SlotType::Text => SlotValue::Text(value.to_string()),
        }
    }

    /// Get missing required slots
    pub fn get_missing_slots(&self, intent: &Intent) -> Vec<&Slot> {
        intent.slots.iter()
            .filter(|s| s.required && s.value.is_empty())
            .collect()
    }

    /// Fill a slot in an intent
    pub fn fill_slot(&self, intent: &mut Intent, slot_name: &str, value: SlotValue) {
        if let Some(slot) = intent.slots.iter_mut().find(|s| s.name == slot_name) {
            slot.value = value;
            slot.confirmed = true;
        }
    }

    /// Register built-in intents
    fn register_builtin_intents(&mut self) {
        // File operations
        self.register_intent(IntentDefinition {
            name: "file.list".to_string(),
            description: "列出目录文件".to_string(),
            patterns: vec![
                "列出 {path} 的文件".to_string(),
                "查看目录 {path}".to_string(),
                "ls {path}".to_string(),
                "显示文件列表".to_string(),
                "打开文件夹".to_string(),
            ],
            slots: vec![
                SlotDefinition {
                    name: "path".to_string(),
                    slot_type: SlotType::Path,
                    required: false,
                    prompt: "请输入要列出的目录路径".to_string(),
                    validation_pattern: None,
                    default_value: Some(SlotValue::Path(".".to_string())),
                    enum_values: None,
                },
                SlotDefinition {
                    name: "show_hidden".to_string(),
                    slot_type: SlotType::Boolean,
                    required: false,
                    prompt: "是否显示隐藏文件？".to_string(),
                    validation_pattern: None,
                    default_value: Some(SlotValue::Boolean(false)),
                    enum_values: None,
                },
            ],
            examples: vec![
                "列出当前目录".to_string(),
                "查看 /home 的文件".to_string(),
                "显示桌面上的文件".to_string(),
            ],
        });

        self.register_intent(IntentDefinition {
            name: "file.search".to_string(),
            description: "搜索文件".to_string(),
            patterns: vec![
                "搜索 {pattern}".to_string(),
                "查找文件 {pattern}".to_string(),
                "find {pattern}".to_string(),
                "搜索 {pattern} 在 {path}".to_string(),
            ],
            slots: vec![
                SlotDefinition {
                    name: "pattern".to_string(),
                    slot_type: SlotType::Text,
                    required: true,
                    prompt: "请输入搜索关键词".to_string(),
                    validation_pattern: None,
                    default_value: None,
                    enum_values: None,
                },
                SlotDefinition {
                    name: "path".to_string(),
                    slot_type: SlotType::Path,
                    required: false,
                    prompt: "请输入搜索路径".to_string(),
                    validation_pattern: None,
                    default_value: Some(SlotValue::Path(".".to_string())),
                    enum_values: None,
                },
            ],
            examples: vec![
                "搜索 report.pdf".to_string(),
                "在 /home 中查找 config".to_string(),
                "查找所有 .txt 文件".to_string(),
            ],
        });

        self.register_intent(IntentDefinition {
            name: "file.copy".to_string(),
            description: "复制文件".to_string(),
            patterns: vec![
                "复制 {src} 到 {dst}".to_string(),
                "copy {src} {dst}".to_string(),
                "拷贝文件".to_string(),
            ],
            slots: vec![
                SlotDefinition {
                    name: "src".to_string(),
                    slot_type: SlotType::Path,
                    required: true,
                    prompt: "请输入源文件路径".to_string(),
                    validation_pattern: None,
                    default_value: None,
                    enum_values: None,
                },
                SlotDefinition {
                    name: "dst".to_string(),
                    slot_type: SlotType::Path,
                    required: true,
                    prompt: "请输入目标路径".to_string(),
                    validation_pattern: None,
                    default_value: None,
                    enum_values: None,
                },
            ],
            examples: vec![
                "复制 a.txt 到 b.txt".to_string(),
                "拷贝 /home/file.txt 到 /tmp/".to_string(),
            ],
        });

        // System operations
        self.register_intent(IntentDefinition {
            name: "system.info".to_string(),
            description: "获取系统信息".to_string(),
            patterns: vec![
                "系统信息".to_string(),
                "查看系统".to_string(),
                "sysinfo".to_string(),
                "电脑配置".to_string(),
                "系统状态".to_string(),
            ],
            slots: vec![],
            examples: vec![
                "查看系统信息".to_string(),
                "电脑配置是什么".to_string(),
            ],
        });

        self.register_intent(IntentDefinition {
            name: "system.launch".to_string(),
            description: "启动应用".to_string(),
            patterns: vec![
                "打开 {app}".to_string(),
                "启动 {app}".to_string(),
                "运行 {app}".to_string(),
                "launch {app}".to_string(),
            ],
            slots: vec![
                SlotDefinition {
                    name: "app".to_string(),
                    slot_type: SlotType::Text,
                    required: true,
                    prompt: "请输入要启动的应用名称".to_string(),
                    validation_pattern: None,
                    default_value: None,
                    enum_values: None,
                },
            ],
            examples: vec![
                "打开浏览器".to_string(),
                "启动记事本".to_string(),
                "运行 VSCode".to_string(),
            ],
        });

        // Skill operations
        self.register_intent(IntentDefinition {
            name: "skill.list".to_string(),
            description: "列出可用技能".to_string(),
            patterns: vec![
                "列出技能".to_string(),
                "查看技能列表".to_string(),
                "有什么技能".to_string(),
                "显示所有技能".to_string(),
            ],
            slots: vec![],
            examples: vec![
                "显示所有技能".to_string(),
                "有什么技能可以用".to_string(),
            ],
        });

        self.register_intent(IntentDefinition {
            name: "skill.execute".to_string(),
            description: "执行技能".to_string(),
            patterns: vec![
                "执行 {skill}".to_string(),
                "运行技能 {skill}".to_string(),
                "使用 {skill}".to_string(),
                "run {skill}".to_string(),
            ],
            slots: vec![
                SlotDefinition {
                    name: "skill".to_string(),
                    slot_type: SlotType::Text,
                    required: true,
                    prompt: "请输入要执行的技能名称".to_string(),
                    validation_pattern: None,
                    default_value: None,
                    enum_values: None,
                },
            ],
            examples: vec![
                "执行数据备份".to_string(),
                "运行 OCR 技能".to_string(),
                "使用格式转换".to_string(),
            ],
        });

        // Help intent
        self.register_intent(IntentDefinition {
            name: "help".to_string(),
            description: "获取帮助".to_string(),
            patterns: vec![
                "帮助".to_string(),
                "help".to_string(),
                "使用说明".to_string(),
                "怎么用".to_string(),
                "能做什么".to_string(),
            ],
            slots: vec![],
            examples: vec![
                "帮助".to_string(),
                "怎么使用".to_string(),
                "你能做什么".to_string(),
            ],
        });

        // Greeting
        self.register_intent(IntentDefinition {
            name: "greeting".to_string(),
            description: "问候".to_string(),
            patterns: vec![
                "你好".to_string(),
                "hello".to_string(),
                "hi".to_string(),
                "嗨".to_string(),
                "早上好".to_string(),
                "下午好".to_string(),
                "晚上好".to_string(),
            ],
            slots: vec![],
            examples: vec![
                "你好啊".to_string(),
                "hello world".to_string(),
            ],
        });
    }

    /// List all registered intents
    pub fn list_intents(&self) -> Vec<&IntentDefinition> {
        self.definitions.values().collect()
    }

    /// Get intent definition by name
    pub fn get_intent(&self, name: &str) -> Option<&IntentDefinition> {
        self.definitions.get(name)
    }
}

impl Default for IntentRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recognize_greeting() {
        let recognizer = IntentRecognizer::new();
        let result = recognizer.recognize("你好").unwrap();
        assert!(result.is_some());
        let intent = result.unwrap();
        assert_eq!(intent.name, "greeting");
    }

    #[test]
    fn test_recognize_file_list() {
        let recognizer = IntentRecognizer::new();
        let result = recognizer.recognize("列出当前目录文件").unwrap();
        assert!(result.is_some());
        let intent = result.unwrap();
        assert_eq!(intent.name, "file.list");
    }

    #[test]
    fn test_recognize_system_info() {
        let recognizer = IntentRecognizer::new();
        let result = recognizer.recognize("查看系统信息").unwrap();
        assert!(result.is_some());
        let intent = result.unwrap();
        assert_eq!(intent.name, "system.info");
    }

    #[test]
    fn test_slot_extraction_path() {
        let recognizer = IntentRecognizer::new();
        let result = recognizer.recognize("查看 /home/user 目录").unwrap();
        assert!(result.is_some());
        let intent = result.unwrap();
        let path_slot = intent.slots.iter().find(|s| s.name == "path");
        assert!(path_slot.is_some());
    }

    #[test]
    fn test_fuzzy_match() {
        let recognizer = IntentRecognizer::new();
        let score = recognizer.fuzzy_match("列出文件", "列出所有文件");
        assert!(score > 0.5);
    }
}
