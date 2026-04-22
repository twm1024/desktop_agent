// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

use crate::database::Database;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;

/// Session record in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub user_id: String,
    pub chat_id: String,
    pub platform: String,
    pub current_intent: Option<String>,
    pub slots: Option<String>, // JSON
    pub messages: Option<String>, // JSON array
    pub created_at: i64,
    pub last_active: i64,
    pub state: String,
    pub metadata: Option<String>, // JSON
}

/// Repository for session operations
pub struct SessionRepository {
    pool: SqlitePool,
}

impl SessionRepository {
    pub fn new(db: &Arc<Database>) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    /// Insert a new session
    pub async fn insert(&self, session: &SessionRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO sessions (
                id, user_id, chat_id, platform, current_intent, slots,
                messages, created_at, last_active, state, metadata
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&session.id)
        .bind(&session.user_id)
        .bind(&session.chat_id)
        .bind(&session.platform)
        .bind(&session.current_intent)
        .bind(&session.slots)
        .bind(&session.messages)
        .bind(session.created_at)
        .bind(session.last_active)
        .bind(&session.state)
        .bind(&session.metadata)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get a session by ID
    pub async fn get_by_id(&self, id: &str) -> Result<Option<SessionRecord>> {
        let session = sqlx::query_as::<_, SessionRecord>(
            "SELECT * FROM sessions WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(session)
    }

    /// Get or create a session for a user and chat
    pub async fn get_or_create(
        &self,
        user_id: &str,
        chat_id: &str,
        platform: &str,
    ) -> Result<SessionRecord> {
        // Try to find existing session
        if let Some(session) = sqlx::query_as::<_, SessionRecord>(
            "SELECT * FROM sessions WHERE user_id = ? AND chat_id = ? AND platform = ? AND state = 'active'"
        )
        .bind(user_id)
        .bind(chat_id)
        .bind(platform)
        .fetch_optional(&self.pool)
        .await?
        {
            return Ok(session);
        }

        // Create new session
        let now = chrono::Utc::now().timestamp();
        let id = uuid::Uuid::new_v4().to_string();
        let session = SessionRecord {
            id: id.clone(),
            user_id: user_id.to_string(),
            chat_id: chat_id.to_string(),
            platform: platform.to_string(),
            current_intent: None,
            slots: None,
            messages: None,
            created_at: now,
            last_active: now,
            state: "active".to_string(),
            metadata: None,
        };

        self.insert(&session).await?;
        Ok(session)
    }

    /// Update a session
    pub async fn update(&self, session: &SessionRecord) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE sessions SET
                current_intent = ?, slots = ?, messages = ?,
                last_active = ?, state = ?, metadata = ?
            WHERE id = ?
            "#,
        )
        .bind(&session.current_intent)
        .bind(&session.slots)
        .bind(&session.messages)
        .bind(session.last_active)
        .bind(&session.state)
        .bind(&session.metadata)
        .bind(&session.id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Update session last active timestamp
    pub async fn update_last_active(&self, id: &str, timestamp: i64) -> Result<()> {
        sqlx::query("UPDATE sessions SET last_active = ? WHERE id = ?")
            .bind(timestamp)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Set session state
    pub async fn set_state(&self, id: &str, state: &str) -> Result<()> {
        sqlx::query("UPDATE sessions SET state = ? WHERE id = ?")
            .bind(state)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// List sessions for a user
    pub async fn list_by_user(&self, user_id: &str) -> Result<Vec<SessionRecord>> {
        let sessions = sqlx::query_as::<_, SessionRecord>(
            "SELECT * FROM sessions WHERE user_id = ? ORDER BY last_active DESC"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(sessions)
    }

    /// List active sessions
    pub async fn list_active(&self, limit: Option<usize>) -> Result<Vec<SessionRecord>> {
        let query = if let Some(limit) = limit {
            format!("SELECT * FROM sessions WHERE state = 'active' ORDER BY last_active DESC LIMIT {}", limit)
        } else {
            "SELECT * FROM sessions WHERE state = 'active' ORDER BY last_active DESC".to_string()
        };

        let sessions = sqlx::query_as::<_, SessionRecord>(&query)
            .fetch_all(&self.pool)
            .await?;
        Ok(sessions)
    }

    /// Delete a session
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Delete inactive sessions older than given timestamp
    pub async fn delete_inactive_older_than(&self, timestamp: i64) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM sessions WHERE state != 'active' AND last_active < ?"
        )
        .bind(timestamp)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }
}
