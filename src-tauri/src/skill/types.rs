// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Skill execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillContext {
    pub user_id: String,
    pub chat_id: String,
    pub platform: String,
    pub session_id: String,
    pub timestamp: i64,
}

/// Skill execution parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillParameters {
    pub values: serde_json::Value,
}

/// Skill execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillResult {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub message: String,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// Skill execution progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillProgress {
    pub skill_id: String,
    pub task_id: String,
    pub progress: u8, // 0-100
    pub message: String,
    pub current: usize,
    pub total: usize,
}

/// Skill information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub tags: Vec<String>,
    pub enabled: bool,
    pub installed_at: i64,
    pub last_executed_at: Option<i64>,
    pub execution_count: u64,
    pub icon: Option<String>,
}

impl SkillResult {
    pub fn success(data: Option<serde_json::Value>, message: String) -> Self {
        Self {
            success: true,
            data,
            message,
            error: None,
            execution_time_ms: 0,
        }
    }

    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            data: None,
            message: "Execution failed".to_string(),
            error: Some(error),
            execution_time_ms: 0,
        }
    }
}

impl Default for SkillResult {
    fn default() -> Self {
        Self::success(None, String::new())
    }
}
