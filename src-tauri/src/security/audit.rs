// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Audit logging for security events
//!
//! Records security-relevant events for compliance and incident investigation

#![allow(dead_code)]
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};

/// Audit event severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditLevel {
    Info,
    Warning,
    Critical,
}

/// Audit event category
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditCategory {
    Authentication,
    Authorization,
    DataAccess,
    Configuration,
    SkillExecution,
    FileSystem,
    Network,
    System,
    UserManagement,
    Backup,
    Plugin,
}

/// An audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: i64,
    pub level: AuditLevel,
    pub category: AuditCategory,
    pub action: String,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub source_ip: Option<String>,
    pub resource: Option<String>,
    pub details: Option<serde_json::Value>,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Audit log configuration
#[derive(Debug, Clone)]
pub struct AuditConfig {
    pub enabled: bool,
    pub log_dir: PathBuf,
    pub max_file_size_bytes: u64,
    pub max_files: usize,
    pub flush_interval_secs: u64,
    pub log_to_stdout: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_dir: PathBuf::from("logs/audit"),
            max_file_size_bytes: 50 * 1024 * 1024, // 50MB
            max_files: 10,
            flush_interval_secs: 5,
            log_to_stdout: false,
        }
    }
}

/// Audit logger
pub struct AuditLogger {
    config: AuditConfig,
    buffer: Arc<RwLock<Vec<AuditEntry>>>,
    current_file: Arc<RwLock<Option<PathBuf>>>,
    current_size: Arc<RwLock<u64>>,
}

impl AuditLogger {
    pub fn new(config: AuditConfig) -> Self {
        Self {
            config,
            buffer: Arc::new(RwLock::new(Vec::new())),
            current_file: Arc::new(RwLock::new(None)),
            current_size: Arc::new(RwLock::new(0)),
        }
    }

    /// Log an audit event
    pub async fn log(
        &self,
        level: AuditLevel,
        category: AuditCategory,
        action: &str,
        user_id: Option<&str>,
        session_id: Option<&str>,
        source_ip: Option<&str>,
        resource: Option<&str>,
        details: Option<serde_json::Value>,
        success: bool,
        error_message: Option<&str>,
    ) {
        if !self.config.enabled {
            return;
        }

        let entry = AuditEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: crate::utils::current_timestamp(),
            level,
            category,
            action: action.to_string(),
            user_id: user_id.map(|s| s.to_string()),
            session_id: session_id.map(|s| s.to_string()),
            source_ip: source_ip.map(|s| s.to_string()),
            resource: resource.map(|s| s.to_string()),
            details,
            success,
            error_message: error_message.map(|s| s.to_string()),
        };

        if self.config.log_to_stdout {
            info!(
                "[AUDIT] {} {:?} {:?} {} user={:?} resource={:?} success={}",
                entry.id, entry.level, entry.category, entry.action,
                entry.user_id, entry.resource, entry.success
            );
        }

        let mut buffer = self.buffer.write().await;
        buffer.push(entry);

        // Auto-flush when buffer reaches threshold
        if buffer.len() >= 100 {
            drop(buffer);
            if let Err(e) = self.flush().await {
                warn!("Failed to flush audit log: {}", e);
            }
        }
    }

    /// Log a simple audit event
    pub async fn log_event(
        &self,
        category: AuditCategory,
        action: &str,
        success: bool,
    ) {
        self.log(
            if success { AuditLevel::Info } else { AuditLevel::Warning },
            category,
            action,
            None, None, None, None,
            None,
            success,
            None,
        ).await;
    }

    /// Log an authentication event
    pub async fn log_auth(
        &self,
        user_id: &str,
        action: &str,
        success: bool,
        source_ip: Option<&str>,
    ) {
        self.log(
            if success { AuditLevel::Info } else { AuditLevel::Warning },
            AuditCategory::Authentication,
            action,
            Some(user_id),
            None,
            source_ip,
            None,
            None,
            success,
            if success { None } else { Some("Authentication failed") },
        ).await;
    }

    /// Log a data access event
    pub async fn log_data_access(
        &self,
        user_id: &str,
        resource: &str,
        action: &str,
        success: bool,
    ) {
        self.log(
            AuditLevel::Info,
            AuditCategory::DataAccess,
            action,
            Some(user_id),
            None,
            None,
            Some(resource),
            None,
            success,
            None,
        ).await;
    }

    /// Log a skill execution event
    pub async fn log_skill_execution(
        &self,
        user_id: &str,
        skill_name: &str,
        success: bool,
        duration_ms: u64,
        error: Option<&str>,
    ) {
        self.log(
            if success { AuditLevel::Info } else { AuditLevel::Warning },
            AuditCategory::SkillExecution,
            "execute",
            Some(user_id),
            None,
            None,
            Some(skill_name),
            Some(serde_json::json!({ "duration_ms": duration_ms })),
            success,
            error,
        ).await;
    }

    /// Log a configuration change
    pub async fn log_config_change(
        &self,
        user_id: &str,
        key: &str,
        old_value: Option<&str>,
        new_value: Option<&str>,
    ) {
        self.log(
            AuditLevel::Warning,
            AuditCategory::Configuration,
            "config_change",
            Some(user_id),
            None,
            None,
            Some(key),
            Some(serde_json::json!({
                "old_value": old_value.map(|_v| "[REDACTED]"),
                "new_value": new_value.map(|_v| "[REDACTED]"),
            })),
            true,
            None,
        ).await;
    }

    /// Flush buffered entries to disk
    pub async fn flush(&self) -> Result<()> {
        let entries = {
            let mut buffer = self.buffer.write().await;
            std::mem::take(&mut *buffer)
        };

        if entries.is_empty() {
            return Ok(());
        }

        // Ensure log directory exists
        tokio::fs::create_dir_all(&self.config.log_dir).await?;

        // Get or create current log file
        let log_file = self.get_current_log_file().await?;

        // Append entries
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .await?;

        for entry in &entries {
            let line = serde_json::to_string(entry)? + "\n";
            file.write_all(line.as_bytes()).await?;
        }

        file.flush().await?;

        // Update current file size
        let mut current_size = self.current_size.write().await;
        *current_size += entries.iter()
            .map(|e| serde_json::to_string(e).map(|s| s.len() + 1).unwrap_or(0) as u64)
            .sum::<u64>();

        // Rotate if needed
        if *current_size > self.config.max_file_size_bytes {
            drop(current_size);
            self.rotate_log().await?;
        }

        Ok(())
    }

    async fn get_current_log_file(&self) -> Result<PathBuf> {
        let mut current_file = self.current_file.write().await;

        if let Some(file) = current_file.as_ref() {
            if file.exists() {
                return Ok(file.clone());
            }
        }

        let now = chrono::Utc::now();
        let filename = format!("audit_{}.jsonl", now.format("%Y%m%d_%H%M%S"));
        let path = self.config.log_dir.join(&filename);

        *current_file = Some(path.clone());
        Ok(path)
    }

    async fn rotate_log(&self) -> Result<()> {
        // Reset current file and size
        *self.current_file.write().await = None;
        *self.current_size.write().await = 0;

        // Clean up old files
        self.cleanup_old_files().await?;
        Ok(())
    }

    async fn cleanup_old_files(&self) -> Result<()> {
        let mut files: Vec<(i64, PathBuf)> = Vec::new();

        let mut entries = tokio::fs::read_dir(&self.config.log_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                let modified = entry.metadata().await?
                    .modified()?
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                files.push((modified, path));
            }
        }

        // Sort by modification time (oldest first)
        files.sort_by_key(|(time, _)| *time);

        // Remove oldest files if exceeding max_files
        while files.len() > self.config.max_files {
            let (_, old_file) = files.remove(0);
            if let Err(e) = tokio::fs::remove_file(&old_file).await {
                warn!("Failed to remove old audit log {:?}: {}", old_file, e);
            }
        }

        Ok(())
    }

    /// Get recent audit entries from log files
    pub async fn get_recent_entries(&self, limit: usize) -> Result<Vec<AuditEntry>> {
        let mut all_entries = Vec::new();

        let mut entries = tokio::fs::read_dir(&self.config.log_dir).await?;
        let mut files = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            files.push(entry.path());
        }

        // Sort by name (contains timestamp) descending
        files.sort_by(|a, b| b.cmp(a));

        for file in files {
            if all_entries.len() >= limit {
                break;
            }

            let content = tokio::fs::read_to_string(&file).await?;
            for line in content.lines().rev() {
                if all_entries.len() >= limit {
                    break;
                }
                if let Ok(entry) = serde_json::from_str::<AuditEntry>(line) {
                    all_entries.push(entry);
                }
            }
        }

        all_entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all_entries.truncate(limit);
        Ok(all_entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_audit_log_event() {
        let temp_dir = TempDir::new().unwrap();
        let config = AuditConfig {
            enabled: true,
            log_dir: temp_dir.path().join("audit"),
            log_to_stdout: false,
            ..Default::default()
        };

        let logger = AuditLogger::new(config);
        logger.log_event(AuditCategory::Authentication, "login", true).await;
        logger.log_auth("user1", "login", true, Some("127.0.0.1")).await;
        logger.flush().await.unwrap();

        let entries = logger.get_recent_entries(10).await.unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].category, AuditCategory::Authentication);
    }

    #[tokio::test]
    async fn test_audit_disabled() {
        let config = AuditConfig {
            enabled: false,
            log_dir: PathBuf::from("/tmp/nonexistent"),
            ..Default::default()
        };

        let logger = AuditLogger::new(config);
        // Should not error even with invalid path since disabled
        logger.log_event(AuditCategory::System, "test", true).await;
    }

    #[tokio::test]
    async fn test_audit_skill_execution() {
        let temp_dir = TempDir::new().unwrap();
        let config = AuditConfig {
            enabled: true,
            log_dir: temp_dir.path().join("audit"),
            log_to_stdout: false,
            ..Default::default()
        };

        let logger = AuditLogger::new(config);
        logger.log_skill_execution("user1", "ocr_skill", true, 1500, None).await;
        logger.flush().await.unwrap();

        let entries = logger.get_recent_entries(10).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].category, AuditCategory::SkillExecution);
    }
}
