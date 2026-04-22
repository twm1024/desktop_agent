// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

use crate::core::state::AppState;
use crate::error::Result;
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub async fn list_directory(
    path: PathBuf,
    show_hidden: bool,
    recursive: bool,
    state: State<'_, AppState>,
) -> Result<Vec<crate::services::file_service::FileInfo>> {
    state
        .services
        .file_service
        .list_directory(&path, show_hidden, recursive)
        .await
}

#[tauri::command]
pub async fn search_files(
    path: PathBuf,
    name_pattern: Option<String>,
    content_pattern: Option<String>,
    max_results: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::services::file_service::SearchResult>> {
    state
        .services
        .file_service
        .search_files(&path, name_pattern.as_deref(), content_pattern.as_deref(), max_results.unwrap_or(100))
        .await
}
