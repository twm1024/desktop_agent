// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

#![allow(dead_code)]
use crate::error::Result;
use crate::skill::manifest::SkillManifest;
use crate::skill::permissions::SkillPermissions;
use crate::skill::types::{SkillContext, SkillParameters, SkillResult, SkillProgress};
use crate::services::ServiceContainer;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::{error, info};

pub struct SkillExecutor {
    services: Arc<ServiceContainer>,
    skills_dir: PathBuf,
}

impl SkillExecutor {
    pub fn new(services: Arc<ServiceContainer>, skills_dir: PathBuf) -> Self {
        Self {
            services,
            skills_dir,
        }
    }

    /// Execute a skill
    pub async fn execute(
        &self,
        manifest: &SkillManifest,
        _permissions: &SkillPermissions,
        params: SkillParameters,
        context: SkillContext,
    ) -> SkillResult {
        let start_time = std::time::Instant::now();
        let skill_id = manifest.get_id();

        info!("Executing skill: {}", skill_id);

        // Determine runtime based on main file extension
        let runtime = self.determine_runtime(&manifest.main);

        // Build command
        let mut cmd = match runtime {
            RuntimeType::Python => match self.build_python_command(manifest) {
                Ok(cmd) => cmd,
                Err(e) => return SkillResult::failure(format!("Failed to build python command: {}", e)),
            },
            RuntimeType::Node => match self.build_node_command(manifest) {
                Ok(cmd) => cmd,
                Err(e) => return SkillResult::failure(format!("Failed to build node command: {}", e)),
            },
            RuntimeType::Unknown => {
                return SkillResult::failure(format!("Unknown runtime for {}", manifest.main))
            }
        };

        // Prepare input/output files
        let (input_file, output_file) = match self.prepare_io_files(&skill_id, &params, &context).await {
            Ok(files) => files,
            Err(e) => {
                error!("Failed to prepare IO files: {}", e);
                return SkillResult::failure(format!("Failed to prepare IO files: {}", e));
            }
        };

        // Set environment variables
        cmd.env("DESKTOP_AGENT_INPUT", &input_file)
            .env("DESKTOP_AGENT_OUTPUT", &output_file)
            .env("DESKTOP_AGENT_SKILL_ID", &skill_id)
            .env("DESKTOP_AGENT_VERSION", env!("CARGO_PKG_VERSION"))
            .env("DESKTOP_AGENT_USER_ID", &context.user_id)
            .env("DESKTOP_AGENT_CHAT_ID", &context.chat_id)
            .env("DESKTOP_AGENT_PLATFORM", &context.platform)
            .env("DESKTOP_AGENT_SESSION_ID", &context.session_id);

        // Set working directory
        let skill_path = self.skills_dir.join(&skill_id);
        cmd.current_dir(&skill_path);

        // Setup stdout/stderr capture
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Spawn process
        let child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                error!("Failed to spawn skill process: {}", e);
                return SkillResult::failure(format!("Failed to execute skill: {}", e));
            }
        };

        // Wait for completion
        match child.wait_with_output().await {
            Ok(output) => {
                let execution_time_ms = start_time.elapsed().as_millis() as u64;

                if output.status.success() {
                    // Read output
                    match self.read_output(&output_file).await {
                        Ok(result) => {
                            let mut skill_result = result;
                            skill_result.execution_time_ms = execution_time_ms;
                            info!("Skill {} executed successfully in {}ms", skill_id, execution_time_ms);
                            skill_result
                        }
                        Err(e) => {
                            error!("Failed to read skill output: {}", e);
                            SkillResult::failure(format!("Failed to read output: {}", e))
                        }
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    error!("Skill {} failed: {}", skill_id, stderr);
                    SkillResult::failure(format!("Execution failed: {}", stderr))
                }
            }
            Err(e) => {
                error!("Failed to wait for skill process: {}", e);
                SkillResult::failure(format!("Execution error: {}", e))
            }
        }
    }

    /// Execute skill with progress callback
    pub async fn execute_with_progress(
        &self,
        manifest: &SkillManifest,
        permissions: &SkillPermissions,
        params: SkillParameters,
        context: SkillContext,
        _progress_tx: mpsc::Sender<SkillProgress>,
    ) -> SkillResult {
        // TODO: Implement progress tracking
        // For now, just use basic execute
        self.execute(manifest, permissions, params, context).await
    }

    fn determine_runtime(&self, main: &str) -> RuntimeType {
        if main.ends_with(".py") {
            RuntimeType::Python
        } else if main.ends_with(".js") || main.ends_with(".ts") {
            RuntimeType::Node
        } else {
            RuntimeType::Unknown
        }
    }

    fn build_python_command(&self, manifest: &SkillManifest) -> Result<Command> {
        let mut cmd = Command::new("python3");
        cmd.arg(&manifest.main);
        Ok(cmd)
    }

    fn build_node_command(&self, manifest: &SkillManifest) -> Result<Command> {
        let mut cmd = Command::new("node");
        cmd.arg(&manifest.main);
        Ok(cmd)
    }

    async fn prepare_io_files(
        &self,
        skill_id: &str,
        params: &SkillParameters,
        context: &SkillContext,
    ) -> Result<(String, String)> {
        // Create temp directory
        let temp_dir = std::env::temp_dir();
        let session_dir = temp_dir.join("desktop-agent").join(&context.session_id);
        tokio::fs::create_dir_all(&session_dir).await?;

        // Prepare input
        let input = serde_json::json!({
            "skill_id": skill_id,
            "parameters": params.values,
            "context": context,
        });

        let input_file = session_dir.join("input.json");
        tokio::fs::write(&input_file, serde_json::to_vec(&input)?).await?;

        // Output file path
        let output_file = session_dir.join("output.json");

        Ok((
            input_file.to_string_lossy().to_string(),
            output_file.to_string_lossy().to_string(),
        ))
    }

    async fn read_output(&self, output_file: &str) -> Result<SkillResult> {
        let content = tokio::fs::read_to_string(output_file).await?;
        let result: SkillResult = serde_json::from_str(&content)?;
        Ok(result)
    }
}

enum RuntimeType {
    Python,
    Node,
    Unknown,
}
