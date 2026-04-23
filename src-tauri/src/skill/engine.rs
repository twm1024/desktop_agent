// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

#![allow(dead_code)]
use crate::database::Database;
use crate::error::{AppError, Result};
use crate::services::ServiceContainer;
use crate::skill::executor::SkillExecutor;
use crate::skill::loader::SkillLoader;
use crate::skill::types::{SkillContext, SkillInfo, SkillParameters, SkillResult, SkillProgress};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::info;

pub struct SkillEngine {
    skills_dir: PathBuf,
    db: Arc<Database>,
    services: Arc<ServiceContainer>,
    loader: Arc<RwLock<SkillLoader>>,
    executor: SkillExecutor,
    progress_senders: Arc<RwLock<Vec<mpsc::Sender<SkillProgress>>>>,
}

impl SkillEngine {
    pub fn new(skills_dir: PathBuf, services: Arc<ServiceContainer>) -> Result<Self> {
        let db = services.db.clone();
        let executor = SkillExecutor::new(services.clone(), skills_dir.clone());

        Ok(Self {
            skills_dir: skills_dir.clone(),
            db: db.clone(),
            services: services.clone(),
            loader: Arc::new(RwLock::new(SkillLoader::new(
                skills_dir,
                db,
                services,
            ))),
            executor,
            progress_senders: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Initialize the skill engine
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing skill engine");
        self.loader.write().await.initialize().await?;
        Ok(())
    }

    /// List all skills
    pub async fn list_skills(&self) -> Result<Vec<SkillInfo>> {
        let loader = self.loader.read().await;
        Ok(loader.list_skills())
    }

    /// Get skill info
    pub async fn get_skill(&self, skill_id: &str) -> Result<SkillInfo> {
        let loader = self.loader.read().await;
        loader
            .get_skill_info(skill_id)
            .ok_or_else(|| AppError::not_found(format!("Skill not found: {}", skill_id)))
    }

    /// Execute a skill
    pub async fn execute_skill(
        &self,
        skill_id: &str,
        params: SkillParameters,
        context: SkillContext,
    ) -> Result<SkillResult> {
        let (manifest, permissions) = {
            let loader = self.loader.read().await;

            let manifest = loader
                .get_manifest(skill_id)
                .ok_or_else(|| AppError::not_found(format!("Skill not found: {}", skill_id)))?
                .clone();

            let permissions = loader
                .get_permissions(skill_id)
                .ok_or_else(|| AppError::not_found(format!("Skill permissions not found: {}", skill_id)))?
                .clone();

            (manifest, permissions)
        };

        // Update execution count
        self.update_execution_count(skill_id).await;

        // Execute skill
        let result = self
            .executor
            .execute(&manifest, &permissions, params, context)
            .await;

        // Update last executed time
        self.update_last_executed(skill_id).await;

        Ok(result)
    }

    /// Subscribe to skill progress events
    pub async fn subscribe_progress(&self) -> mpsc::Receiver<SkillProgress> {
        let (tx, rx) = mpsc::channel(100);
        self.progress_senders.write().await.push(tx);
        rx
    }

    /// Reload a skill
    pub async fn reload_skill(&self, skill_id: &str) -> Result<()> {
        self.loader.write().await.reload_skill(skill_id).await
    }

    /// Enable a skill
    pub async fn enable_skill(&self, skill_id: &str) -> Result<()> {
        sqlx::query("UPDATE skills SET enabled = 1 WHERE id = ?")
            .bind(skill_id)
            .execute(self.db.pool())
            .await?;
        Ok(())
    }

    /// Disable a skill
    pub async fn disable_skill(&self, skill_id: &str) -> Result<()> {
        sqlx::query("UPDATE skills SET enabled = 0 WHERE id = ?")
            .bind(skill_id)
            .execute(self.db.pool())
            .await?;
        Ok(())
    }

    async fn update_execution_count(&self, skill_id: &str) {
        sqlx::query("UPDATE skills SET execution_count = execution_count + 1 WHERE id = ?")
            .bind(skill_id)
            .execute(self.db.pool())
            .await
            .ok();
    }

    async fn update_last_executed(&self, skill_id: &str) {
        let now = chrono::Utc::now().timestamp();
        sqlx::query("UPDATE skills SET last_executed_at = ? WHERE id = ?")
            .bind(now)
            .bind(skill_id)
            .execute(self.db.pool())
            .await
            .ok();
    }
}
