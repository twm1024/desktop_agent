// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

use crate::database::Database;
use crate::services::ServiceContainer;
use crate::skill::SkillEngine;
use std::sync::Arc;

/// Application state shared across Tauri commands
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub services: Arc<ServiceContainer>,
    pub skill_engine: Arc<SkillEngine>,
}

impl AppState {
    pub fn new(
        db: Arc<Database>,
        services: Arc<ServiceContainer>,
        skill_engine: Arc<SkillEngine>,
    ) -> Self {
        Self {
            db,
            services,
            skill_engine,
        }
    }
}
