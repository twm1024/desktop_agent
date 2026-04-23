// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Database migration system

#![allow(dead_code)]
use sqlx::SqlitePool;
use tracing::{info, warn};

pub mod m001_initial_schema;
pub mod m002_add_skill_checksum;
pub mod m003_add_user_roles;
pub mod m004_add_task_metadata;

/// Migration trait
pub trait Migration: Send + Sync {
    /// Migration version (e.g., "001")
    fn version(&self) -> &'static str;

    /// Migration name/description
    fn name(&self) -> &'static str;

    /// Migration SQL
    fn sql(&self) -> &'static str;

    /// Optional rollback SQL
    fn rollback_sql(&self) -> Option<&'static str> {
        None
    }
}

/// Migration manager
pub struct MigrationManager {
    pool: SqlitePool,
    migrations: Vec<Box<dyn Migration>>,
}

impl MigrationManager {
    /// Create a new migration manager
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            migrations: Vec::new(),
        }
    }

    /// Register a migration
    pub fn register_migration(&mut self, migration: Box<dyn Migration>) -> Result<(), String> {
        // Check for duplicate version
        if self.migrations.iter().any(|m| m.version() == migration.version()) {
            return Err(format!("Duplicate migration version: {}", migration.version()));
        }

        self.migrations.push(migration);
        Ok(())
    }

    /// Ensure migrations table exists
    async fn ensure_schema_migrations_table(&self) -> crate::error::Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at INTEGER NOT NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get applied migrations
    async fn get_applied_migrations(&self) -> crate::error::Result<Vec<String>> {
        self.ensure_schema_migrations_table().await?;

        let versions: Vec<String> = sqlx::query_scalar("SELECT version FROM schema_migrations ORDER BY version")
            .fetch_all(&self.pool)
            .await?;

        Ok(versions)
    }

    /// Apply pending migrations
    pub async fn apply_migrations(&self) -> crate::error::Result<usize> {
        let applied = self.get_applied_migrations().await?;

        // Sort migrations by version (use indices to avoid cloning Box<dyn Migration>)
        let mut indices: Vec<usize> = (0..self.migrations.len()).collect();
        indices.sort_by_key(|&i| self.migrations[i].version());

        let mut count = 0;

        for idx in indices {
            let migration = &self.migrations[idx];
            if applied.contains(&migration.version().to_string()) {
                info!("Migration {} ({}) already applied, skipping", migration.version(), migration.name());
                continue;
            }

            info!("Applying migration {} ({})", migration.version(), migration.name());

            // Apply migration in a transaction
            let mut tx = self.pool.begin().await?;

            // Execute migration SQL
            sqlx::query(migration.sql())
                .execute(&mut *tx)
                .await?;

            // Record migration
            let now = chrono::Utc::now().timestamp();
            sqlx::query(
                "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?, ?, ?)"
            )
            .bind(migration.version())
            .bind(migration.name())
            .bind(now)
            .execute(&mut *tx)
            .await?;

            tx.commit().await?;

            info!("Migration {} ({}) applied successfully", migration.version(), migration.name());
            count += 1;
        }

        Ok(count)
    }

    /// Rollback a specific migration
    pub async fn rollback_migration(&self, version: &str) -> crate::error::Result<bool> {
        let migration = self.migrations.iter().find(|m| m.version() == version);

        if let Some(migration) = migration {
            if let Some(rollback_sql) = migration.rollback_sql() {
                info!("Rolling back migration {} ({})", migration.version(), migration.name());

                let mut tx = self.pool.begin().await?;

                // Execute rollback SQL
                sqlx::query(rollback_sql)
                    .execute(&mut *tx)
                    .await?;

                // Remove migration record
                sqlx::query("DELETE FROM schema_migrations WHERE version = ?")
                    .bind(version)
                    .execute(&mut *tx)
                    .await?;

                tx.commit().await?;

                info!("Migration {} ({}) rolled back successfully", migration.version(), migration.name());
                return Ok(true);
            } else {
                warn!("Migration {} ({}) has no rollback defined", migration.version(), migration.name());
            }
        }

        Ok(false)
    }
}
