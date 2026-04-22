// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

use super::Migration;

pub fn migration() -> Box<dyn Migration> {
    Box::new(M003AddUserRoles)
}

struct M003AddUserRoles;

impl Migration for M003AddUserRoles {
    fn version(&self) -> &'static str {
        "003"
    }

    fn name(&self) -> &'static str {
        "Enhance user roles and permissions"
    }

    fn sql(&self) -> &'static str {
        r#"
        -- Add quota fields to users
        ALTER TABLE users ADD COLUMN daily_quota INTEGER DEFAULT 1000;
        ALTER TABLE users ADD COLUMN quota_reset_at INTEGER;
        ALTER TABLE users ADD COLUMN api_key TEXT UNIQUE;
        CREATE INDEX IF NOT EXISTS idx_users_api_key ON users(api_key);

        -- Create user activity tracking table
        CREATE TABLE IF NOT EXISTS user_activity (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT NOT NULL,
            activity_type TEXT NOT NULL,
            activity_data TEXT,
            timestamp INTEGER NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_user_activity_user ON user_activity(user_id);
        CREATE INDEX IF NOT EXISTS idx_user_activity_timestamp ON user_activity(timestamp);
        "#
    }

    fn rollback_sql(&self) -> Option<&'static str> {
        Some(r#"
        DROP INDEX IF EXISTS idx_user_activity_timestamp;
        DROP INDEX IF EXISTS idx_user_activity_user;
        DROP TABLE IF EXISTS user_activity;
        DROP INDEX IF EXISTS idx_users_api_key;
        -- Note: SQLite doesn't support dropping columns, so we can't rollback fully
        "#)
    }
}
