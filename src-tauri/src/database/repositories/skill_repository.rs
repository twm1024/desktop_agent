// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

use crate::database::Database;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;

/// Skill record in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRecord {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub tags: Option<String>, // JSON array as string
    pub manifest: String, // JSON
    pub enabled: bool,
    pub installed_at: i64,
    pub updated_at: i64,
    pub last_executed_at: Option<i64>,
    pub execution_count: i64,
    pub source: Option<String>,
    pub checksum: Option<String>,
    pub metadata: Option<String>, // JSON
}

/// Repository for skill operations
pub struct SkillRepository {
    pool: SqlitePool,
}

impl SkillRepository {
    pub fn new(db: &Arc<Database>) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    /// Insert a new skill
    pub async fn insert(&self, skill: &SkillRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO skills (
                id, name, version, description, author, tags, manifest,
                enabled, installed_at, updated_at, last_executed_at,
                execution_count, source, checksum, metadata
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&skill.id)
        .bind(&skill.name)
        .bind(&skill.version)
        .bind(&skill.description)
        .bind(&skill.author)
        .bind(&skill.tags)
        .bind(&skill.manifest)
        .bind(skill.enabled)
        .bind(skill.installed_at)
        .bind(skill.updated_at)
        .bind(skill.last_executed_at)
        .bind(skill.execution_count)
        .bind(&skill.source)
        .bind(&skill.checksum)
        .bind(&skill.metadata)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Update an existing skill
    pub async fn update(&self, skill: &SkillRecord) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE skills SET
                name = ?, version = ?, description = ?, author = ?, tags = ?,
                manifest = ?, enabled = ?, updated_at = ?, checksum = ?, metadata = ?
            WHERE id = ?
            "#,
        )
        .bind(&skill.name)
        .bind(&skill.version)
        .bind(&skill.description)
        .bind(&skill.author)
        .bind(&skill.tags)
        .bind(&skill.manifest)
        .bind(skill.enabled)
        .bind(skill.updated_at)
        .bind(&skill.checksum)
        .bind(&skill.metadata)
        .bind(&skill.id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get a skill by ID
    pub async fn get_by_id(&self, id: &str) -> Result<Option<SkillRecord>> {
        let skill = sqlx::query_as::<_, SkillRecord>(
            "SELECT * FROM skills WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(skill)
    }

    /// Get a skill by name
    pub async fn get_by_name(&self, name: &str) -> Result<Option<SkillRecord>> {
        let skill = sqlx::query_as::<_, SkillRecord>(
            "SELECT * FROM skills WHERE name = ?"
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        Ok(skill)
    }

    /// List all skills
    pub async fn list_all(&self) -> Result<Vec<SkillRecord>> {
        let skills = sqlx::query_as::<_, SkillRecord>(
            "SELECT * FROM skills ORDER BY name"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(skills)
    }

    /// List enabled skills only
    pub async fn list_enabled(&self) -> Result<Vec<SkillRecord>> {
        let skills = sqlx::query_as::<_, SkillRecord>(
            "SELECT * FROM skills WHERE enabled = 1 ORDER BY name"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(skills)
    }

    /// Delete a skill
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM skills WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Update skill enabled status
    pub async fn set_enabled(&self, id: &str, enabled: bool) -> Result<()> {
        sqlx::query("UPDATE skills SET enabled = ? WHERE id = ?")
            .bind(enabled)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Update last execution time
    pub async fn update_last_execution(&self, id: &str, timestamp: i64) -> Result<()> {
        sqlx::query(
            "UPDATE skills SET last_executed_at = ?, execution_count = execution_count + 1 WHERE id = ?"
        )
        .bind(timestamp)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Search skills by name or description
    pub async fn search(&self, query: &str) -> Result<Vec<SkillRecord>> {
        let pattern = format!("%{}%", query);
        let skills = sqlx::query_as::<_, SkillRecord>(
            "SELECT * FROM skills WHERE name LIKE ? OR description LIKE ?"
        )
        .bind(&pattern)
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await?;
        Ok(skills)
    }
}
