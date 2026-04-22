// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Data processing service
//!
//! Provides utilities for data manipulation, transformation, and analysis

use crate::error::{AppError, Result};
use crate::services::data_service::processors::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod processors;

/// Data transformation type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "params")]
pub enum DataTransform {
    JsonPath { path: String },
    JsonQuery { query: String },
    CsvParse { delimiter: Option<char> },
    CsvStringify { delimiter: Option<char> },
    XmlToJson {},
    JsonToXml {},
    Base64Encode {},
    Base64Decode {},
    UrlEncode {},
    UrlDecode {},
    Hash { algorithm: HashAlgorithm },
    Encrypt { algorithm: String, key: String },
    Decrypt { algorithm: String, key: String },
    Template { template: String },
    Filter { condition: String },
    Sort { key: Option<String>, order: SortOrder },
    Group { key: String },
    Aggregate { operation: AggregateOp, key: Option<String> },
    Merge { strategy: MergeStrategy },
    Split { separator: String },
    Join { separator: String },
    Map { script: String },
    Reduce { operation: String, initial: serde_json::Value },
    Format { pattern: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HashAlgorithm {
    Md5,
    Sha1,
    Sha256,
    Sha512,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AggregateOp {
    Sum,
    Avg,
    Min,
    Max,
    Count,
    First,
    Last,
    Join,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MergeStrategy {
    Replace,
    Merge,
    Append,
    Prepend,
}

/// Data processing result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataResult {
    pub success: bool,
    pub data: serde_json::Value,
    pub metadata: DataMetadata,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataMetadata {
    pub record_count: Option<usize>,
    pub size_bytes: Option<usize>,
    pub format: Option<String>,
    pub processing_time_ms: u64,
}

/// Data service
pub struct DataService;

impl DataService {
    pub fn new() -> Self {
        Self
    }

    /// Transform data according to specified transformation
    pub async fn transform(&self, data: serde_json::Value, transform: DataTransform) -> Result<DataResult> {
        let start = std::time::Instant::now();

        let result = match transform {
            DataTransform::JsonPath { path } => {
                JsonPathProcessor::process(&data, &path)
            }
            DataTransform::CsvParse { delimiter } => {
                CsvProcessor::parse(&data, delimiter.unwrap_or(','))
            }
            DataTransform::CsvStringify { delimiter } => {
                CsvProcessor::stringify(&data, delimiter.unwrap_or(','))
            }
            DataTransform::Base64Encode {} => {
                EncodingProcessor::base64_encode(&data)
            }
            DataTransform::Base64Decode {} => {
                EncodingProcessor::base64_decode(&data)
            }
            DataTransform::UrlEncode {} => {
                EncodingProcessor::url_encode(&data)
            }
            DataTransform::UrlDecode {} => {
                EncodingProcessor::url_decode(&data)
            }
            DataTransform::Hash { algorithm } => {
                HashProcessor::hash(&data, algorithm)
            }
            DataTransform::Filter { condition } => {
                FilterProcessor::process(&data, &condition)
            }
            DataTransform::Sort { key, order } => {
                SortProcessor::process(&data, key.as_deref(), order)
            }
            DataTransform::Group { key } => {
                GroupProcessor::process(&data, &key)
            }
            DataTransform::Aggregate { operation, key } => {
                AggregateProcessor::process(&data, operation, key.as_deref())
            }
            DataTransform::Merge { strategy } => {
                MergeProcessor::process(&data, strategy)
            }
            DataTransform::Split { separator } => {
                StringProcessor::split(&data, &separator)
            }
            DataTransform::Join { separator } => {
                StringProcessor::join(&data, &separator)
            }
            DataTransform::Format { pattern } => {
                FormatProcessor::process(&data, &pattern)
            }
            _ => Err(AppError::Serialization("Transform not implemented".to_string())),
        };

        let duration = start.elapsed();

        match result {
            Ok(data) => {
                let metadata = DataMetadata {
                    record_count: calculate_record_count(&data),
                    size_bytes: Some(serde_json::to_vec(&data)?.len()),
                    format: detect_format(&data),
                    processing_time_ms: duration.as_millis() as u64,
                };

                Ok(DataResult {
                    success: true,
                    data,
                    metadata,
                    error: None,
                })
            }
            Err(e) => Ok(DataResult {
                success: false,
                data: serde_json::Value::Null,
                metadata: DataMetadata {
                    record_count: None,
                    size_bytes: None,
                    format: None,
                    processing_time_ms: duration.as_millis() as u64,
                },
                error: Some(e.to_string()),
            }),
        }
    }

    /// Batch transform multiple data items
    pub async fn batch_transform(
        &self,
        items: Vec<serde_json::Value>,
        transforms: Vec<DataTransform>,
    ) -> Result<Vec<DataResult>> {
        let mut results = Vec::with_capacity(items.len());

        for item in items {
            let mut data = item;
            for transform in &transforms {
                let result = self.transform(data, transform.clone()).await?;
                if !result.success {
                    results.push(result);
                    break;
                }
                data = result.data;
            }
        }

        Ok(results)
    }

    /// Validate data against schema
    pub async fn validate(&self, data: &serde_json::Value, schema: &serde_json::Value) -> Result<bool> {
        // Basic JSON schema validation
        match (data, schema) {
            (serde_json::Value::Object(obj), serde_json::Value::Object(schema_obj)) => {
                for (key, schema_value) in schema_obj {
                    if !obj.contains_key(key) {
                        return Ok(false);
                    }
                    if let Some(value) = obj.get(key) {
                        if !self.validate(value, schema_value).await? {
                            return Ok(false);
                        }
                    }
                }
                Ok(true)
            }
            (serde_json::Value::Array(arr), serde_json::Value::Array(schema_arr)) => {
                if !schema_arr.is_empty() {
                    let schema_item = &schema_arr[0];
                    for item in arr {
                        if !self.validate(item, schema_item).await? {
                            return Ok(false);
                        }
                    }
                }
                Ok(true)
            }
            (serde_json::Value::String(_), serde_json::Value::String(_)) => Ok(true),
            (serde_json::Value::Number(_), serde_json::Value::Number(_)) => Ok(true),
            (serde_json::Value::Bool(_), serde_json::Value::Bool(_)) => Ok(true),
            (serde_json::Value::Null, serde_json::Value::Null) => Ok(true),
            _ => Ok(false),
        }
    }

    /// Convert data between formats
    pub async fn convert(&self, data: serde_json::Value, from: &str, to: &str) -> Result<serde_json::Value> {
        match (from, to) {
            ("json", "string") => Ok(serde_json::json!(data.to_string())),
            ("string", "json") => {
                let s = data.as_str()
                    .ok_or_else(|| AppError::Serialization("Expected string".to_string()))?;
                serde_json::from_str(s)
                    .map_err(|e| AppError::Serialization(format!("Invalid JSON: {}", e)))
            }
            _ => Err(AppError::Serialization(format!(
                "Conversion from {} to {} not supported",
                from, to
            ))),
        }
    }
}

impl Default for DataService {
    fn default() -> Self {
        Self::new()
    }
}

fn calculate_record_count(data: &serde_json::Value) -> Option<usize> {
    match data {
        serde_json::Value::Array(arr) => Some(arr.len()),
        serde_json::Value::Object(obj) => Some(obj.len()),
        _ => None,
    }
}

fn detect_format(data: &serde_json::Value) -> Option<String> {
    match data {
        serde_json::Value::Null => Some("null".to_string()),
        serde_json::Value::Bool(_) => Some("boolean".to_string()),
        serde_json::Value::Number(_) => Some("number".to_string()),
        serde_json::Value::String(_) => Some("string".to_string()),
        serde_json::Value::Array(_) => Some("array".to_string()),
        serde_json::Value::Object(_) => Some("object".to_string()),
    }
}

pub mod processors {
    use super::*;
    use crate::error::Result;

    pub struct JsonPathProcessor;
    impl JsonPathProcessor {
        pub fn process(data: &serde_json::Value, path: &str) -> Result<serde_json::Value> {
            // Simple JSONPath implementation (supports basic paths like "user.name" or "items[0]")
            let parts: Vec<&str> = path.split('.').collect();
            let mut current = data;

            for part in parts {
                if let Some(index) = part.trim_start_matches('[').trim_end_matches(']').parse::<usize>().ok() {
                    current = current.get(index)
                        .ok_or_else(|| AppError::Serialization(format!("Index {} out of bounds", index)))?;
                } else {
                    current = current.get(part)
                        .ok_or_else(|| AppError::Serialization(format!("Key '{}' not found", part)))?;
                }
            }

            Ok(current.clone())
        }
    }

    pub struct CsvProcessor;
    impl CsvProcessor {
        pub fn parse(data: &serde_json::Value, delimiter: char) -> Result<serde_json::Value> {
            let input = data.as_str()
                .ok_or_else(|| AppError::Serialization("Expected string input".to_string()))?;

            let mut rdr = csv::ReaderBuilder::new()
                .delimiter(delimiter as u8)
                .from_reader(input.as_bytes());

            let mut result = Vec::new();
            for record in rdr.deserialize() {
                let record: HashMap<String, String> = record
                    .map_err(|e| AppError::Serialization(format!("CSV parse error: {}", e)))?;
                let json_value = serde_json::to_value(record)
                    .map_err(|e| AppError::Serialization(format!("JSON conversion error: {}", e)))?;
                result.push(json_value);
            }

            Ok(serde_json::json!(result))
        }

        pub fn stringify(data: &serde_json::Value, delimiter: char) -> Result<serde_json::Value> {
            let arr = data.as_array()
                .ok_or_else(|| AppError::Serialization("Expected array input".to_string()))?;

            let mut wtr = csv::WriterBuilder::new()
                .delimiter(delimiter as u8)
                .from_writer(vec![]);

            for item in arr {
                let obj = item.as_object()
                    .ok_or_else(|| AppError::Serialization("Expected object in array".to_string()))?;

                let mut map = HashMap::new();
                for (k, v) in obj {
                    let str_val = v.as_str()
                        .unwrap_or(&serde_json::to_string(v).unwrap_or_default());
                    map.insert(k.as_str(), str_val);
                }

                wtr.serialize(&map)
                    .map_err(|e| AppError::Serialization(format!("CSV write error: {}", e)))?;
            }

            let csv_string = String::from_utf8(wtr.into_inner())
                .map_err(|e| AppError::Serialization(format!("UTF-8 conversion error: {}", e)))?;

            Ok(serde_json::json!(csv_string))
        }
    }

    pub struct EncodingProcessor;
    impl EncodingProcessor {
        pub fn base64_encode(data: &serde_json::Value) -> Result<serde_json::Value> {
            let input = data.as_str()
                .ok_or_else(|| AppError::Serialization("Expected string input".to_string()))?;

            let encoded = base64::encode(input);
            Ok(serde_json::json!(encoded))
        }

        pub fn base64_decode(data: &serde_json::Value) -> Result<serde_json::Value> {
            let input = data.as_str()
                .ok_or_else(|| AppError::Serialization("Expected string input".to_string()))?;

            let decoded = base64::decode(input)
                .map_err(|e| AppError::Serialization(format!("Base64 decode error: {}", e)))?;

            let string = String::from_utf8(decoded)
                .map_err(|e| AppError::Serialization(format!("UTF-8 conversion error: {}", e)))?;

            Ok(serde_json::json!(string))
        }

        pub fn url_encode(data: &serde_json::Value) -> Result<serde_json::Value> {
            let input = data.as_str()
                .ok_or_else(|| AppError::Serialization("Expected string input".to_string()))?;

            let encoded = percent_encoding::utf8_percent_encode(input, percent_encoding::NON_ALPHANUMERIC).to_string();
            Ok(serde_json::json!(encoded))
        }

        pub fn url_decode(data: &serde_json::Value) -> Result<serde_json::Value> {
            let input = data.as_str()
                .ok_or_else(|| AppError::Serialization("Expected string input".to_string()))?;

            let decoded = percent_encoding::percent_decode_str(input)
                .decode_utf8()
                .map_err(|e| AppError::Serialization(format!("URL decode error: {}", e)))?
                .to_string();

            Ok(serde_json::json!(decoded))
        }
    }

    pub struct HashProcessor;
    impl HashProcessor {
        pub fn hash(data: &serde_json::Value, algorithm: HashAlgorithm) -> Result<serde_json::Value> {
            use sha2::{Sha256, Sha512, Digest};
            use md5::{Md5};

            let input = data.as_str()
                .ok_or_else(|| AppError::Serialization("Expected string input".to_string()))?;

            let bytes = input.as_bytes();
            let hash = match algorithm {
                HashAlgorithm::Md5 => {
                    let mut hasher = Md5::new();
                    hasher.update(bytes);
                    hasher.finalize()
                }
                HashAlgorithm::Sha1 => {
                    use sha1::Sha1;
                    let mut hasher = Sha1::new();
                    hasher.update(bytes);
                    hasher.digest().bytes()
                }
                HashAlgorithm::Sha256 => {
                    let mut hasher = Sha256::new();
                    hasher.update(bytes);
                    hasher.finalize()
                }
                HashAlgorithm::Sha512 => {
                    let mut hasher = Sha512::new();
                    hasher.update(bytes);
                    hasher.finalize()
                }
            };

            Ok(serde_json::json!(hex::encode(hash)))
        }
    }

    pub struct FilterProcessor;
    impl FilterProcessor {
        pub fn process(data: &serde_json::Value, _condition: &str) -> Result<serde_json::Value> {
            // Simple filter implementation
            // In production, this would parse and evaluate the condition
            match data {
                serde_json::Value::Array(arr) => {
                    // For now, return all non-null items
                    let filtered: Vec<_> = arr.iter()
                        .filter(|v| !v.is_null())
                        .cloned()
                        .collect();
                    Ok(serde_json::json!(filtered))
                }
                _ => Ok(data.clone()),
            }
        }
    }

    pub struct SortProcessor;
    impl SortProcessor {
        pub fn process(data: &serde_json::Value, key: Option<&str>, order: SortOrder) -> Result<serde_json::Value> {
            let arr = data.as_array()
                .ok_or_else(|| AppError::Serialization("Expected array input".to_string()))?;

            let mut sorted = arr.clone();

            sorted.sort_by(|a, b| {
                let a_val = if let Some(k) = key {
                    a.get(k).unwrap_or(a)
                } else {
                    a
                };

                let b_val = if let Some(k) = key {
                    b.get(k).unwrap_or(b)
                } else {
                    b
                };

                let cmp = compare_json_values(a_val, b_val);
                if order == SortOrder::Desc {
                    cmp.reverse()
                } else {
                    cmp
                }
            });

            Ok(serde_json::json!(sorted))
        }
    }

    fn compare_json_values(a: &serde_json::Value, b: &serde_json::Value) -> std::cmp::Ordering {
        match (a, b) {
            (serde_json::Value::String(a), serde_json::Value::String(b)) => a.cmp(b),
            (serde_json::Value::Number(a), serde_json::Value::Number(b)) => {
                if let (Some(a_f), Some(b_f)) = (a.as_f64(), b.as_f64()) {
                    a_f.partial_cmp(&b_f).unwrap_or(std::cmp::Ordering::Equal)
                } else {
                    std::cmp::Ordering::Equal
                }
            }
            (serde_json::Value::Bool(a), serde_json::Value::Bool(b)) => a.cmp(b),
            _ => std::cmp::Ordering::Equal,
        }
    }

    pub struct GroupProcessor;
    impl GroupProcessor {
        pub fn process(data: &serde_json::Value, key: &str) -> Result<serde_json::Value> {
            let arr = data.as_array()
                .ok_or_else(|| AppError::Serialization("Expected array input".to_string()))?;

            let mut groups: HashMap<String, Vec<serde_json::Value>> = HashMap::new();

            for item in arr {
                let group_key = if let Some(obj) = item.as_object() {
                    obj.get(key)
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string()
                } else {
                    "unknown".to_string()
                };

                groups.entry(group_key).or_default().push(item.clone());
            }

            Ok(serde_json::json!(groups))
        }
    }

    pub struct AggregateProcessor;
    impl AggregateProcessor {
        pub fn process(data: &serde_json::Value, op: AggregateOp, key: Option<&str>) -> Result<serde_json::Value> {
            let arr = data.as_array()
                .ok_or_else(|| AppError::Serialization("Expected array input".to_string()))?;

            let result = match op {
                AggregateOp::Count => {
                    arr.len()
                }
                AggregateOp::Sum => {
                    let mut sum = 0.0;
                    for item in arr {
                        let val = if let Some(k) = key {
                            item.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0)
                        } else {
                            item.as_f64().unwrap_or(0.0)
                        };
                        sum += val;
                    }
                    sum
                }
                AggregateOp::Avg => {
                    let mut sum = 0.0;
                    let mut count = 0;
                    for item in arr {
                        let val = if let Some(k) = key {
                            item.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0)
                        } else {
                            item.as_f64().unwrap_or(0.0)
                        };
                        sum += val;
                        count += 1;
                    }
                    if count > 0 { sum / count as f64 } else { 0.0 }
                }
                AggregateOp::Min => {
                    let mut min = f64::MAX;
                    for item in arr {
                        let val = if let Some(k) = key {
                            item.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0)
                        } else {
                            item.as_f64().unwrap_or(0.0)
                        };
                        min = min.min(val);
                    }
                    min
                }
                AggregateOp::Max => {
                    let mut max = f64::MIN;
                    for item in arr {
                        let val = if let Some(k) = key {
                            item.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0)
                        } else {
                            item.as_f64().unwrap_or(0.0)
                        };
                        max = max.max(val);
                    }
                    max
                }
                _ => {
                    return Err(AppError::Serialization("Aggregate operation not implemented".to_string()));
                }
            };

            Ok(serde_json::json!(result))
        }
    }

    pub struct MergeProcessor;
    impl MergeProcessor {
        pub fn process(_data: &serde_json::Value, _strategy: MergeStrategy) -> Result<serde_json::Value> {
            // Placeholder for merge operation
            Ok(serde_json::json!(null))
        }
    }

    pub struct StringProcessor;
    impl StringProcessor {
        pub fn split(data: &serde_json::Value, separator: &str) -> Result<serde_json::Value> {
            let input = data.as_str()
                .ok_or_else(|| AppError::Serialization("Expected string input".to_string()))?;

            let parts: Vec<&str> = input.split(separator).collect();
            Ok(serde_json::json!(parts))
        }

        pub fn join(data: &serde_json::Value, separator: &str) -> Result<serde_json::Value> {
            let arr = data.as_array()
                .ok_or_else(|| AppError::Serialization("Expected array input".to_string()))?;

            let strings: Vec<&str> = arr.iter()
                .filter_map(|v| v.as_str())
                .collect();

            Ok(serde_json::json!(strings.join(separator)))
        }
    }

    pub struct FormatProcessor;
    impl FormatProcessor {
        pub fn process(data: &serde_json::Value, pattern: &str) -> Result<serde_json::Value> {
            // Simple template formatting: {key} -> value
            let obj = data.as_object()
                .ok_or_else(|| AppError::Serialization("Expected object input".to_string()))?;

            let mut result = pattern.to_string();
            for (key, value) in obj {
                let placeholder = format!("{{{}}}", key);
                let value_str = value.as_str()
                    .unwrap_or(&serde_json::to_string(value).unwrap_or_default());
                result = result.replace(&placeholder, value_str);
            }

            Ok(serde_json::json!(result))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_json_path_processor() {
        let data = serde_json::json!({
            "user": {
                "name": "Alice",
                "age": 30
            }
        });

        let result = JsonPathProcessor::process(&data, "user.name").unwrap();
        assert_eq!(result, "Alice");
    }

    #[tokio::test]
    async fn test_base64_encode() {
        let data = serde_json::json!("Hello");
        let result = EncodingProcessor::base64_encode(&data).unwrap();
        assert_eq!(result, "SGVsbG8=");
    }
}
