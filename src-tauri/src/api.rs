// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Management API interface
//!
//! Provides HTTP API endpoints for remote management

use crate::database::Database;
use crate::dialog::engine::DialogEngine;
use crate::error::Result;
use crate::queue::TaskQueue;
use crate::security::rbac::RbacManager;
use crate::services::ServiceContainer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// API request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub query: HashMap<String, String>,
    pub body: Option<serde_json::Value>,
    pub client_ip: Option<String>,
    pub user_id: Option<String>,
}

/// API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: serde_json::Value,
}

impl ApiResponse {
    pub fn ok(data: serde_json::Value) -> Self {
        Self {
            status: 200,
            headers: HashMap::new(),
            body: serde_json::json!({ "success": true, "data": data }),
        }
    }

    pub fn created(data: serde_json::Value) -> Self {
        Self {
            status: 201,
            headers: HashMap::new(),
            body: serde_json::json!({ "success": true, "data": data }),
        }
    }

    pub fn error(status: u16, message: &str) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: serde_json::json!({ "success": false, "error": message }),
        }
    }

    pub fn not_found(message: &str) -> Self {
        Self::error(404, message)
    }

    pub fn unauthorized(message: &str) -> Self {
        Self::error(401, message)
    }

    pub fn forbidden(message: &str) -> Self {
        Self::error(403, message)
    }

    pub fn bad_request(message: &str) -> Self {
        Self::error(400, message)
    }
}

/// API route handler
pub struct ApiRouter {
    db: Arc<Database>,
    services: Arc<ServiceContainer>,
    rbac: Arc<RbacManager>,
    dialog: Arc<DialogEngine>,
    task_queue: Arc<TaskQueue>,
}

impl ApiRouter {
    pub fn new(
        db: Arc<Database>,
        services: Arc<ServiceContainer>,
        rbac: Arc<RbacManager>,
        dialog: Arc<DialogEngine>,
        task_queue: Arc<TaskQueue>,
    ) -> Self {
        Self { db, services, rbac, dialog, task_queue }
    }

    /// Route an API request to the appropriate handler
    pub async fn handle(&self, request: ApiRequest) -> Result<ApiResponse> {
        let path = request.path.trim_end_matches('/').to_string();

        // Authenticate request
        if let Err(e) = self.authenticate(&request).await {
            return Ok(ApiResponse::unauthorized(&e));
        }

        // Route to handler
        match (request.method.as_str(), path.as_str()) {
            // System endpoints
            ("GET", "/api/v1/system/info") => self.handle_system_info().await,
            ("GET", "/api/v1/system/stats") => self.handle_system_stats().await,

            // Skill endpoints
            ("GET", "/api/v1/skills") => self.handle_list_skills(request).await,
            ("GET", path) if path.starts_with("/api/v1/skills/") => {
                let id = path.strip_prefix("/api/v1/skills/").unwrap();
                self.handle_get_skill(id).await
            }
            ("POST", "/api/v1/skills/execute") => self.handle_execute_skill(request).await,

            // Task endpoints
            ("GET", "/api/v1/tasks") => self.handle_list_tasks(request).await,
            ("GET", path) if path.starts_with("/api/v1/tasks/") => {
                let id = path.strip_prefix("/api/v1/tasks/").unwrap();
                self.handle_get_task(id).await
            }
            ("POST", "/api/v1/tasks") => self.handle_create_task(request).await,
            ("DELETE", path) if path.starts_with("/api/v1/tasks/") => {
                let id = path.strip_prefix("/api/v1/tasks/").unwrap();
                self.handle_cancel_task(id).await
            }

            // User endpoints
            ("GET", "/api/v1/users") => self.handle_list_users(request).await,
            ("GET", path) if path.starts_with("/api/v1/users/") => {
                let id = path.strip_prefix("/api/v1/users/").unwrap();
                self.handle_get_user(id).await
            }

            // Dialog endpoints
            ("POST", "/api/v1/dialog/message") => self.handle_dialog_message(request).await,

            // Log endpoints
            ("GET", "/api/v1/logs") => self.handle_list_logs(request).await,

            // Backup endpoints
            ("POST", "/api/v1/backup") => self.handle_create_backup(request).await,
            ("POST", "/api/v1/backup/restore") => self.handle_restore_backup(request).await,

            // Health check
            ("GET", "/api/v1/health") => Ok(ApiResponse::ok(serde_json::json!({
                "status": "healthy",
                "version": env!("CARGO_PKG_VERSION"),
            }))),

            _ => Ok(ApiResponse::not_found("Endpoint not found")),
        }
    }

    async fn authenticate(&self, request: &ApiRequest) -> std::result::Result<(), String> {
        // Skip auth for health check
        if request.path == "/api/v1/health" {
            return Ok(());
        }

        // Check API key
        let api_key = request.headers.get("Authorization")
            .and_then(|h| h.strip_prefix("Bearer "))
            .or_else(|| request.headers.get("X-API-Key"));

        if api_key.is_none() {
            return Err("Missing authentication".to_string());
        }

        // In production, validate the API key against the database
        Ok(())
    }

    // System handlers
    async fn handle_system_info(&self) -> Result<ApiResponse> {
        let mut sys_service = crate::services::system_service::SystemService::new()?;
        let info = sys_service.get_system_info()?;
        Ok(ApiResponse::ok(serde_json::to_value(info)?))
    }

    async fn handle_system_stats(&self) -> Result<ApiResponse> {
        let stats = self.db.get_stats().await?;
        Ok(ApiResponse::ok(serde_json::to_value(stats)?))
    }

    // Skill handlers
    async fn handle_list_skills(&self, _request: ApiRequest) -> Result<ApiResponse> {
        use crate::database::repositories::SkillRepository;
        let repo = SkillRepository::new(&self.db);
        let skills = repo.list_all().await?;
        Ok(ApiResponse::ok(serde_json::to_value(skills)?))
    }

    async fn handle_get_skill(&self, id: &str) -> Result<ApiResponse> {
        use crate::database::repositories::SkillRepository;
        let repo = SkillRepository::new(&self.db);
        match repo.get_by_id(id).await? {
            Some(skill) => Ok(ApiResponse::ok(serde_json::to_value(skill)?)),
            None => Ok(ApiResponse::not_found("Skill not found")),
        }
    }

    async fn handle_execute_skill(&self, request: ApiRequest) -> Result<ApiResponse> {
        let body = request.body.unwrap_or(serde_json::Value::Null);
        Ok(ApiResponse::ok(serde_json::json!({
            "message": "Skill execution queued",
            "task_id": uuid::Uuid::new_v4().to_string(),
        })))
    }

    // Task handlers
    async fn handle_list_tasks(&self, request: ApiRequest) -> Result<ApiResponse> {
        use crate::database::repositories::TaskRepository;
        let repo = TaskRepository::new(&self.db);
        let user_id = request.query.get("user_id").map(|s| s.as_str()).unwrap_or("all");
        let tasks = if user_id == "all" {
            repo.list_by_status(crate::database::repositories::TaskStatus::Pending, Some(100)).await?
        } else {
            repo.list_by_user(user_id, Some(100)).await?
        };
        Ok(ApiResponse::ok(serde_json::to_value(tasks)?))
    }

    async fn handle_get_task(&self, id: &str) -> Result<ApiResponse> {
        use crate::database::repositories::TaskRepository;
        let repo = TaskRepository::new(&self.db);
        match repo.get_by_id(id).await? {
            Some(task) => Ok(ApiResponse::ok(serde_json::to_value(task)?)),
            None => Ok(ApiResponse::not_found("Task not found")),
        }
    }

    async fn handle_create_task(&self, _request: ApiRequest) -> Result<ApiResponse> {
        Ok(ApiResponse::created(serde_json::json!({
            "task_id": uuid::Uuid::new_v4().to_string(),
            "status": "pending",
        })))
    }

    async fn handle_cancel_task(&self, id: &str) -> Result<ApiResponse> {
        self.task_queue.cancel(id).await?;
        Ok(ApiResponse::ok(serde_json::json!({ "cancelled": true })))
    }

    // User handlers
    async fn handle_list_users(&self, _request: ApiRequest) -> Result<ApiResponse> {
        use crate::database::repositories::UserRepository;
        let repo = UserRepository::new(&self.db);
        let users = repo.list_all().await?;
        Ok(ApiResponse::ok(serde_json::to_value(users)?))
    }

    async fn handle_get_user(&self, id: &str) -> Result<ApiResponse> {
        use crate::database::repositories::UserRepository;
        let repo = UserRepository::new(&self.db);
        match repo.get_by_id(id).await? {
            Some(user) => Ok(ApiResponse::ok(serde_json::to_value(user)?)),
            None => Ok(ApiResponse::not_found("User not found")),
        }
    }

    // Dialog handler
    async fn handle_dialog_message(&self, request: ApiRequest) -> Result<ApiResponse> {
        let body = request.body.unwrap_or(serde_json::Value::Null);

        let user_id = body.get("user_id")
            .and_then(|v| v.as_str()).unwrap_or("anonymous");
        let chat_id = body.get("chat_id")
            .and_then(|v| v.as_str()).unwrap_or("default");
        let platform = body.get("platform")
            .and_then(|v| v.as_str()).unwrap_or("api");
        let message = body.get("message")
            .and_then(|v| v.as_str()).unwrap_or("");

        let response = self.dialog.process_message(user_id, chat_id, platform, message).await?;
        Ok(ApiResponse::ok(serde_json::to_value(response)?))
    }

    // Log handler
    async fn handle_list_logs(&self, request: ApiRequest) -> Result<ApiResponse> {
        use crate::database::repositories::LogRepository;
        let repo = LogRepository::new(&self.db);
        let user_id = request.query.get("user_id").map(|s| s.as_str()).unwrap_or("all");
        let logs = if user_id == "all" {
            repo.list_by_time_range(0, chrono::Utc::now().timestamp(), Some(100)).await?
        } else {
            repo.list_by_user(user_id, Some(100), None).await?
        };
        Ok(ApiResponse::ok(serde_json::to_value(logs)?))
    }

    // Backup handlers
    async fn handle_create_backup(&self, _request: ApiRequest) -> Result<ApiResponse> {
        let backup_path = crate::config::Config::data_dir()?
            .join("backups")
            .join(format!("backup_{}.zip", chrono::Utc::now().format("%Y%m%d_%H%M%S")));

        self.services.backup_service.create_backup(
            crate::services::backup_service::BackupOptions {
                destination: backup_path.clone(),
                ..Default::default()
            }
        ).await?;

        Ok(ApiResponse::ok(serde_json::json!({
            "backup_path": backup_path.to_string_lossy(),
        })))
    }

    async fn handle_restore_backup(&self, request: ApiRequest) -> Result<ApiResponse> {
        let body = request.body.unwrap_or(serde_json::Value::Null);
        let backup_path = body.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::error::AppError::Config("Missing backup path".to_string()))?;

        self.services.backup_service.restore_backup(
            std::path::Path::new(backup_path),
            crate::services::backup_service::RestoreOptions {
                force: false,
                stop_on_error: true,
            }
        ).await?;

        Ok(ApiResponse::ok(serde_json::json!({ "restored": true })))
    }
}
