// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Command system for the dialog engine
//!
//! Provides command registration, matching, and execution

#![allow(dead_code)]
use crate::error::{AppError, Result};
use crate::dialog::intent::Intent;
use async_trait::async_trait;
use chrono::Timelike;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

/// Command execution context
#[derive(Debug, Clone)]
pub struct CommandContext {
    pub user_id: String,
    pub chat_id: String,
    pub platform: String,
    pub session_id: String,
    pub intent: Intent,
    pub extra: HashMap<String, serde_json::Value>,
}

/// Command execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub follow_up: Option<FollowUp>,
}

impl CommandResult {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
            follow_up: None,
        }
    }

    pub fn success_with_data(message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
            follow_up: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
            follow_up: None,
        }
    }

    pub fn with_follow_up(mut self, follow_up: FollowUp) -> Self {
        self.follow_up = Some(follow_up);
        self
    }

    pub fn need_confirmation(mut self, prompt: impl Into<String>) -> Self {
        self.follow_up = Some(FollowUp {
            follow_up_type: FollowUpType::Confirmation,
            prompt: prompt.into(),
            options: None,
        });
        self
    }

    pub fn need_input(mut self, prompt: impl Into<String>, slot_name: impl Into<String>) -> Self {
        self.follow_up = Some(FollowUp {
            follow_up_type: FollowUpType::SlotFill { slot_name: slot_name.into() },
            prompt: prompt.into(),
            options: None,
        });
        self
    }

    pub fn with_options(mut self, prompt: impl Into<String>, options: Vec<String>) -> Self {
        self.follow_up = Some(FollowUp {
            follow_up_type: FollowUpType::Choice,
            prompt: prompt.into(),
            options: Some(options),
        });
        self
    }
}

/// Follow-up action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowUp {
    pub follow_up_type: FollowUpType,
    pub prompt: String,
    pub options: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum FollowUpType {
    Confirmation,
    SlotFill { slot_name: String },
    Choice,
    Retry,
}

/// Command trait
#[async_trait]
pub trait Command: Send + Sync {
    /// Command name (matches intent name)
    fn name(&self) -> &str;

    /// Command description
    fn description(&self) -> &str;

    /// Aliases for the command
    fn aliases(&self) -> Vec<&str> {
        vec![]
    }

    /// Execute the command
    async fn execute(&self, context: CommandContext) -> Result<CommandResult>;
}

/// Command registry
pub struct CommandRegistry {
    commands: HashMap<String, Arc<dyn Command>>,
    aliases: HashMap<String, String>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
            aliases: HashMap::new(),
        }
    }

    /// Register a command
    pub fn register(&mut self, command: Arc<dyn Command>) {
        let name = command.name().to_string();

        // Register aliases
        for alias in command.aliases() {
            self.aliases.insert(alias.to_string(), name.clone());
        }

        info!("Registered command: {}", name);
        self.commands.insert(name, command);
    }

    /// Unregister a command
    pub fn unregister(&mut self, name: &str) {
        self.commands.remove(name);
        self.aliases.retain(|_, v| v != name);
    }

    /// Get a command by name or alias
    pub fn get(&self, name: &str) -> Option<Arc<dyn Command>> {
        if let Some(cmd) = self.commands.get(name) {
            return Some(cmd.clone());
        }

        // Check aliases
        if let Some(real_name) = self.aliases.get(name) {
            return self.commands.get(real_name).cloned();
        }

        None
    }

    /// Execute a command by name
    pub async fn execute(&self, name: &str, context: CommandContext) -> Result<CommandResult> {
        let command = self.get(name).ok_or_else(|| {
            AppError::Config(format!("Command not found: {}", name))
        })?;

        info!("Executing command '{}' for user '{}'", name, context.user_id);
        command.execute(context).await
    }

    /// List all registered commands
    pub fn list_commands(&self) -> Vec<(&str, &str)> {
        self.commands.iter()
            .map(|(name, cmd)| (name.as_str(), cmd.description()))
            .collect()
    }

    /// Check if a command is registered
    pub fn has_command(&self, name: &str) -> bool {
        self.commands.contains_key(name) || self.aliases.contains_key(name)
    }

    /// Get command count
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Built-in commands

/// Help command
pub struct HelpCommand;

#[async_trait]
impl Command for HelpCommand {
    fn name(&self) -> &str { "help" }
    fn description(&self) -> &str { "显示帮助信息" }

    async fn execute(&self, _context: CommandContext) -> Result<CommandResult> {
        Ok(CommandResult::success(
            "我是 Desktop Agent 助手，可以帮助你：\n\
             📁 文件操作：列出、搜索、复制、移动文件\n\
             💻 系统操作：查看系统信息、启动应用\n\
             ⚡ 技能管理：列出、执行技能\n\
             ❓ 帮助：输入「帮助」查看此信息\n\n\
             请告诉我你需要什么帮助？"
        ))
    }
}

/// Greeting command
pub struct GreetingCommand;

#[async_trait]
impl Command for GreetingCommand {
    fn name(&self) -> &str { "greeting" }
    fn description(&self) -> &str { "问候回复" }

    async fn execute(&self, _context: CommandContext) -> Result<CommandResult> {
        let hour = chrono::Local::now().hour();
        let greeting = match hour {
            0..=5 => "夜深了",
            6..=11 => "早上好",
            12..=13 => "中午好",
            14..=17 => "下午好",
            18..=22 => "晚上好",
            _ => "夜深了",
        };

        Ok(CommandResult::success(format!(
            "{}！我是 Desktop Agent 助手。有什么可以帮助你的？",
            greeting
        )))
    }
}

/// Unknown command fallback
pub struct UnknownCommand;

#[async_trait]
impl Command for UnknownCommand {
    fn name(&self) -> &str { "unknown" }
    fn description(&self) -> &str { "未知命令回退" }

    async fn execute(&self, _context: CommandContext) -> Result<CommandResult> {
        Ok(CommandResult::success(
            "抱歉，我不太理解你的意思。你可以输入「帮助」查看我能做什么。"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog::intent::{Intent, Slot, SlotValue};

    fn make_context(intent_name: &str) -> CommandContext {
        CommandContext {
            user_id: "test_user".to_string(),
            chat_id: "test_chat".to_string(),
            platform: "test".to_string(),
            session_id: "test_session".to_string(),
            intent: Intent {
                name: intent_name.to_string(),
                confidence: 1.0,
                slots: vec![],
                raw_input: "test".to_string(),
            },
            extra: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_registry_register() {
        let mut registry = CommandRegistry::new();
        registry.register(Arc::new(HelpCommand));
        assert!(registry.has_command("help"));
        assert_eq!(registry.command_count(), 1);
    }

    #[tokio::test]
    async fn test_help_command() {
        let cmd = HelpCommand;
        let result = cmd.execute(make_context("help")).await.unwrap();
        assert!(result.success);
        assert!(result.message.contains("Desktop Agent"));
    }

    #[tokio::test]
    async fn test_greeting_command() {
        let cmd = GreetingCommand;
        let result = cmd.execute(make_context("greeting")).await.unwrap();
        assert!(result.success);
    }
}
