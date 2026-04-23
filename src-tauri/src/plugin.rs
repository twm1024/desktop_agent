// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Plugin system
//!
//! Provides a plugin architecture for extending application functionality

#![allow(dead_code)]
use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Plugin manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub main: String,
    pub permissions: Vec<String>,
    pub dependencies: Vec<String>,
    pub config_schema: Option<serde_json::Value>,
    pub hooks: Vec<HookDefinition>,
}

/// Hook definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDefinition {
    pub event: String,
    pub handler: String,
    pub priority: Option<i32>,
}

/// Hook events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HookEvent {
    OnStartup,
    OnShutdown,
    OnMessageReceived,
    OnMessageSent,
    OnSkillExecuted,
    OnFileOperation,
    OnConfigChanged,
    OnUserLogin,
    OnUserLogout,
    OnCustom(&'static str),
}

/// Plugin state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginState {
    Loaded,
    Initialized,
    Running,
    Stopped,
    Error,
}

/// Loaded plugin instance
#[derive(Debug)]
pub struct PluginInstance {
    pub manifest: PluginManifest,
    pub state: PluginState,
    pub config: serde_json::Value,
    pub path: PathBuf,
    pub loaded_at: i64,
}

/// Plugin manager
pub struct PluginManager {
    plugins: HashMap<String, PluginInstance>,
    plugins_dir: PathBuf,
    hooks: HashMap<String, Vec<HookEntry>>,
}

struct HookEntry {
    plugin_id: String,
    handler: String,
    priority: i32,
}

impl PluginManager {
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self {
            plugins: HashMap::new(),
            plugins_dir,
            hooks: HashMap::new(),
        }
    }

    /// Load all plugins from the plugins directory
    pub async fn load_all(&mut self) -> Result<Vec<String>> {
        if !self.plugins_dir.exists() {
            tokio::fs::create_dir_all(&self.plugins_dir).await?;
            return Ok(Vec::new());
        }

        let mut loaded = Vec::new();

        let mut entries = tokio::fs::read_dir(&self.plugins_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                let manifest_path = path.join("plugin.yaml");
                if manifest_path.exists() {
                    match self.load_plugin(&path).await {
                        Ok(id) => {
                            info!("Loaded plugin: {}", id);
                            loaded.push(id);
                        }
                        Err(e) => {
                            warn!("Failed to load plugin from {:?}: {}", path, e);
                        }
                    }
                }
            }
        }

        Ok(loaded)
    }

    /// Load a single plugin
    pub async fn load_plugin(&mut self, plugin_dir: &Path) -> Result<String> {
        let manifest_path = plugin_dir.join("plugin.yaml");
        let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
        let manifest: PluginManifest = serde_yaml::from_str(&manifest_content)?;

        let plugin_id = manifest.id.clone();

        // Register hooks
        for hook in &manifest.hooks {
            let entry = HookEntry {
                plugin_id: plugin_id.clone(),
                handler: hook.handler.clone(),
                priority: hook.priority.unwrap_or(0),
            };

            self.hooks
                .entry(hook.event.clone())
                .or_default()
                .push(entry);
        }

        let instance = PluginInstance {
            manifest,
            state: PluginState::Loaded,
            config: serde_json::Value::Null,
            path: plugin_dir.to_path_buf(),
            loaded_at: chrono::Utc::now().timestamp(),
        };

        self.plugins.insert(plugin_id.clone(), instance);
        Ok(plugin_id)
    }

    /// Unload a plugin
    pub async fn unload_plugin(&mut self, plugin_id: &str) -> Result<()> {
        if let Some(mut instance) = self.plugins.remove(plugin_id) {
            instance.state = PluginState::Stopped;

            // Remove hooks
            for entries in self.hooks.values_mut() {
                entries.retain(|e| e.plugin_id != plugin_id);
            }

            info!("Plugin {} unloaded", plugin_id);
        }
        Ok(())
    }

    /// Initialize a loaded plugin
    pub async fn initialize_plugin(&mut self, plugin_id: &str) -> Result<()> {
        let instance = self.plugins.get_mut(plugin_id)
            .ok_or_else(|| AppError::Config(format!("Plugin {} not found", plugin_id)))?;

        // Run plugin initialization script
        let init_script = instance.path.join(&instance.manifest.main);
        if init_script.exists() {
            // In production, this would execute in a sandboxed environment
            info!("Initializing plugin {} from {:?}", plugin_id, init_script);
        }

        instance.state = PluginState::Initialized;
        Ok(())
    }

    /// Start a plugin
    pub async fn start_plugin(&mut self, plugin_id: &str) -> Result<()> {
        let instance = self.plugins.get_mut(plugin_id)
            .ok_or_else(|| AppError::Config(format!("Plugin {} not found", plugin_id)))?;

        instance.state = PluginState::Running;
        info!("Plugin {} started", plugin_id);
        Ok(())
    }

    /// Stop a plugin
    pub async fn stop_plugin(&mut self, plugin_id: &str) -> Result<()> {
        let instance = self.plugins.get_mut(plugin_id)
            .ok_or_else(|| AppError::Config(format!("Plugin {} not found", plugin_id)))?;

        instance.state = PluginState::Stopped;
        info!("Plugin {} stopped", plugin_id);
        Ok(())
    }

    /// Get plugin by ID
    pub fn get_plugin(&self, plugin_id: &str) -> Option<&PluginInstance> {
        self.plugins.get(plugin_id)
    }

    /// List all plugins
    pub fn list_plugins(&self) -> Vec<(&String, &PluginInstance)> {
        self.plugins.iter().collect()
    }

    /// List plugins by state
    pub fn list_by_state(&self, state: PluginState) -> Vec<&PluginInstance> {
        self.plugins.values().filter(|p| p.state == state).collect()
    }

    /// Update plugin configuration
    pub async fn update_config(&mut self, plugin_id: &str, config: serde_json::Value) -> Result<()> {
        let instance = self.plugins.get_mut(plugin_id)
            .ok_or_else(|| AppError::Config(format!("Plugin {} not found", plugin_id)))?;

        instance.config = config;
        Ok(())
    }

    /// Get hooks for a specific event
    pub fn get_hooks(&self, event: &str) -> Vec<(&str, &str)> {
        self.hooks.get(event)
            .map(|entries| {
                let mut sorted: Vec<_> = entries.iter()
                    .map(|e| (e.plugin_id.as_str(), e.handler.as_str()))
                    .collect();
                sorted.sort_by(|a, b| b.1.cmp(a.1)); // Sort by handler name (stable)
                sorted
            })
            .unwrap_or_default()
    }

    /// Get plugin count
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manager_creation() {
        let manager = PluginManager::new(PathBuf::from("/tmp/plugins"));
        assert_eq!(manager.plugin_count(), 0);
    }
}
