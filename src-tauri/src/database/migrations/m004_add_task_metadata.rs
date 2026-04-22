// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

use super::Migration;

pub fn migration() -> Box<dyn Migration> {
    Box::new(M004AddTaskMetadata)
}

struct M004AddTaskMetadata;

impl Migration for M004AddTaskMetadata {
    fn version(&self) -> &'static str {
        "004"
    }

    fn name(&self) -> &'static str {
        "Enhance task tracking with metadata"
    }

    fn sql(&self) -> &'static str {
        r#"
        -- Add priority and retry fields to tasks
        ALTER TABLE tasks ADD COLUMN priority INTEGER DEFAULT 0;
        ALTER TABLE tasks ADD COLUMN max_retries INTEGER DEFAULT 3;
        ALTER TABLE tasks ADD COLUMN retry_count INTEGER DEFAULT 0;
        ALTER TABLE tasks ADD COLUMN parent_task_id TEXT;
        ALTER TABLE tasks ADD COLUMN scheduled_at INTEGER;

        -- Create index for priority queries
        CREATE INDEX IF NOT EXISTS idx_tasks_priority ON tasks(priority);
        CREATE INDEX IF NOT EXISTS idx_tasks_parent ON tasks(parent_task_id);

        -- Create task queue table for better scheduling
        CREATE TABLE IF NOT EXISTS task_queue (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id TEXT NOT NULL,
            priority INTEGER NOT NULL DEFAULT 0,
            scheduled_at INTEGER NOT NULL,
            attempts INTEGER DEFAULT 0,
            locked_until INTEGER,
            created_at INTEGER NOT NULL,
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_task_queue_scheduled ON task_queue(scheduled_at, priority);
        CREATE INDEX IF NOT EXISTS idx_task_queue_locked ON task_queue(locked_until);
        "#
    }

    fn rollback_sql(&self) -> Option<&'static str> {
        Some(r#"
        DROP INDEX IF EXISTS idx_task_queue_locked;
        DROP INDEX IF EXISTS idx_task_queue_scheduled;
        DROP TABLE IF EXISTS task_queue;
        DROP INDEX IF EXISTS idx_tasks_parent;
        DROP INDEX IF EXISTS idx_tasks_priority;
        -- Note: SQLite doesn't support dropping columns, so we can't rollback fully
        "#)
    }
}
