// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

use super::Migration;

pub fn migration() -> Box<dyn Migration> {
    Box::new(M001InitialSchema)
}

struct M001InitialSchema;

impl Migration for M001InitialSchema {
    fn version(&self) -> &'static str {
        "001"
    }

    fn name(&self) -> &'static str {
        "Initial schema"
    }

    fn sql(&self) -> &'static str {
        r#"
        -- Skills table
        CREATE TABLE IF NOT EXISTS skills (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            version TEXT NOT NULL,
            description TEXT,
            author TEXT,
            tags TEXT,
            manifest TEXT NOT NULL,
            enabled BOOLEAN DEFAULT 1,
            installed_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            last_executed_at INTEGER,
            execution_count INTEGER DEFAULT 0,
            source TEXT,
            metadata TEXT
        );

        -- Sessions table
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            chat_id TEXT NOT NULL,
            platform TEXT NOT NULL,
            current_intent TEXT,
            slots TEXT,
            messages TEXT,
            created_at INTEGER NOT NULL,
            last_active INTEGER NOT NULL,
            state TEXT DEFAULT 'active',
            metadata TEXT
        );

        -- Operation logs table
        CREATE TABLE IF NOT EXISTS operation_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            user_id TEXT NOT NULL,
            platform TEXT NOT NULL,
            operation_type TEXT NOT NULL,
            operation_data TEXT NOT NULL,
            result TEXT NOT NULL,
            skill_id TEXT,
            session_id TEXT,
            duration_ms INTEGER,
            status TEXT NOT NULL,
            error_message TEXT,
            ip_address TEXT,
            user_agent TEXT
        );

        -- Users table
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            platform TEXT NOT NULL,
            platform_user_id TEXT NOT NULL,
            name TEXT,
            avatar TEXT,
            role TEXT DEFAULT 'user',
            permissions TEXT,
            created_at INTEGER NOT NULL,
            last_active_at INTEGER NOT NULL,
            is_blocked BOOLEAN DEFAULT 0,
            metadata TEXT,
            UNIQUE(platform, platform_user_id)
        );

        -- Tasks table
        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            skill_id TEXT NOT NULL,
            session_id TEXT,
            status TEXT NOT NULL,
            input_params TEXT NOT NULL,
            output_result TEXT,
            progress INTEGER DEFAULT 0,
            error_message TEXT,
            created_at INTEGER NOT NULL,
            started_at INTEGER,
            completed_at INTEGER,
            duration_ms INTEGER,
            metadata TEXT
        );

        -- Create indexes
        CREATE INDEX IF NOT EXISTS idx_skills_enabled ON skills(enabled);
        CREATE INDEX IF NOT EXISTS idx_skills_name ON skills(name);
        CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
        CREATE INDEX IF NOT EXISTS idx_sessions_last_active ON sessions(last_active);
        CREATE INDEX IF NOT EXISTS idx_logs_user ON operation_logs(user_id);
        CREATE INDEX IF NOT EXISTS idx_logs_timestamp ON operation_logs(timestamp);
        CREATE INDEX IF NOT EXISTS idx_tasks_user ON tasks(user_id);
        CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
        CREATE INDEX IF NOT EXISTS idx_users_platform ON users(platform, platform_user_id);
        "#
    }

    fn rollback_sql(&self) -> Option<&'static str> {
        Some(r#"
        DROP INDEX IF EXISTS idx_users_platform;
        DROP INDEX IF EXISTS idx_tasks_status;
        DROP INDEX IF EXISTS idx_tasks_user;
        DROP INDEX IF EXISTS idx_logs_timestamp;
        DROP INDEX IF EXISTS idx_logs_user;
        DROP INDEX IF EXISTS idx_sessions_last_active;
        DROP INDEX IF EXISTS idx_sessions_user;
        DROP INDEX IF EXISTS idx_skills_name;
        DROP INDEX IF EXISTS idx_skills_enabled;
        DROP TABLE IF EXISTS tasks;
        DROP TABLE IF EXISTS users;
        DROP TABLE IF EXISTS operation_logs;
        DROP TABLE IF EXISTS sessions;
        DROP TABLE IF EXISTS skills;
        "#)
    }
}
