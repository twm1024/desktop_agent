#![allow(dead_code)]
// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Application configuration management
//!
//! Handles loading, saving, and accessing application configuration

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, warn};

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Application name
    pub app_name: String,
    /// Application version
    pub version: String,
    /// Data directory
    pub data_dir: PathBuf,
    /// Database configuration
    pub database: DatabaseConfig,
    /// Server configuration
    pub server: ServerConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// Platform configurations
    pub platforms: PlatformConfigs,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Skill configuration
    pub skills: SkillConfig,
    /// Plugin configuration
    pub plugins: PluginConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: Option<PathBuf>,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_secs: u64,
    pub idle_timeout_secs: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: None,
            max_connections: 10,
            min_connections: 2,
            connect_timeout_secs: 30,
            idle_timeout_secs: 600,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
    pub max_body_size: usize,
    pub request_timeout_secs: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 17321,
            cors_origins: vec!["http://localhost:1420".to_string()],
            max_body_size: 10 * 1024 * 1024, // 10MB
            request_timeout_secs: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub rate_limit_per_minute: u32,
    pub rate_limit_per_hour: u32,
    pub max_sessions_per_user: u32,
    pub session_timeout_secs: u64,
    pub enable_audit_log: bool,
    pub enable_log_sanitization: bool,
    pub allowed_skill_paths: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            rate_limit_per_minute: 60,
            rate_limit_per_hour: 1000,
            max_sessions_per_user: 5,
            session_timeout_secs: 3600,
            enable_audit_log: true,
            enable_log_sanitization: true,
            allowed_skill_paths: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfigs {
    pub feishu: Option<FeishuConfig>,
    pub wecom: Option<WeComConfig>,
    pub dingtalk: Option<DingTalkConfig>,
}

impl Default for PlatformConfigs {
    fn default() -> Self {
        Self {
            feishu: None,
            wecom: None,
            dingtalk: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuConfig {
    pub app_id: String,
    pub app_secret: String,
    pub verification_token: String,
    pub encrypt_key: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeComConfig {
    pub corp_id: String,
    pub agent_id: String,
    pub secret: String,
    pub token: String,
    pub encoding_aes_key: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingTalkConfig {
    pub client_id: String,
    pub client_secret: String,
    pub robot_code: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file_enabled: bool,
    pub file_path: Option<PathBuf>,
    pub max_file_size_mb: u64,
    pub max_files: usize,
    pub console_enabled: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file_enabled: true,
            file_path: None,
            max_file_size_mb: 50,
            max_files: 5,
            console_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillConfig {
    pub skills_dir: Option<PathBuf>,
    pub max_concurrent_executions: usize,
    pub default_timeout_secs: u64,
    pub max_memory_mb: u64,
    pub sandbox_enabled: bool,
}

impl Default for SkillConfig {
    fn default() -> Self {
        Self {
            skills_dir: None,
            max_concurrent_executions: 4,
            default_timeout_secs: 300,
            max_memory_mb: 512,
            sandbox_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub plugins_dir: Option<PathBuf>,
    pub auto_load: bool,
    pub max_plugins: usize,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            plugins_dir: None,
            auto_load: true,
            max_plugins: 50,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = Self::default_data_dir();
        Self {
            app_name: "Desktop Agent".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            data_dir: data_dir.clone(),
            database: DatabaseConfig {
                path: Some(data_dir.join("data.db")),
                ..Default::default()
            },
            server: Default::default(),
            security: Default::default(),
            platforms: Default::default(),
            logging: LoggingConfig {
                file_path: Some(data_dir.join("logs")),
                ..Default::default()
            },
            skills: SkillConfig {
                skills_dir: Some(data_dir.join("skills")),
                ..Default::default()
            },
            plugins: PluginConfig {
                plugins_dir: Some(data_dir.join("plugins")),
                ..Default::default()
            },
        }
    }
}

impl Config {
    /// Default data directory
    fn default_data_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("desktop-agent")
    }

    /// Load configuration from file
    pub async fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = tokio::fs::read_to_string(&config_path).await?;
            let config: Config = serde_json::from_str(&content)?;
            info!("Configuration loaded from {:?}", config_path);
            Ok(config)
        } else {
            let config = Config::default();
            config.save().await?;
            info!("Created default configuration at {:?}", config_path);
            Ok(config)
        }
    }

    /// Save configuration to file
    pub async fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        if let Some(parent) = config_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(self)?;
        tokio::fs::write(&config_path, content).await?;
        Ok(())
    }

    /// Get configuration file path
    pub fn config_path() -> Result<PathBuf> {
        let data_dir = Self::data_dir()?;
        Ok(data_dir.join("config.json"))
    }

    /// Get data directory
    pub fn data_dir() -> Result<PathBuf> {
        let dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("desktop-agent");
        Ok(dir)
    }

    /// Get database path
    pub fn database_path() -> Result<PathBuf> {
        let data_dir = Self::data_dir()?;
        Ok(data_dir.join("data.db"))
    }

    /// Get skills directory
    pub fn skill_dir() -> Result<PathBuf> {
        let data_dir = Self::data_dir()?;
        Ok(data_dir.join("skills"))
    }

    /// Get plugins directory
    pub fn plugins_dir() -> Result<PathBuf> {
        let data_dir = Self::data_dir()?;
        Ok(data_dir.join("plugins"))
    }

    /// Get logs directory
    pub fn logs_dir() -> Result<PathBuf> {
        let data_dir = Self::data_dir()?;
        Ok(data_dir.join("logs"))
    }

    /// Get a configuration value by key path
    pub fn get_value(&self, key: &str) -> Option<serde_json::Value> {
        let full = serde_json::to_value(self).ok()?;
        key.split('.')
            .fold(Some(full), |acc, k| {
                acc.and_then(|v| v.get(k).cloned())
            })
    }

    /// Set a configuration value by key path
    pub async fn set_value(&mut self, key: &str, value: &str) -> Result<()> {
        let value: serde_json::Value = serde_json::from_str(value)
            .unwrap_or(serde_json::Value::String(value.to_string()));

        match key {
            "server.host" => self.server.host = value.as_str().unwrap_or_default().to_string(),
            "server.port" => self.server.port = value.as_u64().unwrap_or(17321) as u16,
            "security.rate_limit_per_minute" => {
                self.security.rate_limit_per_minute = value.as_u64().unwrap_or(60) as u32;
            }
            "security.enable_audit_log" => {
                self.security.enable_audit_log = value.as_bool().unwrap_or(true);
            }
            "logging.level" => self.logging.level = value.as_str().unwrap_or("info").to_string(),
            _ => {
                warn!("Unknown config key: {}", key);
            }
        }

        self.save().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.app_name, "Desktop Agent");
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 17321);
    }

    #[test]
    fn test_get_value() {
        let config = Config::default();
        let value = config.get_value("server.host");
        assert_eq!(value, Some(serde_json::Value::String("127.0.0.1".to_string())));
    }
}
