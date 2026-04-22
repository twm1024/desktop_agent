// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

use super::Migration;

pub fn migration() -> Box<dyn Migration> {
    Box::new(M002AddSkillChecksum)
}

struct M002AddSkillChecksum;

impl Migration for M002AddSkillChecksum {
    fn version(&self) -> &'static str {
        "002"
    }

    fn name(&self) -> &'static str {
        "Add checksum to skills table"
    }

    fn sql(&self) -> &'static str {
        r#"
        ALTER TABLE skills ADD COLUMN checksum TEXT;
        CREATE INDEX IF NOT EXISTS idx_skills_checksum ON skills(checksum);
        "#
    }

    fn rollback_sql(&self) -> Option<&'static str> {
        Some(r#"
        DROP INDEX IF EXISTS idx_skills_checksum;
        -- Note: SQLite doesn't support dropping columns, so we can't rollback fully
        "#)
    }
}
