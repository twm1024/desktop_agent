// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

use crate::database::Database;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;

/// User record in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecord {
    pub id: String,
    pub platform: String,
    pub platform_user_id: String,
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub role: String,
    pub permissions: Option<String>, // JSON
    pub created_at: i64,
    pub last_active_at: i64,
    pub is_blocked: bool,
    pub metadata: Option<String>, // JSON
    pub daily_quota: Option<i64>,
    pub quota_reset_at: Option<i64>,
    pub api_key: Option<String>,
}

/// Repository for user operations
pub struct UserRepository {
    pool: SqlitePool,
}

impl UserRepository {
    pub fn new(db: &Arc<Database>) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    /// Insert a new user
    pub async fn insert(&self, user: &UserRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO users (
                id, platform, platform_user_id, name, avatar, role,
                permissions, created_at, last_active_at, is_blocked,
                metadata, daily_quota, quota_reset_at, api_key
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&user.id)
        .bind(&user.platform)
        .bind(&user.platform_user_id)
        .bind(&user.name)
        .bind(&user.avatar)
        .bind(&user.role)
        .bind(&user.permissions)
        .bind(user.created_at)
        .bind(user.last_active_at)
        .bind(user.is_blocked)
        .bind(&user.metadata)
        .bind(user.daily_quota)
        .bind(user.quota_reset_at)
        .bind(&user.api_key)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get a user by ID
    pub async fn get_by_id(&self, id: &str) -> Result<Option<UserRecord>> {
        let user = sqlx::query_as::<_, UserRecord>(
            "SELECT * FROM users WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    /// Get a user by platform and platform user ID
    pub async fn get_by_platform_user_id(
        &self,
        platform: &str,
        platform_user_id: &str,
    ) -> Result<Option<UserRecord>> {
        let user = sqlx::query_as::<_, UserRecord>(
            "SELECT * FROM users WHERE platform = ? AND platform_user_id = ?"
        )
        .bind(platform)
        .bind(platform_user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    /// Get a user by API key
    pub async fn get_by_api_key(&self, api_key: &str) -> Result<Option<UserRecord>> {
        let user = sqlx::query_as::<_, UserRecord>(
            "SELECT * FROM users WHERE api_key = ?"
        )
        .bind(api_key)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    /// Update a user
    pub async fn update(&self, user: &UserRecord) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE users SET
                name = ?, avatar = ?, role = ?, permissions = ?,
                last_active_at = ?, is_blocked = ?, metadata = ?,
                daily_quota = ?, quota_reset_at = ?, api_key = ?
            WHERE id = ?
            "#,
        )
        .bind(&user.name)
        .bind(&user.avatar)
        .bind(&user.role)
        .bind(&user.permissions)
        .bind(user.last_active_at)
        .bind(user.is_blocked)
        .bind(&user.metadata)
        .bind(user.daily_quota)
        .bind(user.quota_reset_at)
        .bind(&user.api_key)
        .bind(&user.id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Update last active timestamp
    pub async fn update_last_active(&self, id: &str, timestamp: i64) -> Result<()> {
        sqlx::query("UPDATE users SET last_active_at = ? WHERE id = ?")
            .bind(timestamp)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Set user blocked status
    pub async fn set_blocked(&self, id: &str, blocked: bool) -> Result<()> {
        sqlx::query("UPDATE users SET is_blocked = ? WHERE id = ?")
            .bind(blocked)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// List all users
    pub async fn list_all(&self) -> Result<Vec<UserRecord>> {
        let users = sqlx::query_as::<_, UserRecord>(
            "SELECT * FROM users ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(users)
    }

    /// List users by role
    pub async fn list_by_role(&self, role: &str) -> Result<Vec<UserRecord>> {
        let users = sqlx::query_as::<_, UserRecord>(
            "SELECT * FROM users WHERE role = ? ORDER BY created_at DESC"
        )
        .bind(role)
        .fetch_all(&self.pool)
        .await?;
        Ok(users)
    }

    /// Delete a user
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
