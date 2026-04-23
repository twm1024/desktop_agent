// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

#![allow(dead_code)]
use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSearchResult {
    pub line: usize,
    pub content: String,
    pub column: usize,
    pub length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextReplaceResult {
    pub count: usize,
    pub changes: Vec<ReplaceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaceInfo {
    pub line: usize,
    pub original: String,
    pub replacement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextStats {
    pub lines: usize,
    pub words: usize,
    pub characters: usize,
    pub bytes: usize,
}

pub struct TextService;

impl TextService {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    /// Search text in a file
    pub async fn search_file(
        &self,
        path: &Path,
        pattern: &str,
        case_sensitive: bool,
        regex_mode: bool,
    ) -> Result<Vec<TextSearchResult>> {
        info!("Searching in {:?} for pattern: {}", path, pattern);

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| AppError::io(e))?;

        let results = if regex_mode {
            self.search_with_regex(&content, pattern, case_sensitive)?
        } else {
            self.search_with_literal(&content, pattern, case_sensitive)?
        };

        Ok(results)
    }

    fn search_with_regex(
        &self,
        content: &str,
        pattern: &str,
        case_sensitive: bool,
    ) -> Result<Vec<TextSearchResult>> {
        let mut results = Vec::new();

        let flags = if case_sensitive {
            regex::Regex::new(pattern)
        } else {
            regex::Regex::new(&format!("(?i){}", pattern))
        };

        let regex = flags.map_err(|e| AppError::invalid_input(format!("Invalid regex: {}", e)))?;

        for (line_num, line) in content.lines().enumerate() {
            for mat in regex.find_iter(line) {
                results.push(TextSearchResult {
                    line: line_num + 1,
                    content: line.to_string(),
                    column: mat.start(),
                    length: mat.end() - mat.start(),
                });
            }
        }

        Ok(results)
    }

    fn search_with_literal(
        &self,
        content: &str,
        pattern: &str,
        case_sensitive: bool,
    ) -> Result<Vec<TextSearchResult>> {
        let mut results = Vec::new();

        let _search_content = if case_sensitive {
            content.to_string()
        } else {
            content.to_lowercase()
        };

        let search_pattern = if case_sensitive {
            pattern.to_string()
        } else {
            pattern.to_lowercase()
        };

        for (line_num, line) in content.lines().enumerate() {
            let line_to_search = if case_sensitive {
                line.to_string()
            } else {
                line.to_lowercase()
            };

            let mut start = 0;
            while let Some(pos) = line_to_search[start..].find(&search_pattern) {
                results.push(TextSearchResult {
                    line: line_num + 1,
                    content: line.to_string(),
                    column: start + pos,
                    length: pattern.len(),
                });
                start += pos + pattern.len();
            }
        }

        Ok(results)
    }

    /// Replace text in a file
    pub async fn replace_in_file(
        &self,
        path: &Path,
        pattern: &str,
        replacement: &str,
        case_sensitive: bool,
        regex_mode: bool,
        create_backup: bool,
    ) -> Result<TextReplaceResult> {
        info!("Replacing in {:?} pattern: {} with {}", path, pattern, replacement);

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| AppError::io(e))?;

        // Create backup if requested
        if create_backup {
            let backup_path = format!("{}.backup", path.display());
            tokio::fs::copy(path, &backup_path).await?;
            debug!("Created backup: {}", backup_path);
        }

        let (new_content, changes) = if regex_mode {
            self.replace_with_regex(&content, pattern, replacement, case_sensitive)?
        } else {
            self.replace_with_literal(&content, pattern, replacement, case_sensitive)?
        };

        if changes.is_empty() {
            return Ok(TextReplaceResult {
                count: 0,
                changes: Vec::new(),
            });
        }

        tokio::fs::write(path, new_content)
            .await
            .map_err(|e| AppError::io(e))?;

        Ok(TextReplaceResult {
            count: changes.len(),
            changes,
        })
    }

    fn replace_with_regex(
        &self,
        content: &str,
        pattern: &str,
        replacement: &str,
        case_sensitive: bool,
    ) -> Result<(String, Vec<ReplaceInfo>)> {
        let flags = if case_sensitive {
            regex::Regex::new(pattern)
        } else {
            regex::Regex::new(&format!("(?i){}", pattern))
        };

        let regex = flags.map_err(|e| AppError::invalid_input(format!("Invalid regex: {}", e)))?;

        let mut changes = Vec::new();
        let new_content = regex.replace_all(content, replacement).to_string();

        // Find changes
        for (line_num, line) in content.lines().enumerate() {
            if regex.is_match(line) {
                let replaced = regex.replace_all(line, replacement).to_string();
                if replaced != line {
                    changes.push(ReplaceInfo {
                        line: line_num + 1,
                        original: line.to_string(),
                        replacement: replaced,
                    });
                }
            }
        }

        Ok((new_content, changes))
    }

    fn replace_with_literal(
        &self,
        content: &str,
        pattern: &str,
        replacement: &str,
        case_sensitive: bool,
    ) -> Result<(String, Vec<ReplaceInfo>)> {
        let _search_content = if case_sensitive {
            content.to_string()
        } else {
            content.to_lowercase()
        };

        let search_pattern = if case_sensitive {
            pattern.to_string()
        } else {
            pattern.to_lowercase()
        };

        let mut new_content = String::new();
        let mut changes = Vec::new();
        let mut lines_processed = 0;

        for line in content.lines() {
            lines_processed += 1;
            let line_to_search = if case_sensitive {
                line.to_string()
            } else {
                line.to_lowercase()
            };

            if line_to_search.contains(&search_pattern) {
                let replaced = if case_sensitive {
                    line.replace(pattern, replacement)
                } else {
                    let mut result = String::new();
                    let mut pos = 0;
                    while let Some(found) = line_to_search[pos..].find(&search_pattern) {
                        result.push_str(&line[pos..pos + found]);
                        result.push_str(replacement);
                        pos += found + pattern.len();
                    }
                    result.push_str(&line[pos..]);
                    result
                };

                changes.push(ReplaceInfo {
                    line: lines_processed,
                    original: line.to_string(),
                    replacement: replaced.clone(),
                });

                new_content.push_str(&replaced);
            } else {
                new_content.push_str(line);
            }
            new_content.push('\n');
        }

        Ok((new_content, changes))
    }

    /// Get text statistics
    pub async fn get_text_stats(&self, path: &Path) -> Result<TextStats> {
        info!("Getting text stats for {:?}", path);

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| AppError::io(e))?;

        let lines = content.lines().count();
        let words = content.split_whitespace().count();
        let characters = content.chars().count();
        let bytes = content.len();

        Ok(TextStats {
            lines,
            words,
            characters,
            bytes,
        })
    }

    /// Count occurrences of a pattern
    pub async fn count_occurrences(
        &self,
        path: &Path,
        pattern: &str,
        case_sensitive: bool,
    ) -> Result<usize> {
        info!("Counting occurrences in {:?}", path);

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| AppError::io(e))?;

        let count = if case_sensitive {
            content.matches(pattern).count()
        } else {
            let lower_content = content.to_lowercase();
            let lower_pattern = pattern.to_lowercase();
            lower_content.matches(&lower_pattern).count()
        };

        Ok(count)
    }
}
