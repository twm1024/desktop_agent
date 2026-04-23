// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Command-line interface support
//!
//! Provides CLI commands for managing Desktop Agent from the terminal

#![allow(dead_code)]
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// CLI command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CliCommand {
    /// Start the application
    Start { config: Option<PathBuf> },

    /// Stop the application
    Stop { force: bool },

    /// Show application status
    Status,

    /// List skills
    SkillList { filter: Option<String> },

    /// Install a skill
    SkillInstall { name: String, version: Option<String> },

    /// Uninstall a skill
    SkillUninstall { name: String },

    /// Execute a skill
    SkillExecute {
        name: String,
        params: Option<serde_json::Value>,
    },

    /// Update skills
    SkillUpdate { name: Option<String> },

    /// Search for skills in market
    SkillSearch { query: String },

    /// Show system information
    SystemInfo,

    /// Show database statistics
    DbStats,

    /// Run database migration
    DbMigrate,

    /// Backup database
    Backup { output: Option<PathBuf> },

    /// Restore database
    Restore { input: PathBuf },

    /// Show logs
    Logs {
        level: Option<String>,
        follow: bool,
        lines: Option<usize>,
    },

    /// Manage users
    UserList,
    UserCreate { name: String, role: Option<String> },
    UserDelete { id: String },
    UserRole { id: String, role: String },

    /// Manage configuration
    ConfigGet { key: Option<String> },
    ConfigSet { key: String, value: String },

    /// Show help
    Help { command: Option<String> },

    /// Show version
    Version,
}

/// CLI output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
    Yaml,
    Table,
}

/// CLI response
#[derive(Debug, Clone)]
pub struct CliResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub format: OutputFormat,
}

impl CliResponse {
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
            format: OutputFormat::Text,
        }
    }

    pub fn ok_with_data(message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
            format: OutputFormat::Json,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
            format: OutputFormat::Text,
        }
    }

    /// Format the response for terminal output
    pub fn format_output(&self) -> String {
        match self.format {
            OutputFormat::Json => {
                let output = serde_json::json!({
                    "success": self.success,
                    "message": self.message,
                    "data": self.data,
                });
                serde_json::to_string_pretty(&output).unwrap_or_default()
            }
            OutputFormat::Text => {
                if let Some(data) = &self.data {
                    format!("{}\n{}", self.message, serde_json::to_string_pretty(data).unwrap_or_default())
                } else {
                    self.message.clone()
                }
            }
            _ => self.message.clone(),
        }
    }
}

/// CLI argument parser
pub struct CliParser;

impl CliParser {
    /// Parse CLI arguments into a command
    pub fn parse(args: &[String]) -> Result<Option<CliCommand>> {
        if args.is_empty() {
            return Ok(None);
        }

        let command = &args[0];
        let rest = &args[1..];

        match command.as_str() {
            "start" => Ok(Some(CliCommand::Start {
                config: rest.first().map(PathBuf::from),
            })),
            "stop" => Ok(Some(CliCommand::Stop {
                force: rest.contains(&"--force".to_string()),
            })),
            "status" => Ok(Some(CliCommand::Status)),
            "version" | "-v" | "--version" => Ok(Some(CliCommand::Version)),
            "help" | "-h" | "--help" => Ok(Some(CliCommand::Help {
                command: rest.first().cloned(),
            })),
            "skill" | "skills" => Self::parse_skill_command(rest),
            "system" | "sys" => Self::parse_system_command(rest),
            "db" | "database" => Self::parse_db_command(rest),
            "backup" => Ok(Some(CliCommand::Backup {
                output: rest.first().map(PathBuf::from),
            })),
            "restore" => Ok(Some(CliCommand::Restore {
                input: rest.first()
                    .map(PathBuf::from)
                    .ok_or_else(|| crate::error::AppError::Config("Missing backup file path".to_string()))?,
            })),
            "logs" => Ok(Some(CliCommand::Logs {
                level: Self::get_flag_value(rest, "--level"),
                follow: rest.contains(&"--follow".to_string()) || rest.contains(&"-f".to_string()),
                lines: Self::get_flag_value(rest, "--lines")
                    .or(Self::get_flag_value(rest, "-n"))
                    .and_then(|s| s.parse().ok()),
            })),
            "user" | "users" => Self::parse_user_command(rest),
            "config" => Self::parse_config_command(rest),
            _ => Ok(None),
        }
    }

    fn parse_skill_command(rest: &[String]) -> Result<Option<CliCommand>> {
        if rest.is_empty() {
            return Ok(Some(CliCommand::SkillList { filter: None }));
        }

        match rest[0].as_str() {
            "list" | "ls" => Ok(Some(CliCommand::SkillList {
                filter: rest.get(1).cloned(),
            })),
            "install" | "add" => Ok(Some(CliCommand::SkillInstall {
                name: rest.get(1).cloned().unwrap_or_default(),
                version: rest.get(2).cloned(),
            })),
            "uninstall" | "remove" | "rm" => Ok(Some(CliCommand::SkillUninstall {
                name: rest.get(1).cloned().unwrap_or_default(),
            })),
            "execute" | "run" | "exec" => Ok(Some(CliCommand::SkillExecute {
                name: rest.get(1).cloned().unwrap_or_default(),
                params: rest.get(2).and_then(|s| serde_json::from_str(s).ok()),
            })),
            "update" | "upgrade" => Ok(Some(CliCommand::SkillUpdate {
                name: rest.get(1).cloned(),
            })),
            "search" | "find" => Ok(Some(CliCommand::SkillSearch {
                query: rest[1..].join(" "),
            })),
            _ => Ok(None),
        }
    }

    fn parse_system_command(rest: &[String]) -> Result<Option<CliCommand>> {
        if rest.is_empty() {
            return Ok(Some(CliCommand::SystemInfo));
        }

        match rest[0].as_str() {
            "info" => Ok(Some(CliCommand::SystemInfo)),
            _ => Ok(None),
        }
    }

    fn parse_db_command(rest: &[String]) -> Result<Option<CliCommand>> {
        if rest.is_empty() {
            return Ok(Some(CliCommand::DbStats));
        }

        match rest[0].as_str() {
            "stats" => Ok(Some(CliCommand::DbStats)),
            "migrate" => Ok(Some(CliCommand::DbMigrate)),
            _ => Ok(None),
        }
    }

    fn parse_user_command(rest: &[String]) -> Result<Option<CliCommand>> {
        if rest.is_empty() {
            return Ok(Some(CliCommand::UserList));
        }

        match rest[0].as_str() {
            "list" | "ls" => Ok(Some(CliCommand::UserList)),
            "create" | "add" => Ok(Some(CliCommand::UserCreate {
                name: rest.get(1).cloned().unwrap_or_default(),
                role: Self::get_flag_value(rest, "--role"),
            })),
            "delete" | "rm" => Ok(Some(CliCommand::UserDelete {
                id: rest.get(1).cloned().unwrap_or_default(),
            })),
            "role" => Ok(Some(CliCommand::UserRole {
                id: rest.get(1).cloned().unwrap_or_default(),
                role: rest.get(2).cloned().unwrap_or_default(),
            })),
            _ => Ok(None),
        }
    }

    fn parse_config_command(rest: &[String]) -> Result<Option<CliCommand>> {
        if rest.is_empty() {
            return Ok(Some(CliCommand::ConfigGet { key: None }));
        }

        match rest[0].as_str() {
            "get" => Ok(Some(CliCommand::ConfigGet {
                key: rest.get(1).cloned(),
            })),
            "set" => Ok(Some(CliCommand::ConfigSet {
                key: rest.get(1).cloned().unwrap_or_default(),
                value: rest.get(2).cloned().unwrap_or_default(),
            })),
            _ => Ok(None),
        }
    }

    fn get_flag_value(args: &[String], flag: &str) -> Option<String> {
        args.iter().position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
    }
}

/// Generate help text
pub fn generate_help() -> String {
    r#"Desktop Agent CLI - 桌面自动化助手

用法: desktop-agent <command> [options]

命令:
  start [config]           启动应用
  stop [--force]            停止应用
  status                    查看应用状态
  version, -v               显示版本信息
  help [command]            显示帮助信息

技能管理:
  skill list [filter]       列出已安装技能
  skill install <name> [v]  安装技能
  skill uninstall <name>    卸载技能
  skill execute <name> [p]  执行技能
  skill update [name]       更新技能
  skill search <query>      搜索技能

系统:
  system info               显示系统信息

数据库:
  db stats                  数据库统计
  db migrate                运行迁移

备份:
  backup [output]           创建备份
  restore <input>           恢复备份

日志:
  logs [--level lvl] [-f] [-n N]  查看日志

用户:
  user list                 列出用户
  user create <name> [--role r]   创建用户
  user delete <id>          删除用户
  user role <id> <role>     设置角色

配置:
  config get [key]          查看配置
  config set <key> <value>  设置配置
"#.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_help() {
        let args = vec!["help".to_string()];
        let cmd = CliParser::parse(&args).unwrap();
        assert!(matches!(cmd, Some(CliCommand::Help { command: None })));
    }

    #[test]
    fn test_parse_version() {
        let args = vec!["--version".to_string()];
        let cmd = CliParser::parse(&args).unwrap();
        assert!(matches!(cmd, Some(CliCommand::Version)));
    }

    #[test]
    fn test_parse_skill_list() {
        let args = vec!["skill".to_string(), "list".to_string()];
        let cmd = CliParser::parse(&args).unwrap();
        assert!(matches!(cmd, Some(CliCommand::SkillList { .. })));
    }

    #[test]
    fn test_parse_skill_install() {
        let args = vec!["skill".to_string(), "install".to_string(), "ocr".to_string()];
        let cmd = CliParser::parse(&args).unwrap();
        assert!(matches!(cmd, Some(CliCommand::SkillInstall { name, .. }) if name == "ocr"));
    }

    #[test]
    fn test_cli_response_format() {
        let resp = CliResponse::ok("Done");
        assert_eq!(resp.format_output(), "Done");
    }
}
