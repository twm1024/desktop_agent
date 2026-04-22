// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

use crate::database::Database;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;

/// Log record in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRecord {
    pub id: Option<i64>,
    pub timestamp: i64,
    pub user_id: String,
    pub platform: String,
    pub operation_type: String,
    pub operation_data: String, // JSON
    pub result: String, // JSON
    pub skill_id: Option<String>,
    pub session_id: Option<String>,
    pub duration_ms: Option<i64>,
    pub status: String,
    pub error_message: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

/// Repository for operation log operations
pub struct LogRepository {
    pool: SqlitePool,
}

impl LogRepository {
    pub fn new(db: &Arc<Database>) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    /// Insert a new log entry
    pub async fn insert(&self, log: &LogRecord) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO operation_logs (
                timestamp, user_id, platform, operation_type, operation_data,
                result, skill_id, session_id, duration_ms, status,
                error_message, ip_address, user_agent
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(log.timestamp)
        .bind(&log.user_id)
        .bind(&log.platform)
        .bind(&log.operation_type)
        .bind(&log.operation_data)
        .bind(&log.result)
        .bind(&log.skill_id)
        .bind(&log.session_id)
        .bind(log.duration_ms)
        .bind(&log.status)
        .bind(&log.error_message)
        .bind(&log.ip_address)
        .bind(&log.user_agent)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get a log entry by ID
    pub async fn get_by_id(&self, id: i64) -> Result<Option<LogRecord>> {
        let log = sqlx::query_as::<_, LogRecord>(
            "SELECT * FROM operation_logs WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(log)
    }

    /// List logs for a user
    pub async fn list_by_user(
        &self,
        user_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<LogRecord>> {
        let mut query = "SELECT * FROM operation_logs WHERE user_id = ?".to_string();

        if offset.is_some() || limit.is_some() {
            query.push_str(" ORDER BY timestamp DESC");

            if let Some(offset) = offset {
                query.push_str(&format!(" OFFSET {}", offset));
            }

            if let Some(limit) = limit {
                query.push_str(&format!(" LIMIT {}", limit));
            }
        } else {
            query.push_str(" ORDER BY timestamp DESC");
        }

        let logs = sqlx::query_as::<_, LogRecord>(&query)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(logs)
    }

    /// List logs for a session
    pub async fn list_by_session(
        &self,
        session_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<LogRecord>> {
        let query = if let Some(limit) = limit {
            format!(
                "SELECT * FROM operation_logs WHERE session_id = ? ORDER BY timestamp DESC LIMIT {}",
                limit
            )
        } else {
            "SELECT * FROM operation_logs WHERE session_id = ? ORDER BY timestamp DESC".to_string()
        };

        let logs = sqlx::query_as::<_, LogRecord>(&query)
            .bind(session_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(logs)
    }

    /// List logs by operation type
    pub async fn list_by_operation_type(
        &self,
        operation_type: &str,
        limit: Option<usize>,
    ) -> Result<Vec<LogRecord>> {
        let query = if let Some(limit) = limit {
            format!(
                "SELECT * FROM operation_logs WHERE operation_type = ? ORDER BY timestamp DESC LIMIT {}",
                limit
            )
        } else {
            "SELECT * FROM operation_logs WHERE operation_type = ? ORDER BY timestamp DESC".to_string()
        };

        let logs = sqlx::query_as::<_, LogRecord>(&query)
            .bind(operation_type)
            .fetch_all(&self.pool)
            .await?;
        Ok(logs)
    }

    /// List logs by status
    pub async fn list_by_status(
        &self,
        status: &str,
        limit: Option<usize>,
    ) -> Result<Vec<LogRecord>> {
        let query = if let Some(limit) = limit {
            format!(
                "SELECT * FROM operation_logs WHERE status = ? ORDER BY timestamp DESC LIMIT {}",
                limit
            )
        } else {
            "SELECT * FROM operation_logs WHERE status = ? ORDER BY timestamp DESC".to_string()
        };

        let logs = sqlx::query_as::<_, LogRecord>(&query)
            .bind(status)
            .fetch_all(&self.pool)
            .await?;
        Ok(logs)
    }

    /// List logs in a time range
    pub async fn list_by_time_range(
        &self,
        start: i64,
        end: i64,
        limit: Option<usize>,
    ) -> Result<Vec<LogRecord>> {
        let query = if let Some(limit) = limit {
            format!(
                "SELECT * FROM operation_logs WHERE timestamp >= ? AND timestamp <= ? ORDER BY timestamp DESC LIMIT {}",
                limit
            )
        } else {
            "SELECT * FROM operation_logs WHERE timestamp >= ? AND timestamp <= ? ORDER BY timestamp DESC".to_string()
        };

        let logs = sqlx::query_as::<_, LogRecord>(&query)
            .bind(start)
            .bind(end)
            .fetch_all(&self.pool)
            .await?;
        Ok(logs)
    }

    /// Get log statistics for a user
    pub async fn get_user_stats(
        &self,
        user_id: &str,
        start_time: Option<i64>,
        end_time: Option<i64>,
    ) -> Result<UserLogStats> {
        let (start, end) = match (start_time, end_time) {
            (Some(s), Some(e)) => (s, e),
            (Some(s), None) => (s, chrono::Utc::now().timestamp()),
            (None, Some(e)) => (chrono::Utc::now().timestamp() - 86400, e),
            (None, None) => (chrono::Utc::now().timestamp() - 86400, chrono::Utc::now().timestamp()),
        };

        let total_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM operation_logs WHERE user_id = ? AND timestamp >= ? AND timestamp <= ?"
        )
        .bind(user_id)
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await?;

        let success_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM operation_logs WHERE user_id = ? AND timestamp >= ? AND timestamp <= ? AND status = 'success'"
        )
        .bind(user_id)
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await?;

        let failed_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM operation_logs WHERE user_id = ? AND timestamp >= ? AND timestamp <= ? AND status = 'error'"
        )
        .bind(user_id)
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await?;

        let total_duration: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(duration_ms), 0) FROM operation_logs WHERE user_id = ? AND timestamp >= ? AND timestamp <= ?"
        )
        .bind(user_id)
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await?;

        Ok(UserLogStats {
            total_count: total_count as usize,
            success_count: success_count as usize,
            failed_count: failed_count as usize,
            total_duration_ms: total_duration,
        })
    }

    /// Delete old logs
    pub async fn delete_old(&self, older_than: i64) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM operation_logs WHERE timestamp < ?"
        )
        .bind(older_than)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Delete logs for a specific user
    pub async fn delete_for_user(&self, user_id: &str) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM operation_logs WHERE user_id = ?"
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }
}

/// User log statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLogStats {
    pub total_count: usize,
    pub success_count: usize,
    pub failed_count: usize,
    pub total_duration_ms: i64,
}

impl UserLogStats {
    pub fn success_rate(&self) -> f64 {
        if self.total_count == 0 {
            0.0
        } else {
            self.success_count as f64 / self.total_count as f64
        }
    }

    pub fn average_duration_ms(&self) -> f64 {
        if self.total_count == 0 {
            0.0
        } else {
            self.total_duration_ms as f64 / self.total_count as f64
        }
    }
}
