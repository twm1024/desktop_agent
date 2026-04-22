// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

use crate::database::Database;
use crate::error::{AppError, Result};
use crate::skill::manifest::SkillManifest;
use crate::skill::permissions::SkillPermissions;
use crate::skill::types::{SkillInfo, SkillResult};
use crate::services::ServiceContainer;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tracing::{info, warn};

pub struct SkillLoader {
    skills_dir: PathBuf,
    db: Arc<Database>,
    services: Arc<ServiceContainer>,
    loaded_skills: HashMap<String, LoadedSkill>,
}

struct LoadedSkill {
    manifest: SkillManifest,
    path: PathBuf,
    permissions: SkillPermissions,
    checksum: String,
}

impl SkillLoader {
    pub fn new(skills_dir: PathBuf, db: Arc<Database>, services: Arc<ServiceContainer>) -> Self {
        Self {
            skills_dir,
            db,
            services,
            loaded_skills: HashMap::new(),
        }
    }

    /// Initialize and load all skills
    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing skill loader from {:?}", self.skills_dir);

        // Ensure skills directory exists
        if !self.skills_dir.exists() {
            fs::create_dir_all(&self.skills_dir).await?;
            info!("Created skills directory: {:?}", self.skills_dir);
        }

        // Load skills from disk
        self.load_skills_from_disk().await?;

        // Sync with database
        self.sync_with_database().await?;

        info!("Skill loader initialized, loaded {} skills", self.loaded_skills.len());
        Ok(())
    }

    /// Load all skills from disk
    async fn load_skills_from_disk(&mut self) -> Result<()> {
        let mut entries = fs::read_dir(&self.skills_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            // Check if it has a skill.yaml
            let manifest_path = path.join("skill.yaml");
            if !manifest_path.exists() {
                continue;
            }

            // Load manifest
            match SkillManifest::load(&path) {
                Ok(manifest) => {
                    let skill_id = manifest.get_id();

                    // Load permissions
                    let permissions = SkillPermissions::from_decl(&manifest.permissions)?;

                    // Calculate checksum
                    let checksum = self.calculate_checksum(&path).await?;

                    // Store loaded skill
                    self.loaded_skills.insert(
                        skill_id.clone(),
                        LoadedSkill {
                            manifest,
                            path: path.clone(),
                            permissions,
                            checksum,
                        },
                    );

                    info!("Loaded skill: {}", skill_id);
                }
                Err(e) => {
                    warn!("Failed to load skill from {:?}: {}", path, e);
                }
            }
        }

        Ok(())
    }

    /// Sync loaded skills with database
    async fn sync_with_database(&self) -> Result<()> {
        for (skill_id, loaded) in &self.loaded_skills {
            // Check if skill exists in database
            let exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM skills WHERE id = ?)"
            )
            .bind(skill_id)
            .fetch_one(self.db.pool())
            .await?;

            if !exists {
                // Insert new skill
                let now = chrono::Utc::now().timestamp();
                let manifest_json = serde_json::to_string(&loaded.manifest)
                    .map_err(|e| AppError::serialization(e))?;

                sqlx::query(
                    r#"
                    INSERT INTO skills (
                        id, name, version, description, author, tags, manifest,
                        enabled, installed_at, updated_at, source, checksum
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(skill_id)
                .bind(&loaded.manifest.name)
                .bind(&loaded.manifest.version)
                .bind(&loaded.manifest.description)
                .bind(&loaded.manifest.author)
                .bind(serde_json::to_string(&loaded.manifest.tags).unwrap())
                .bind(&manifest_json)
                .bind(true)
                .bind(now)
                .bind(now)
                .bind("local")
                .bind(&loaded.checksum)
                .execute(self.db.pool())
                .await?;

                info!("Registered skill in database: {}", skill_id);
            }
        }

        Ok(())
    }

    /// Get loaded skill info
    pub fn get_skill_info(&self, skill_id: &str) -> Option<SkillInfo> {
        self.loaded_skills.get(skill_id).map(|loaded| SkillInfo {
            id: skill_id.to_string(),
            name: loaded.manifest.name.clone(),
            version: loaded.manifest.version.clone(),
            description: loaded.manifest.description.clone(),
            author: loaded.manifest.author.clone(),
            tags: loaded.manifest.tags.clone(),
            enabled: true, // TODO: Load from database
            installed_at: 0, // TODO: Load from database
            last_executed_at: None, // TODO: Load from database
            execution_count: 0, // TODO: Load from database
            icon: Some(loaded.manifest.icon.value.clone()),
        })
    }

    /// List all loaded skills
    pub fn list_skills(&self) -> Vec<SkillInfo> {
        self.loaded_skills
            .keys()
            .filter_map(|id| self.get_skill_info(id))
            .collect()
    }

    /// Get skill manifest
    pub fn get_manifest(&self, skill_id: &str) -> Option<&SkillManifest> {
        self.loaded_skills.get(skill_id).map(|s| &s.manifest)
    }

    /// Get skill permissions
    pub fn get_permissions(&self, skill_id: &str) -> Option<&SkillPermissions> {
        self.loaded_skills.get(skill_id).map(|s| &s.permissions)
    }

    /// Reload a skill
    pub async fn reload_skill(&mut self, skill_id: &str) -> Result<()> {
        if let Some(loaded) = self.loaded_skills.get(skill_id) {
            let path = loaded.path.clone();

            // Reload manifest
            let manifest = SkillManifest::load(&path)?;
            let permissions = SkillPermissions::from_decl(&manifest.permissions)?;
            let checksum = self.calculate_checksum(&path).await?;

            // Update loaded skill
            self.loaded_skills.insert(
                skill_id.to_string(),
                LoadedSkill {
                    manifest,
                    path,
                    permissions,
                    checksum,
                },
            );

            info!("Reloaded skill: {}", skill_id);
            Ok(())
        } else {
            Err(AppError::not_found(format!("Skill not found: {}", skill_id)))
        }
    }

    /// Calculate SHA256 checksum of a directory
    async fn calculate_checksum(&self, path: &PathBuf) -> Result<String> {
        let mut hasher = Sha256::new();

        // Read skill.yaml
        let manifest_path = path.join("skill.yaml");
        if manifest_path.exists() {
            let content = fs::read_to_string(&manifest_path).await?;
            hasher.update(content.as_bytes());
        }

        // Read main file
        if let Some(loaded) = self.loaded_skills.values().next() {
            let main_path = path.join(&loaded.manifest.main);
            if main_path.exists() {
                let content = fs::read(&main_path).await?;
                hasher.update(&content);
            }
        }

        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }
}
