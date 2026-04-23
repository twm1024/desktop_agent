// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

use crate::config::Config;
use crate::error::Result;

#[tauri::command]
pub async fn get_config(key: Option<String>) -> Result<serde_json::Value> {
    let config = Config::load().await?;

    if let Some(k) = key {
        // TODO: Get specific config key
        Ok(serde_json::json!({
            "key": k,
            "value": null
        }))
    } else {
        // Return entire config (sensitive info filtered)
        Ok(serde_json::to_value(config)?)
    }
}

#[tauri::command]
pub async fn set_config(_configs: serde_json::Value) -> Result<()> {
    // TODO: Set config values
    Ok(())
}
