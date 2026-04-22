// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Skill manifest file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub main: String,
    #[serde(default)]
    pub dependencies: SkillDependencies,
    pub permissions: SkillPermissionDecl,
    #[serde(default)]
    pub parameters: Vec<SkillParameter>,
    #[serde(default)]
    pub outputs: Vec<SkillOutput>,
    #[serde(default)]
    pub icon: SkillIcon,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDependencies {
    #[serde(default)]
    pub python: Option<String>,
    #[serde(default)]
    pub pip: Vec<String>,
    #[serde(default)]
    pub node: Option<String>,
    #[serde(default)]
    pub npm: Vec<String>,
}

impl Default for SkillDependencies {
    fn default() -> Self {
        Self {
            python: None,
            pip: Vec::new(),
            node: None,
            npm: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPermissionDecl {
    #[serde(default)]
    pub file: Vec<FilePermissionDecl>,
    #[serde(default)]
    pub network: Vec<NetworkPermissionDecl>,
    #[serde(default)]
    pub system: Vec<SystemPermissionDecl>,
}

impl Default for SkillPermissionDecl {
    fn default() -> Self {
        Self {
            file: Vec::new(),
            network: Vec::new(),
            system: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePermissionDecl {
    pub path: String,
    pub access: String, // read/write/readwrite
    #[serde(default)]
    pub recursive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPermissionDecl {
    pub domain: String,
    #[serde(default)]
    pub allowed: bool,
    #[serde(default)]
    pub ports: Vec<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPermissionDecl {
    pub action: String, // execute_command/screenshot/clipboard
    #[serde(default)]
    pub allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillParameter {
    pub name: String,
    pub r#type: String,
    pub label: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub options: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOutput {
    pub name: String,
    pub r#type: String,
    pub description: String,
    #[serde(default)]
    pub items: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillIcon {
    #[serde(rename = "type")]
    pub icon_type: String, // emoji/url
    pub value: String,
}

impl Default for SkillIcon {
    fn default() -> Self {
        Self {
            icon_type: "emoji".to_string(),
            value: "🔧".to_string(),
        }
    }
}

impl SkillManifest {
    /// Load manifest from a skill directory
    pub fn load(skill_dir: &Path) -> Result<Self> {
        let manifest_path = skill_dir.join("skill.yaml");

        if !manifest_path.exists() {
            return Err(AppError::not_found(format!(
                "Manifest not found: {:?}",
                manifest_path
            )));
        }

        let content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| AppError::io(e))?;

        let manifest: Self = serde_yaml::from_str(&content)
            .map_err(|e| AppError::config(format!("Failed to parse manifest: {}", e)))?;

        // Validate manifest
        manifest.validate()?;

        Ok(manifest)
    }

    /// Validate manifest
    fn validate(&self) -> Result<()> {
        // Check required fields
        if self.name.is_empty() {
            return Err(AppError::config("Skill name is required"));
        }

        if self.version.is_empty() {
            return Err(AppError::config("Skill version is required"));
        }

        if self.main.is_empty() {
            return Err(AppError::config("Main entry point is required"));
        }

        // Validate version format (semver)
        if !semver::Version::parse(&self.version).is_ok() {
            return Err(AppError::config(format!(
                "Invalid version format: {}",
                self.version
            )));
        }

        Ok(())
    }

    /// Get the skill ID (format: author.name)
    pub fn get_id(&self) -> String {
        format!("{}.{}", self.author, self.name)
            .to_lowercase()
            .replace(" ", "-")
    }

    /// Check if skill requires Python
    pub fn requires_python(&self) -> bool {
        self.dependencies.python.is_some() || !self.dependencies.pip.is_empty()
    }

    /// Check if skill requires Node.js
    pub fn requires_node(&self) -> bool {
        self.dependencies.node.is_some() || !self.dependencies.npm.is_empty()
    }
}
