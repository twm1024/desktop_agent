// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Database module with migration system and repository layer

#![allow(dead_code)]
pub mod migrations;
pub mod repositories;

use crate::error::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::{Sqlite, Transaction};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tracing::info;

pub use migrations::MigrationManager;

/// Main database connection pool
pub struct Database {
    pool: SqlitePool,
    db_path: PathBuf,
}

impl Database {
    /// Create a new database connection
    pub async fn new(db_path: &Path) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Create connection options with WAL mode for better concurrency
        let options = SqliteConnectOptions::from_str(db_path.to_str().unwrap())?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            .busy_timeout(std::time::Duration::from_secs(30));

        // Create connection pool
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .min_connections(2)
            .acquire_timeout(std::time::Duration::from_secs(30))
            .connect_with(options)
            .await?;

        info!("Database connected at {:?}", db_path);

        let db = Self { pool, db_path: db_path.to_path_buf() };

        // Run migrations
        db.run_migrations().await?;

        Ok(db)
    }

    /// Get the connection pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Begin a transaction
    pub async fn begin(&self) -> Result<Transaction<'static, Sqlite>> {
        Ok(self.pool.begin().await?)
    }

    /// Run database migrations
    async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations...");

        let mut manager = MigrationManager::new(self.pool.clone());

        // Register all migrations
        manager.register_migration(migrations::m001_initial_schema::migration())?;
        manager.register_migration(migrations::m002_add_skill_checksum::migration())?;
        manager.register_migration(migrations::m003_add_user_roles::migration())?;
        manager.register_migration(migrations::m004_add_task_metadata::migration())?;

        // Apply pending migrations
        let applied = manager.apply_migrations().await?;

        if applied > 0 {
            info!("Applied {} database migration(s)", applied);
        } else {
            info!("Database is up to date");
        }

        Ok(())
    }

    /// Close the database connection
    pub async fn close(&self) {
        self.pool.close().await;
        info!("Database connection closed");
    }

    /// Perform database backup
    pub async fn backup(&self, backup_path: &Path) -> Result<()> {
        info!("Creating database backup to {:?}", backup_path);

        // Ensure backup directory exists
        if let Some(parent) = backup_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Use SQL backup command (SQLite 3.27+)
        sqlx::query(&format!(
            "VACUUM INTO '{}'",
            backup_path.to_str().unwrap().replace('\\', "\\\\")
        ))
        .execute(&self.pool)
        .await?;

        info!("Database backup completed");
        Ok(())
    }

    /// Restore database from backup
    pub async fn restore(&self, backup_path: &Path) -> Result<()> {
        info!("Restoring database from {:?}", backup_path);

        if !backup_path.exists() {
            return Err(crate::error::AppError::Database(
                format!("Backup file not found: {:?}", backup_path)
            ));
        }

        // Get db path before closing pool
        let db_path = self.db_path.clone();

        // Close existing pool
        self.pool.close().await;

        // Copy backup file to database location
        tokio::fs::copy(backup_path, &db_path).await?;

        info!("Database restore completed");
        Ok(())
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<DatabaseStats> {
        let skills_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM skills")
            .fetch_one(&self.pool)
            .await?;

        let users_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;

        let sessions_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions")
            .fetch_one(&self.pool)
            .await?;

        let tasks_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&self.pool)
            .await?;

        let logs_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM operation_logs")
            .fetch_one(&self.pool)
            .await?;

        // Get database size
        let db_size: i64 = sqlx::query_scalar("SELECT page_count * page_size as size FROM pragma_page_count(), pragma_page_size()")
            .fetch_one(&self.pool)
            .await?;

        Ok(DatabaseStats {
            skills_count: skills_count as usize,
            users_count: users_count as usize,
            sessions_count: sessions_count as usize,
            tasks_count: tasks_count as usize,
            logs_count: logs_count as usize,
            database_size_bytes: db_size as u64,
        })
    }

    /// Vacuum database to reclaim space
    pub async fn vacuum(&self) -> Result<()> {
        info!("Running VACUUM on database");
        sqlx::query("VACUUM").execute(&self.pool).await?;
        info!("VACUUM completed");
        Ok(())
    }

    /// Analyze database to update query planner statistics
    pub async fn analyze(&self) -> Result<()> {
        info!("Running ANALYZE on database");
        sqlx::query("ANALYZE").execute(&self.pool).await?;
        info!("ANALYZE completed");
        Ok(())
    }
}

/// Database statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct DatabaseStats {
    pub skills_count: usize,
    pub users_count: usize,
    pub sessions_count: usize,
    pub tasks_count: usize,
    pub logs_count: usize,
    pub database_size_bytes: u64,
}

impl DatabaseStats {
    /// Get database size in human-readable format
    pub fn size_human(&self) -> String {
        const KB: u64 = 1024;
        const MB: u64 = 1024 * KB;
        const GB: u64 = 1024 * MB;

        if self.database_size_bytes >= GB {
            format!("{:.2} GB", self.database_size_bytes as f64 / GB as f64)
        } else if self.database_size_bytes >= MB {
            format!("{:.2} MB", self.database_size_bytes as f64 / MB as f64)
        } else if self.database_size_bytes >= KB {
            format!("{:.2} KB", self.database_size_bytes as f64 / KB as f64)
        } else {
            format!("{} B", self.database_size_bytes)
        }
    }
}
