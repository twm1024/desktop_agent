// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

#![allow(dead_code)]
use crate::config::Config;
use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified: i64,
    pub permissions: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    pub modified: i64,
    pub matches: Option<Vec<MatchInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchInfo {
    pub line: usize,
    pub content: String,
    pub start: usize,
    pub end: usize,
}

pub struct FileService {
    config: Config,
}

impl FileService {
    pub fn new(config: Config) -> Result<Self> {
        Ok(Self { config })
    }

    /// List directory contents
    pub async fn list_directory(
        &self,
        path: &Path,
        show_hidden: bool,
        recursive: bool,
    ) -> Result<Vec<FileInfo>> {
        info!("Listing directory: {:?} (recursive: {})", path, recursive);

        if !path.exists() {
            return Err(AppError::not_found(format!("Path not found: {:?}", path)));
        }

        let mut files = Vec::new();

        if recursive {
            self.list_recursive(path, show_hidden, &mut files)?;
        } else {
            self.list_flat(path, show_hidden, &mut files)?;
        }

        Ok(files)
    }

    fn list_flat(&self, path: &Path, show_hidden: bool, files: &mut Vec<FileInfo>) -> Result<()> {
        let entries = fs::read_dir(path)
            .map_err(|e| AppError::io(e))?;

        for entry in entries {
            let entry = entry.map_err(|e| AppError::io(e))?;
            let file_path = entry.path();

            // Check hidden files
            let name = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            if !show_hidden && name.starts_with('.') {
                continue;
            }

            let metadata = entry.metadata().map_err(|e| AppError::io(e))?;

            let file_info = FileInfo {
                name: name.clone(),
                path: file_path.clone(),
                is_dir: metadata.is_dir(),
                size: metadata.len(),
                modified: metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0),
                permissions: None, // TODO: Implement permissions
            };

            files.push(file_info);
        }

        Ok(())
    }

    fn list_recursive(
        &self,
        path: &Path,
        show_hidden: bool,
        files: &mut Vec<FileInfo>,
    ) -> Result<()> {
        let entries = fs::read_dir(path)
            .map_err(|e| AppError::io(e))?;

        for entry in entries {
            let entry = entry.map_err(|e| AppError::io(e))?;
            let file_path = entry.path();

            // Check hidden files
            let name = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            if !show_hidden && name.starts_with('.') {
                continue;
            }

            let metadata = entry.metadata().map_err(|e| AppError::io(e))?;

            let file_info = FileInfo {
                name: name.clone(),
                path: file_path.clone(),
                is_dir: metadata.is_dir(),
                size: metadata.len(),
                modified: metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0),
                permissions: None,
            };

            files.push(file_info);

            // Recurse into directories
            if metadata.is_dir() {
                self.list_recursive(&file_path, show_hidden, files)?;
            }
        }

        Ok(())
    }

    /// Search files by name pattern and/or content
    pub async fn search_files(
        &self,
        path: &Path,
        name_pattern: Option<&str>,
        content_pattern: Option<&str>,
        max_results: usize,
    ) -> Result<Vec<SearchResult>> {
        info!("Searching files in {:?}", path);

        if !path.exists() {
            return Err(AppError::not_found(format!("Path not found: {:?}", path)));
        }

        let mut results = Vec::new();

        if let Some(pattern) = name_pattern {
            self.search_by_name(path, pattern, max_results, &mut results)?;
        }

        if let Some(pattern) = content_pattern {
            self.search_by_content(path, pattern, max_results, &mut results)?;
        }

        Ok(results)
    }

    fn search_by_name(
        &self,
        path: &Path,
        pattern: &str,
        max_results: usize,
        results: &mut Vec<SearchResult>,
    ) -> Result<()> {
        // Convert glob pattern to regex
        let regex_pattern = glob_to_regex(pattern);
        let regex = regex::Regex::new(&regex_pattern)
            .map_err(|e| AppError::invalid_input(format!("Invalid pattern: {}", e)))?;

        let entries = fs::read_dir(path).map_err(|e| AppError::io(e))?;

        for entry in entries {
            if results.len() >= max_results {
                break;
            }

            let entry = entry.map_err(|e| AppError::io(e))?;
            let file_path = entry.path();

            if let Some(name) = file_path.file_name().and_then(|n| n.to_str()) {
                if regex.is_match(name) {
                    let metadata = entry.metadata().map_err(|e| AppError::io(e))?;

                    results.push(SearchResult {
                        path: file_path.clone(),
                        name: name.to_string(),
                        size: metadata.len(),
                        modified: metadata
                            .modified()
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0),
                        matches: None,
                    });
                }
            }

            // Recurse into directories
            if entry.path().is_dir() {
                self.search_by_name(&entry.path(), pattern, max_results, results)?;
            }
        }

        Ok(())
    }

    fn search_by_content(
        &self,
        path: &Path,
        pattern: &str,
        max_results: usize,
        results: &mut Vec<SearchResult>,
    ) -> Result<()> {
        let regex = regex::Regex::new(pattern)
            .map_err(|e| AppError::invalid_input(format!("Invalid regex: {}", e)))?;

        let entries = fs::read_dir(path).map_err(|e| AppError::io(e))?;

        for entry in entries {
            if results.len() >= max_results {
                break;
            }

            let entry = entry.map_err(|e| AppError::io(e))?;
            let file_path = entry.path();

            if file_path.is_file() {
                if let Ok(content) = fs::read_to_string(&file_path) {
                    let mut matches = Vec::new();

                    for (line_num, line) in content.lines().enumerate() {
                        if let Some(m) = regex.find(line) {
                            matches.push(MatchInfo {
                                line: line_num + 1,
                                content: line.to_string(),
                                start: m.start(),
                                end: m.end(),
                            });
                        }
                    }

                    if !matches.is_empty() {
                        let metadata = entry.metadata().map_err(|e| AppError::io(e))?;
                        let name = file_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_string();

                        results.push(SearchResult {
                            path: file_path.clone(),
                            name,
                            size: metadata.len(),
                            modified: metadata
                                .modified()
                                .ok()
                                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                .map(|d| d.as_secs() as i64)
                                .unwrap_or(0),
                            matches: Some(matches),
                        });
                    }
                }
            } else if file_path.is_dir() {
                self.search_by_content(&file_path, pattern, max_results, results)?;
            }
        }

        Ok(())
    }

    /// Copy a file or directory
    pub async fn copy_file(&self, src: &Path, dst: &Path, overwrite: bool) -> Result<()> {
        info!("Copying {:?} to {:?}", src, dst);

        if !src.exists() {
            return Err(AppError::not_found(format!("Source not found: {:?}", src)));
        }

        if dst.exists() && !overwrite {
            return Err(AppError::invalid_input(format!("Destination exists: {:?}", dst)));
        }

        if src.is_dir() {
            self.copy_directory(src, dst)?;
        } else {
            fs::copy(src, dst).map_err(|e| AppError::io(e))?;
        }

        Ok(())
    }

    fn copy_directory(&self, src: &Path, dst: &Path) -> Result<()> {
        fs::create_dir_all(dst).map_err(|e| AppError::io(e))?;

        for entry in fs::read_dir(src).map_err(|e| AppError::io(e))? {
            let entry = entry.map_err(|e| AppError::io(e))?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                self.copy_directory(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path).map_err(|e| AppError::io(e))?;
            }
        }

        Ok(())
    }

    /// Move a file or directory
    pub async fn move_file(&self, src: &Path, dst: &Path, overwrite: bool) -> Result<()> {
        info!("Moving {:?} to {:?}", src, dst);

        if !src.exists() {
            return Err(AppError::not_found(format!("Source not found: {:?}", src)));
        }

        if dst.exists() && !overwrite {
            return Err(AppError::invalid_input(format!("Destination exists: {:?}", dst)));
        }

        fs::rename(src, dst).map_err(|e| AppError::io(e))?;

        Ok(())
    }

    /// Delete files or directories
    pub async fn delete_files(&self, paths: &[PathBuf], recursive: bool) -> Result<Vec<String>> {
        info!("Deleting {} files (recursive: {})", paths.len(), recursive);

        let mut deleted = Vec::new();
        let mut failed = Vec::new();

        for path in paths {
            if !path.exists() {
                failed.push(format!("{:?}: Not found", path));
                continue;
            }

            let result = if path.is_dir() {
                if recursive {
                    fs::remove_dir_all(path)
                } else {
                    fs::remove_dir(path)
                }
            } else {
                fs::remove_file(path)
            };

            match result {
                Ok(_) => {
                    debug!("Deleted: {:?}", path);
                    deleted.push(path.display().to_string());
                }
                Err(e) => {
                    failed.push(format!("{:?}: {}", path, e));
                }
            }
        }

        if !failed.is_empty() {
            return Err(AppError::internal(format!(
                "Some files failed to delete: {:?}",
                failed
            )));
        }

        Ok(deleted)
    }
}

/// Convert glob pattern to regex
fn glob_to_regex(pattern: &str) -> String {
    let mut regex = String::new();
    let mut chars = pattern.chars().peekable();

    regex.push('^');

    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next(); // consume second *
                    regex.push_str(".*");
                } else {
                    regex.push_str("[^/]*");
                }
            }
            '?' => regex.push_str("[^/]"),
            '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' => {
                regex.push('\\');
                regex.push(c);
            }
            _ => regex.push(c),
        }
    }

    regex.push('$');
    regex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_to_regex() {
        assert_eq!(glob_to_regex("*.txt"), r"^[^/]*\.txt$");
        assert_eq!(glob_to_regex("test_*.py"), r"^test_[^/]*\.py$");
        assert_eq!(glob_to_regex("**/*.md"), r"^.*\.md$");
    }
}
