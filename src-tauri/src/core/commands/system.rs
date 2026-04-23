// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

use crate::core::state::AppState;
use crate::error::Result;
use tauri::State;

#[tauri::command]
pub async fn get_system_info(state: State<'_, AppState>) -> Result<crate::services::system_service::SystemInfo> {
    let system_service = state.services.system_service.as_ref();
    let info = system_service.get_system_info()?;
    Ok(info)
}
