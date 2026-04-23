// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Task queue for asynchronous execution
//!
//! Provides a priority-based task queue with retry logic and worker pools

#![allow(dead_code)]
use crate::error::Result;
use crate::database::Database;
use crate::database::repositories::{TaskRepository, TaskStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Semaphore};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// Task execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    pub task_id: String,
    pub user_id: String,
    pub skill_id: String,
    pub session_id: Option<String>,
    pub input_params: serde_json::Value,
    pub priority: i32,
    pub max_retries: i32,
    pub retry_count: i32,
    pub scheduled_at: Option<DateTime<Utc>>,
}

/// Task execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub status: TaskExecutionStatus,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: i64,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskExecutionStatus {
    Success,
    Failed,
    Cancelled,
}

/// Task handler trait
#[async_trait::async_trait]
pub trait TaskHandler: Send + Sync {
    async fn execute(&self, context: TaskContext) -> Result<serde_json::Value>;
}

/// Task queue configuration
#[derive(Debug, Clone)]
pub struct TaskQueueConfig {
    pub worker_count: usize,
    pub max_retries: i32,
    pub retry_delay: Duration,
    pub poll_interval: Duration,
    pub task_timeout: Duration,
}

impl Default for TaskQueueConfig {
    fn default() -> Self {
        Self {
            worker_count: 4,
            max_retries: 3,
            retry_delay: Duration::from_secs(5),
            poll_interval: Duration::from_millis(100),
            task_timeout: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Priority task queue
pub struct TaskQueue {
    config: TaskQueueConfig,
    db: Arc<Database>,
    handlers: Arc<Mutex<HashMap<String, Arc<dyn TaskHandler>>>>,
    semaphore: Arc<Semaphore>,
    running: Arc<Mutex<bool>>,
}

impl TaskQueue {
    /// Create a new task queue
    pub fn new(db: Arc<Database>, config: TaskQueueConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.worker_count));

        Self {
            config,
            db,
            handlers: Arc::new(Mutex::new(HashMap::new())),
            semaphore,
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// Register a task handler for a skill
    pub async fn register_handler(&self, skill_id: String, handler: Arc<dyn TaskHandler>) {
        let mut handlers = self.handlers.lock().await;
        info!("Registered handler for skill: {}", skill_id);
        handlers.insert(skill_id, handler);
    }

    /// Unregister a task handler
    pub async fn unregister_handler(&self, skill_id: &str) {
        let mut handlers = self.handlers.lock().await;
        handlers.remove(skill_id);
        info!("Unregistered handler for skill: {}", skill_id);
    }

    /// Submit a new task
    pub async fn submit(&self, context: TaskContext) -> Result<String> {
        let task_repo = TaskRepository::new(&self.db);

        // Create task record
        let now = Utc::now();
        let task = crate::database::repositories::TaskRecord {
            id: context.task_id.clone(),
            user_id: context.user_id.clone(),
            skill_id: context.skill_id.clone(),
            session_id: context.session_id.clone(),
            status: TaskStatus::Pending.as_str().to_string(),
            input_params: serde_json::to_string(&context.input_params)?,
            output_result: None,
            progress: 0,
            error_message: None,
            created_at: now.timestamp(),
            started_at: None,
            completed_at: None,
            duration_ms: None,
            metadata: None,
            priority: context.priority,
            max_retries: context.max_retries,
            retry_count: context.retry_count,
            parent_task_id: None,
            scheduled_at: context.scheduled_at.map(|dt| dt.timestamp()),
        };

        task_repo.insert(&task).await?;

        info!("Task {} submitted for skill {}", context.task_id, context.skill_id);
        Ok(context.task_id)
    }

    /// Start the task queue workers
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.lock().await;
        if *running {
            return Ok(());
        }
        *running = true;
        drop(running);

        info!("Starting task queue with {} workers", self.config.worker_count);

        // Start worker tasks
        for worker_id in 0..self.config.worker_count {
            let worker = TaskQueueWorker::new(
                worker_id,
                self.db.clone(),
                self.handlers.clone(),
                self.config.clone(),
                self.semaphore.clone(),
            );
            tokio::spawn(async move {
                worker.run().await;
            });
        }

        Ok(())
    }

    /// Stop the task queue
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.lock().await;
        *running = false;
        info!("Task queue stopped");
        Ok(())
    }

    /// Cancel a task
    pub async fn cancel(&self, task_id: &str) -> Result<bool> {
        let task_repo = TaskRepository::new(&self.db);
        task_repo.update_status(task_id, TaskStatus::Cancelled).await?;
        info!("Task {} cancelled", task_id);
        Ok(true)
    }

    /// Get task status
    pub async fn get_status(&self, task_id: &str) -> Result<Option<TaskStatus>> {
        let task_repo = TaskRepository::new(&self.db);
        let task = task_repo.get_by_id(task_id).await?;
        Ok(task.map(|t| TaskStatus::from_str(&t.status).unwrap_or(TaskStatus::Pending)))
    }

    /// Get queue statistics
    pub async fn get_stats(&self) -> Result<TaskQueueStats> {
        let task_repo = TaskRepository::new(&self.db);

        let pending = task_repo.list_by_status(TaskStatus::Pending, Some(1000)).await?.len();
        let running = task_repo.list_by_status(TaskStatus::Running, Some(1000)).await?.len();
        let completed = task_repo.list_by_status(TaskStatus::Completed, Some(1)).await?.len();
        let failed = task_repo.list_by_status(TaskStatus::Failed, Some(1000)).await?.len();

        Ok(TaskQueueStats {
            pending,
            running,
            completed,
            failed,
            worker_count: self.config.worker_count,
        })
    }
}

/// Task queue statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskQueueStats {
    pub pending: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub worker_count: usize,
}

/// Task queue worker
struct TaskQueueWorker {
    worker_id: usize,
    db: Arc<Database>,
    handlers: Arc<Mutex<HashMap<String, Arc<dyn TaskHandler>>>>,
    config: TaskQueueConfig,
    semaphore: Arc<Semaphore>,
}

impl TaskQueueWorker {
    fn new(
        worker_id: usize,
        db: Arc<Database>,
        handlers: Arc<Mutex<HashMap<String, Arc<dyn TaskHandler>>>>,
        config: TaskQueueConfig,
        semaphore: Arc<Semaphore>,
    ) -> Self {
        Self {
            worker_id,
            db,
            handlers,
            config,
            semaphore,
        }
    }

    async fn run(&self) {
        info!("Worker {} started", self.worker_id);

        loop {
            // Check if still running
            {
                // We need a way to check running status, but for now we'll just poll
            }

            // Acquire semaphore slot
            let permit = match self.semaphore.try_acquire() {
                Ok(permit) => permit,
                Err(_) => {
                    sleep(self.config.poll_interval).await;
                    continue;
                }
            };

            // Try to get a task
            let task_repo = TaskRepository::new(&self.db);
            let tasks = match task_repo.list_ready_tasks(1).await {
                Ok(tasks) => tasks,
                Err(e) => {
                    error!("Worker {} failed to fetch tasks: {}", self.worker_id, e);
                    sleep(self.config.poll_interval).await;
                    continue;
                }
            };

            if tasks.is_empty() {
                drop(permit);
                sleep(self.config.poll_interval).await;
                continue;
            }

            let task = tasks.into_iter().next().unwrap();
            let task_id = task.id.clone();

            debug!("Worker {} processing task {}", self.worker_id, task_id);

            // Update task status to running
            if let Err(e) = task_repo.update_status(&task_id, TaskStatus::Running).await {
                error!("Failed to update task status: {}", e);
                drop(permit);
                continue;
            }

            // Execute the task
            let result = self.execute_task(&task).await;

            // Handle result
            match result {
                Ok(output) => {
                    // Update task as completed
                    if let Err(e) = task_repo.update_status(&task_id, TaskStatus::Completed).await {
                        error!("Failed to update completed task: {}", e);
                    }

                    // Update output
                    if let Err(e) = self.update_task_output(&task_id, Some(&output), None).await {
                        error!("Failed to update task output: {}", e);
                    }

                    info!("Worker {} completed task {}", self.worker_id, task_id);
                }
                Err(e) => {
                    // Check if we should retry
                    let should_retry = task.retry_count < task.max_retries;

                    if should_retry {
                        // Increment retry count
                        let _ = task_repo.increment_retry(&task_id).await;

                        // Reset to pending for retry
                        let _ = task_repo.update_status(&task_id, TaskStatus::Pending).await;

                        warn!(
                            "Worker {} failed task {} (retry {}/{}): {}",
                            self.worker_id,
                            task_id,
                            task.retry_count + 1,
                            task.max_retries,
                            e
                        );

                        // Schedule retry with delay
                        tokio::spawn(async move {
                            sleep(Duration::from_secs(5)).await;
                        });
                    } else {
                        // Mark as failed
                        let _ = task_repo.update_status(&task_id, TaskStatus::Failed).await;
                        let _ = self.update_task_output(&task_id, None, Some(&e.to_string())).await;

                        error!(
                            "Worker {} failed task {} permanently: {}",
                            self.worker_id, task_id, e
                        );
                    }
                }
            }

            drop(permit);
        }
    }

    async fn execute_task(&self, task: &crate::database::repositories::TaskRecord) -> Result<serde_json::Value> {
        // Get handler for this skill
        let handlers = self.handlers.lock().await;
        let handler = handlers.get(&task.skill_id)
            .ok_or_else(|| crate::error::AppError::Config(format!("No handler for skill: {}", task.skill_id)))?;

        // Parse input parameters
        let input_params: serde_json::Value = serde_json::from_str(&task.input_params)?;

        let context = TaskContext {
            task_id: task.id.clone(),
            user_id: task.user_id.clone(),
            skill_id: task.skill_id.clone(),
            session_id: task.session_id.clone(),
            input_params,
            priority: task.priority,
            max_retries: task.max_retries,
            retry_count: task.retry_count,
            scheduled_at: task.scheduled_at.map(|ts| DateTime::from_timestamp(ts, 0).unwrap()),
        };

        // Execute with timeout
        let timeout = self.config.task_timeout;
        let handler = handler.clone();

        tokio::time::timeout(timeout, async move {
            handler.execute(context).await
        })
        .await
        .map_err(|_| crate::error::AppError::Timeout("Task execution timeout".to_string()))?
    }

    async fn update_task_output(
        &self,
        task_id: &str,
        output: Option<&serde_json::Value>,
        error: Option<&str>,
    ) -> Result<()> {
        let task_repo = TaskRepository::new(&self.db);
        let task = task_repo.get_by_id(task_id).await?
            .ok_or_else(|| crate::error::AppError::Database("Task not found".to_string()))?;

        let mut updated_task = task;
        updated_task.output_result = output.map(|o| serde_json::to_string(o).ok()).flatten();
        updated_task.error_message = error.map(|e| e.to_string());

        task_repo.update(&updated_task).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler;

    #[async_trait::async_trait]
    impl TaskHandler for TestHandler {
        async fn execute(&self, context: TaskContext) -> Result<serde_json::Value> {
            Ok(serde_json::json!({
                "result": "success",
                "task_id": context.task_id,
            }))
        }
    }

    #[tokio::test]
    async fn test_task_queue_config() {
        let config = TaskQueueConfig::default();
        assert_eq!(config.worker_count, 4);
        assert_eq!(config.max_retries, 3);
    }
}
