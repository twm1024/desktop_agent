// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

#![allow(dead_code)]
use crate::database::Database;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(TaskStatus::Pending),
            "running" => Some(TaskStatus::Running),
            "completed" => Some(TaskStatus::Completed),
            "failed" => Some(TaskStatus::Failed),
            "cancelled" => Some(TaskStatus::Cancelled),
            _ => None,
        }
    }
}

/// Task record in database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TaskRecord {
    pub id: String,
    pub user_id: String,
    pub skill_id: String,
    pub session_id: Option<String>,
    pub status: String,
    pub input_params: String, // JSON
    pub output_result: Option<String>, // JSON
    pub progress: i32,
    pub error_message: Option<String>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub duration_ms: Option<i64>,
    pub metadata: Option<String>, // JSON
    pub priority: i32,
    pub max_retries: i32,
    pub retry_count: i32,
    pub parent_task_id: Option<String>,
    pub scheduled_at: Option<i64>,
}

/// Repository for task operations
pub struct TaskRepository {
    pool: SqlitePool,
}

impl TaskRepository {
    pub fn new(db: &Arc<Database>) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    /// Insert a new task
    pub async fn insert(&self, task: &TaskRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO tasks (
                id, user_id, skill_id, session_id, status, input_params,
                output_result, progress, error_message, created_at,
                started_at, completed_at, duration_ms, metadata,
                priority, max_retries, retry_count, parent_task_id, scheduled_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&task.id)
        .bind(&task.user_id)
        .bind(&task.skill_id)
        .bind(&task.session_id)
        .bind(&task.status)
        .bind(&task.input_params)
        .bind(&task.output_result)
        .bind(task.progress)
        .bind(&task.error_message)
        .bind(task.created_at)
        .bind(task.started_at)
        .bind(task.completed_at)
        .bind(task.duration_ms)
        .bind(&task.metadata)
        .bind(task.priority)
        .bind(task.max_retries)
        .bind(task.retry_count)
        .bind(&task.parent_task_id)
        .bind(task.scheduled_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get a task by ID
    pub async fn get_by_id(&self, id: &str) -> Result<Option<TaskRecord>> {
        let task = sqlx::query_as::<_, TaskRecord>(
            "SELECT * FROM tasks WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(task)
    }

    /// Update a task
    pub async fn update(&self, task: &TaskRecord) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE tasks SET
                status = ?, output_result = ?, progress = ?,
                error_message = ?, started_at = ?, completed_at = ?,
                duration_ms = ?, metadata = ?, priority = ?,
                max_retries = ?, retry_count = ?, scheduled_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&task.status)
        .bind(&task.output_result)
        .bind(task.progress)
        .bind(&task.error_message)
        .bind(task.started_at)
        .bind(task.completed_at)
        .bind(task.duration_ms)
        .bind(&task.metadata)
        .bind(task.priority)
        .bind(task.max_retries)
        .bind(task.retry_count)
        .bind(task.scheduled_at)
        .bind(&task.id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Update task status
    pub async fn update_status(&self, id: &str, status: TaskStatus) -> Result<()> {
        let timestamp = chrono::Utc::now().timestamp();

        match status {
            TaskStatus::Running => {
                sqlx::query(
                    "UPDATE tasks SET status = ?, started_at = ? WHERE id = ?"
                )
                .bind(status.as_str())
                .bind(timestamp)
                .bind(id)
                .execute(&self.pool)
                .await?;
            }
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled => {
                // Calculate duration
                let task = self.get_by_id(id).await?;
                if let Some(t) = task {
                    if let Some(started) = t.started_at {
                        let duration = timestamp - started;
                        sqlx::query(
                            "UPDATE tasks SET status = ?, completed_at = ?, duration_ms = ? WHERE id = ?"
                        )
                        .bind(status.as_str())
                        .bind(timestamp)
                        .bind(duration * 1000)
                        .bind(id)
                        .execute(&self.pool)
                        .await?;
                    } else {
                        sqlx::query(
                            "UPDATE tasks SET status = ?, completed_at = ? WHERE id = ?"
                        )
                        .bind(status.as_str())
                        .bind(timestamp)
                        .bind(id)
                        .execute(&self.pool)
                        .await?;
                    }
                }
            }
            _ => {
                sqlx::query("UPDATE tasks SET status = ? WHERE id = ?")
                    .bind(status.as_str())
                    .bind(id)
                    .execute(&self.pool)
                    .await?;
            }
        }

        Ok(())
    }

    /// Update task progress
    pub async fn update_progress(&self, id: &str, progress: i32) -> Result<()> {
        sqlx::query("UPDATE tasks SET progress = ? WHERE id = ?")
            .bind(progress)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Increment retry count
    pub async fn increment_retry(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE tasks SET retry_count = retry_count + 1 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// List tasks for a user
    pub async fn list_by_user(&self, user_id: &str, limit: Option<usize>) -> Result<Vec<TaskRecord>> {
        let query = if let Some(limit) = limit {
            format!("SELECT * FROM tasks WHERE user_id = ? ORDER BY created_at DESC LIMIT {}", limit)
        } else {
            "SELECT * FROM tasks WHERE user_id = ? ORDER BY created_at DESC".to_string()
        };

        let tasks = sqlx::query_as::<_, TaskRecord>(&query)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(tasks)
    }

    /// List tasks by status
    pub async fn list_by_status(&self, status: TaskStatus, limit: Option<usize>) -> Result<Vec<TaskRecord>> {
        let query = if let Some(limit) = limit {
            format!("SELECT * FROM tasks WHERE status = ? ORDER BY priority DESC, created_at ASC LIMIT {}", limit)
        } else {
            "SELECT * FROM tasks WHERE status = ? ORDER BY priority DESC, created_at ASC".to_string()
        };

        let tasks = sqlx::query_as::<_, TaskRecord>(&query)
            .bind(status.as_str())
            .fetch_all(&self.pool)
            .await?;
        Ok(tasks)
    }

    /// List pending tasks that are ready to run
    pub async fn list_ready_tasks(&self, limit: usize) -> Result<Vec<TaskRecord>> {
        let now = chrono::Utc::now().timestamp();
        let tasks = sqlx::query_as::<_, TaskRecord>(
            r#"
            SELECT * FROM tasks
            WHERE status = 'pending'
            AND (scheduled_at IS NULL OR scheduled_at <= ?)
            AND retry_count < max_retries
            ORDER BY priority DESC, created_at ASC
            LIMIT ?
            "#
        )
        .bind(now)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(tasks)
    }

    /// List child tasks
    pub async fn list_children(&self, parent_id: &str) -> Result<Vec<TaskRecord>> {
        let tasks = sqlx::query_as::<_, TaskRecord>(
            "SELECT * FROM tasks WHERE parent_task_id = ? ORDER BY created_at ASC"
        )
        .bind(parent_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(tasks)
    }

    /// Delete a task
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Delete old completed/failed tasks
    pub async fn delete_old(&self, older_than: i64, status: Option<TaskStatus>) -> Result<u64> {
        let result = if let Some(status) = status {
            sqlx::query(
                "DELETE FROM tasks WHERE status = ? AND completed_at < ?"
            )
            .bind(status.as_str())
            .bind(older_than)
            .execute(&self.pool)
            .await?
        } else {
            sqlx::query(
                "DELETE FROM tasks WHERE completed_at < ?"
            )
            .bind(older_than)
            .execute(&self.pool)
            .await?
        };
        Ok(result.rows_affected())
    }
}
