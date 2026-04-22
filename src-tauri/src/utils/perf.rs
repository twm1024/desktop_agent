// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Performance monitoring and metrics collection
//!
//! Tracks operation metrics, response times, and system resource usage

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::debug;

/// Operation metrics for a single operation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationMetrics {
    pub name: String,
    pub total_calls: u64,
    pub success_count: u64,
    pub error_count: u64,
    pub total_duration_ms: u64,
    pub min_duration_ms: u64,
    pub max_duration_ms: u64,
    pub last_called_at: i64,
    pub avg_duration_ms: f64,
}

impl OperationMetrics {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            total_calls: 0,
            success_count: 0,
            error_count: 0,
            total_duration_ms: 0,
            min_duration_ms: u64::MAX,
            max_duration_ms: 0,
            last_called_at: 0,
            avg_duration_ms: 0.0,
        }
    }
}

/// A single recorded operation
struct OperationRecord {
    name: String,
    start: Instant,
}

/// Performance monitor for tracking application metrics
pub struct PerformanceMonitor {
    metrics: Arc<RwLock<HashMap<String, OperationMetrics>>>,
    active_operations: Arc<RwLock<HashMap<String, Instant>>>,
    enabled: bool,
}

impl PerformanceMonitor {
    pub fn new(enabled: bool) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            active_operations: Arc::new(RwLock::new(HashMap::new())),
            enabled,
        }
    }

    /// Start tracking an operation
    pub async fn start_operation(&self, name: &str) -> Option<String> {
        if !self.enabled {
            return None;
        }

        let op_id = uuid::Uuid::new_v4().to_string();
        let mut active = self.active_operations.write().await;
        active.insert(op_id.clone(), Instant::now());

        let mut metrics = self.metrics.write().await;
        metrics.entry(name.to_string())
            .or_insert_with(|| OperationMetrics::new(name));

        Some(op_id)
    }

    /// End tracking an operation (success)
    pub async fn end_operation(&self, op_id: &str, name: &str, success: bool) {
        if !self.enabled {
            return;
        }

        let duration = {
            let mut active = self.active_operations.write().await;
            active.remove(op_id).map(|start| start.elapsed())
        };

        if let Some(duration) = duration {
            let mut metrics = self.metrics.write().await;
            let entry = metrics.entry(name.to_string())
                .or_insert_with(|| OperationMetrics::new(name));

            let duration_ms = duration.as_millis() as u64;
            entry.total_calls += 1;
            entry.total_duration_ms += duration_ms;
            entry.last_called_at = crate::utils::current_timestamp();

            if success {
                entry.success_count += 1;
            } else {
                entry.error_count += 1;
            }

            if duration_ms < entry.min_duration_ms {
                entry.min_duration_ms = duration_ms;
            }
            if duration_ms > entry.max_duration_ms {
                entry.max_duration_ms = duration_ms;
            }

            entry.avg_duration_ms = entry.total_duration_ms as f64 / entry.total_calls as f64;
        }
    }

    /// Get metrics for a specific operation
    pub async fn get_metrics(&self, name: &str) -> Option<OperationMetrics> {
        let metrics = self.metrics.read().await;
        metrics.get(name).cloned()
    }

    /// Get all operation metrics
    pub async fn get_all_metrics(&self) -> Vec<OperationMetrics> {
        let metrics = self.metrics.read().await;
        metrics.values().cloned().collect()
    }

    /// Get performance summary
    pub async fn get_summary(&self) -> PerformanceSummary {
        let metrics = self.metrics.read().await;
        let total_operations: u64 = metrics.values().map(|m| m.total_calls).sum();
        let total_errors: u64 = metrics.values().map(|m| m.error_count).sum();
        let operation_count = metrics.len();

        PerformanceSummary {
            total_operations,
            total_errors,
            error_rate: if total_operations > 0 {
                total_errors as f64 / total_operations as f64 * 100.0
            } else {
                0.0
            },
            operation_types: operation_count,
            active_operations: self.active_operations.read().await.len(),
            uptime_secs: 0, // Set by caller
        }
    }

    /// Reset all metrics
    pub async fn reset(&self) {
        self.metrics.write().await.clear();
        self.active_operations.write().await.clear();
        debug!("Performance metrics reset");
    }
}

impl Clone for PerformanceMonitor {
    fn clone(&self) -> Self {
        Self {
            metrics: self.metrics.clone(),
            active_operations: self.active_operations.clone(),
            enabled: self.enabled,
        }
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new(true)
    }
}

/// Performance summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    pub total_operations: u64,
    pub total_errors: u64,
    pub error_rate: f64,
    pub operation_types: usize,
    pub active_operations: usize,
    pub uptime_secs: u64,
}

/// Scoped operation guard that automatically records timing
pub struct ScopedOperation {
    monitor: PerformanceMonitor,
    op_id: Option<String>,
    name: String,
    success: bool,
}

impl ScopedOperation {
    pub fn new(monitor: &PerformanceMonitor, name: &str) -> Self {
        let rt = tokio::runtime::Handle::current();
        let op_id = rt.block_on(monitor.start_operation(name));
        Self {
            monitor: monitor.clone(),
            op_id,
            name: name.to_string(),
            success: true,
        }
    }

    pub fn set_failed(&mut self) {
        self.success = false;
    }
}

impl Drop for ScopedOperation {
    fn drop(&mut self) {
        if let Some(op_id) = &self.op_id {
            let rt = tokio::runtime::Handle::current();
            let monitor = self.monitor.clone();
            let op_id = op_id.clone();
            let name = self.name.clone();
            let success = self.success;
            rt.spawn(async move {
                monitor.end_operation(&op_id, &name, success).await;
            });
        }
    }
}

/// Memory usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub usage_percent: f64,
    pub process_rss_bytes: u64,
}

impl MemoryInfo {
    /// Get current memory information
    pub fn current() -> Self {
        let mut total = 0u64;
        let mut available = 0u64;
        let mut rss = 0u64;

        #[cfg(unix)]
        {
            // Read system memory info from /proc/meminfo
            if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
                for line in content.lines() {
                    if line.starts_with("MemTotal:") {
                        total = line.split_whitespace()
                            .nth(1)
                            .and_then(|v| v.parse::<u64>().ok())
                            .unwrap_or(0) * 1024;
                    } else if line.starts_with("MemAvailable:") {
                        available = line.split_whitespace()
                            .nth(1)
                            .and_then(|v| v.parse::<u64>().ok())
                            .unwrap_or(0) * 1024;
                    }
                }
            }

            // Read process RSS from /proc/self/status
            if let Ok(content) = std::fs::read_to_string("/proc/self/status") {
                for line in content.lines() {
                    if line.starts_with("VmRSS:") {
                        rss = line.split_whitespace()
                            .nth(1)
                            .and_then(|v| v.parse::<u64>().ok())
                            .unwrap_or(0) * 1024;
                    }
                }
            }
        }

        let used = total.saturating_sub(available);
        let usage_percent = if total > 0 {
            used as f64 / total as f64 * 100.0
        } else {
            0.0
        };

        Self {
            total_bytes: total,
            used_bytes: used,
            available_bytes: available,
            usage_percent,
            process_rss_bytes: rss,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_performance_monitor() {
        let monitor = PerformanceMonitor::new(true);

        // Start and end an operation
        let op_id = monitor.start_operation("test_op").await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        monitor.end_operation(&op_id, "test_op", true).await;

        let metrics = monitor.get_metrics("test_op").await.unwrap();
        assert_eq!(metrics.total_calls, 1);
        assert_eq!(metrics.success_count, 1);
        assert!(metrics.avg_duration_ms >= 10.0);
    }

    #[tokio::test]
    async fn test_performance_summary() {
        let monitor = PerformanceMonitor::new(true);

        let op_id = monitor.start_operation("op1").await.unwrap();
        monitor.end_operation(&op_id, "op1", true).await;

        let op_id = monitor.start_operation("op2").await.unwrap();
        monitor.end_operation(&op_id, "op2", false).await;

        let summary = monitor.get_summary().await;
        assert_eq!(summary.total_operations, 2);
        assert_eq!(summary.total_errors, 1);
        assert_eq!(summary.operation_types, 2);
    }

    #[test]
    fn test_memory_info() {
        let info = MemoryInfo::current();
        // On systems without /proc/meminfo, values may be 0
        println!("Memory: total={}, used={}, rss={}",
            info.total_bytes, info.used_bytes, info.process_rss_bytes);
    }

    #[tokio::test]
    async fn test_disabled_monitor() {
        let monitor = PerformanceMonitor::new(false);
        let op_id = monitor.start_operation("test").await;
        assert!(op_id.is_none());
    }
}
