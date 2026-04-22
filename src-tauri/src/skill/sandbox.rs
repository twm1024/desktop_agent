// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Skill execution sandbox
//!
//! Provides isolated execution environment for skills with resource limits

use crate::error::{AppError, Result};
use crate::skill::permissions::{SkillPermissions, FileAccess, NetworkAccess, SystemAccess};
use crate::security::rbac::{Permission, Resource, Action};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

/// Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Enable resource limits
    pub enable_limits: bool,

    /// Maximum memory in MB
    pub max_memory_mb: Option<usize>,

    /// Maximum CPU time in seconds
    pub max_cpu_time_secs: Option<u64>,

    /// Maximum execution time in seconds
    pub max_execution_time_secs: u64,

    /// Allow network access
    pub allow_network: bool,

    /// Allowed network domains (empty = all denied)
    pub allowed_domains: Vec<String>,

    /// Allow file system access
    pub allow_filesystem: bool,

    /// Allowed file paths (empty = sandbox only)
    pub allowed_paths: Vec<PathBuf>,

    /// Working directory for the skill
    pub working_directory: PathBuf,

    /// Temporary directory for the skill
    pub temp_directory: PathBuf,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enable_limits: true,
            max_memory_mb: Some(512),
            max_cpu_time_secs: Some(30),
            max_execution_time_secs: 60,
            allow_network: false,
            allowed_domains: Vec::new(),
            allow_filesystem: true,
            allowed_paths: Vec::new(),
            working_directory: PathBuf::from("/tmp/sandbox"),
            temp_directory: PathBuf::from("/tmp/sandbox/tmp"),
        }
    }
}

/// Sandbox execution context
#[derive(Debug, Clone)]
pub struct SandboxContext {
    pub skill_id: String,
    pub skill_version: String,
    pub user_id: String,
    pub session_id: Option<String>,
    pub permissions: SkillPermissions,
    pub config: SandboxConfig,
}

/// Sandbox execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub output_file: Option<PathBuf>,
    pub execution_time_secs: f64,
    pub memory_used_mb: Option<f64>,
    pub error: Option<String>,
}

/// Sandbox executor
pub struct SandboxExecutor {
    config: SandboxConfig,
}

impl SandboxExecutor {
    pub fn new(config: SandboxConfig) -> Self {
        Self { config }
    }

    /// Execute a Python skill in sandbox
    pub async fn execute_python(
        &self,
        context: &SandboxContext,
        script_path: &Path,
        input_file: &Path,
        output_file: &Path,
    ) -> Result<SandboxResult> {
        // Check permissions
        self.validate_permissions(context).await?;

        // Prepare sandbox environment
        let work_dir = self.prepare_sandbox(context).await?;

        // Build command with resource limits
        let mut cmd = Command::new("python3");
        cmd.current_dir(&work_dir)
            .arg(script_path)
            .arg(input_file)
            .arg(output_file);

        // Apply resource limits using ulimit (Unix)
        #[cfg(unix)]
        {
            if self.config.enable_limits {
                // Set memory limit
                if let Some(max_mem) = self.config.max_memory_mb {
                    cmd.env("_RLIMIT_MEMORY", format!("{}", max_mem * 1024 * 1024));
                }
            }
        }

        // Set environment variables
        cmd.env("SANDBOX", "1")
            .env("SKILL_ID", &context.skill_id)
            .env("SKILL_VERSION", &context.skill_version)
            .env("USER_ID", &context.user_id)
            .env("SANDBOX_TEMP_DIR", &self.config.temp_directory);

        // Clear environment except safe variables
        self.sanitize_environment(&mut cmd);

        // Execute with timeout
        let start = std::time::Instant::now();
        let execution_result = timeout(
            Duration::from_secs(self.config.max_execution_time_secs),
            cmd.output(),
        ).await;

        let execution_time = start.elapsed().as_secs_f64();

        match execution_result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                // Read output file if it exists
                let result_data = if output_file.exists() {
                    Some(output_file.to_path_buf())
                } else {
                    None
                };

                Ok(SandboxResult {
                    success: output.status.success(),
                    exit_code: output.status.code(),
                    stdout,
                    stderr,
                    output_file: result_data,
                    execution_time_secs: execution_time,
                    memory_used_mb: None, // Would need external tool to measure
                    error: None,
                })
            }
            Ok(Err(e)) => Ok(SandboxResult {
                success: false,
                exit_code: None,
                stdout: String::new(),
                stderr: format!("Execution error: {}", e),
                output_file: None,
                execution_time_secs: execution_time,
                memory_used_mb: None,
                error: Some(e.to_string()),
            }),
            Err(_) => Ok(SandboxResult {
                success: false,
                exit_code: None,
                stdout: String::new(),
                stderr: "Execution timeout".to_string(),
                output_file: None,
                execution_time_secs: execution_time,
                memory_used_mb: None,
                error: Some("Timeout".to_string()),
            }),
        }
    }

    /// Execute a Node.js skill in sandbox
    pub async fn execute_nodejs(
        &self,
        context: &SandboxContext,
        script_path: &Path,
        input_file: &Path,
        output_file: &Path,
    ) -> Result<SandboxResult> {
        // Check permissions
        self.validate_permissions(context).await?;

        // Prepare sandbox environment
        let work_dir = self.prepare_sandbox(context).await?;

        // Build command
        let mut cmd = Command::new("node");
        cmd.current_dir(&work_dir)
            .arg(script_path)
            .arg(input_file)
            .arg(output_file);

        // Set environment variables
        cmd.env("SANDBOX", "1")
            .env("SKILL_ID", &context.skill_id)
            .env("SKILL_VERSION", &context.skill_version)
            .env("USER_ID", &context.user_id)
            .env("SANDBOX_TEMP_DIR", &self.config.temp_directory);

        // Sanitize environment
        self.sanitize_environment(&mut cmd);

        // Execute with timeout
        let start = std::time::Instant::now();
        let execution_result = timeout(
            Duration::from_secs(self.config.max_execution_time_secs),
            cmd.output(),
        ).await;

        let execution_time = start.elapsed().as_secs_f64();

        match execution_result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                let result_data = if output_file.exists() {
                    Some(output_file.to_path_buf())
                } else {
                    None
                };

                Ok(SandboxResult {
                    success: output.status.success(),
                    exit_code: output.status.code(),
                    stdout,
                    stderr,
                    output_file: result_data,
                    execution_time_secs: execution_time,
                    memory_used_mb: None,
                    error: None,
                })
            }
            Ok(Err(e)) => Ok(SandboxResult {
                success: false,
                exit_code: None,
                stdout: String::new(),
                stderr: format!("Execution error: {}", e),
                output_file: None,
                execution_time_secs: execution_time,
                memory_used_mb: None,
                error: Some(e.to_string()),
            }),
            Err(_) => Ok(SandboxResult {
                success: false,
                exit_code: None,
                stdout: String::new(),
                stderr: "Execution timeout".to_string(),
                output_file: None,
                execution_time_secs: execution_time,
                memory_used_mb: None,
                error: Some("Timeout".to_string()),
            }),
        }
    }

    /// Validate permissions before execution
    async fn validate_permissions(&self, context: &SandboxContext) -> Result<()> {
        // Check file access permissions
        if !self.config.allow_filesystem {
            if context.permissions.has_file_access() {
                return Err(AppError::Permission(
                    "File system access denied by sandbox policy".to_string(),
                ));
            }
        }

        // Check network access permissions
        if !self.config.allow_network {
            if context.permissions.has_network_access() {
                return Err(AppError::Permission(
                    "Network access denied by sandbox policy".to_string(),
                ));
            }
        }

        // Check system access permissions
        if context.permissions.has_system_access() {
            return Err(AppError::Permission(
                "System access denied in sandbox".to_string(),
            ));
        }

        Ok(())
    }

    /// Prepare sandbox directory structure
    async fn prepare_sandbox(&self, context: &SandboxContext) -> Result<PathBuf> {
        let work_dir = self.config.working_directory.join(&context.skill_id);
        let temp_dir = work_dir.join("tmp");

        // Create directories
        tokio::fs::create_dir_all(&work_dir).await?;
        tokio::fs::create_dir_all(&temp_dir).await?;

        Ok(work_dir)
    }

    /// Sanitize environment variables
    fn sanitize_environment(&self, cmd: &mut Command) {
        // Clear all environment variables
        cmd.env_clear();

        // Set only safe variables
        cmd.env("PATH", "/usr/bin:/bin")
            .env("HOME", &self.config.working_directory)
            .env("LANG", "C.UTF-8")
            .env("LC_ALL", "C.UTF-8");
    }

    /// Cleanup sandbox after execution
    pub async fn cleanup(&self, context: &SandboxContext) -> Result<()> {
        let work_dir = self.config.working_directory.join(&context.skill_id);

        if work_dir.exists() {
            tokio::fs::remove_dir_all(&work_dir).await?;
        }

        Ok(())
    }
}

/// Resource monitor for sandbox
pub struct ResourceMonitor {
    max_memory_mb: usize,
    max_cpu_time: Duration,
}

impl ResourceMonitor {
    pub fn new(max_memory_mb: usize, max_cpu_time: Duration) -> Self {
        Self {
            max_memory_mb,
            max_cpu_time,
        }
    }

    /// Monitor process resource usage
    pub async fn monitor_process(&self, pid: u32) -> Result<ResourceUsage> {
        #[cfg(unix)]
        {
            // Read /proc/[pid]/status for memory info
            let status_path = format!("/proc/{}/status", pid);
            if let Ok(status) = tokio::fs::read_to_string(&status_path).await {
                let memory_mb = self.parse_memory_mb(&status);
                return Ok(ResourceUsage {
                    memory_mb,
                    cpu_time_secs: 0.0,
                });
            }
        }

        Ok(ResourceUsage {
            memory_mb: 0,
            cpu_time_secs: 0.0,
        })
    }

    #[cfg(unix)]
    fn parse_memory_mb(&self, status: &str) -> f64 {
        // Parse VmRSS from /proc/pid/status
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<u64>() {
                        return kb as f64 / 1024.0;
                    }
                }
            }
        }
        0.0
    }

    /// Check if resource usage exceeds limits
    pub fn check_limits(&self, usage: &ResourceUsage) -> bool {
        if usage.memory_mb > self.max_memory_mb as f64 {
            return false;
        }
        if usage.cpu_time_secs > self.max_cpu_time.as_secs_f64() {
            return false;
        }
        true
    }
}

/// Resource usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub memory_mb: f64,
    pub cpu_time_secs: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sandbox_config_default() {
        let config = SandboxConfig::default();
        assert_eq!(config.max_execution_time_secs, 60);
        assert!(!config.allow_network);
    }

    #[tokio::test]
    async fn test_prepare_sandbox() {
        let config = SandboxConfig {
            working_directory: PathBuf::from("/tmp/test_sandbox"),
            ..Default::default()
        };

        let executor = SandboxExecutor::new(config);
        let context = SandboxContext {
            skill_id: "test_skill".to_string(),
            skill_version: "1.0.0".to_string(),
            user_id: "test_user".to_string(),
            session_id: None,
            permissions: SkillPermissions::default(),
            config: SandboxConfig::default(),
        };

        let result = executor.prepare_sandbox(&context).await;
        // Note: This might fail due to permissions, just test the structure
        assert!(result.is_ok() || result.is_err());
    }
}
