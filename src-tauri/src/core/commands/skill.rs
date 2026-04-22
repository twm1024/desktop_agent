// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

use crate::core::state::AppState;
use crate::error::Result;
use crate::skill::types::{SkillContext, SkillParameters, SkillResult};
use tauri::State;

#[tauri::command]
pub async fn list_skills(state: State<'_, AppState>) -> Result<Vec<crate::skill::types::SkillInfo>> {
    state.skill_engine.list_skills().await
}

#[tauri::command]
pub async fn get_skill(skill_id: String, state: State<'_, AppState>) -> Result<crate::skill::types::SkillInfo> {
    state.skill_engine.get_skill(&skill_id).await
}

#[tauri::command]
pub async fn execute_skill(
    skill_id: String,
    parameters: serde_json::Value,
    user_id: String,
    chat_id: String,
    platform: String,
    state: State<'_, AppState>,
) -> Result<SkillResult> {
    let params = SkillParameters {
        values: parameters,
    };

    let context = SkillContext {
        user_id,
        chat_id,
        platform,
        session_id: format!("{}:{}:{}", platform, user_id, chat_id),
        timestamp: chrono::Utc::now().timestamp(),
    };

    state
        .skill_engine
        .execute_skill(&skill_id, params, context)
        .await
}
