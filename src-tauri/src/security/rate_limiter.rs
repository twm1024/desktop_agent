// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

use crate::error::{AppError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Rate limit record
#[derive(Debug, Clone)]
struct RateLimitRecord {
    count: usize,
    window_start: Instant,
    blocked_until: Option<Instant>,
}

/// Rate limiter for protecting against abuse
pub struct RateLimiter {
    user_limits: Arc<RwLock<HashMap<String, RateLimitRecord>>>,
    ip_limits: Arc<RwLock<HashMap<String, RateLimitRecord>>>,
    user_limit: usize,
    ip_limit: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(user_limit: usize, ip_limit: usize, window: Duration) -> Self {
        Self {
            user_limits: Arc::new(RwLock::new(HashMap::new())),
            ip_limits: Arc::new(RwLock::new(HashMap::new())),
            user_limit,
            ip_limit,
            window,
        }
    }

    /// Check user rate limit
    pub async fn check_user(&self, user_id: &str) -> Result<bool> {
        self.check(&self.user_limits, user_id, self.user_limit).await
    }

    /// Check IP rate limit
    pub async fn check_ip(&self, ip: &str) -> Result<bool> {
        self.check(&self.ip_limits, ip, self.ip_limit).await
    }

    /// Check rate limit
    async fn check(
        &self,
        storage: &Arc<RwLock<HashMap<String, RateLimitRecord>>>,
        key: &str,
        limit: usize,
    ) -> Result<bool> {
        let mut limits = storage.write().await;
        let now = Instant::now();

        let record = limits.entry(key.to_string()).or_insert(RateLimitRecord {
            count: 0,
            window_start: now,
            blocked_until: None,
        });

        // Check if currently blocked
        if let Some(blocked_until) = record.blocked_until {
            if now < blocked_until {
                let remaining = (blocked_until - now).as_secs();
                warn!("Rate limit exceeded for {}, blocked for {}s", key, remaining);
                return Err(AppError::webhook(format!(
                    "Rate limit exceeded. Try again in {} seconds",
                    remaining
                )));
            } else {
                // Block expired, reset
                record.blocked_until = None;
            }
        }

        // Check if window expired
        if now.duration_since(record.window_start) > self.window {
            // Reset for new window
            record.count = 1;
            record.window_start = now;
            debug!("Rate limit window reset for {}", key);
            return Ok(true);
        }

        // Check limit
        if record.count >= limit {
            // Block for the duration of the window
            record.blocked_until = Some(now + self.window);
            warn!("Rate limit exceeded for {}", key);
            return Err(AppError::webhook("Rate limit exceeded"));
        }

        // Increment count
        record.count += 1;
        debug!(
            "Rate limit check passed for {}: {}/{}",
            key,
            record.count,
            limit
        );

        Ok(true)
    }

    /// Reset rate limit for a user
    pub async fn reset_user(&self, user_id: &str) {
        let mut limits = self.user_limits.write().await;
        limits.remove(user_id);
        debug!("Rate limit reset for user: {}", user_id);
    }

    /// Reset rate limit for an IP
    pub async fn reset_ip(&self, ip: &str) {
        let mut limits = self.ip_limits.write().await;
        limits.remove(ip);
        debug!("Rate limit reset for IP: {}", ip);
    }

    /// Get current rate limit status for a user
    pub async fn get_user_status(&self, user_id: &str) -> RateLimitStatus {
        let limits = self.user_limits.read().await;
        let now = Instant::now();

        limits
            .get(user_id)
            .map(|record| {
                let remaining = self.user_limit.saturating_sub(record.count);
                let reset_at = if now.duration_since(record.window_start) > self.window {
                    0
                } else {
                    self.window.saturating_sub(now.duration_since(record.window_start)).as_secs()
                };

                RateLimitStatus {
                    limit: self.user_limit,
                    remaining,
                    reset_at,
                    is_blocked: record.blocked_until.map_or(false, |t| now < t),
                }
            })
            .unwrap_or_else(|| RateLimitStatus {
                limit: self.user_limit,
                remaining: self.user_limit,
                reset_at: 0,
                is_blocked: false,
            })
    }

    /// Get current rate limit status for an IP
    pub async fn get_ip_status(&self, ip: &str) -> RateLimitStatus {
        let limits = self.ip_limits.read().await;
        let now = Instant::now();

        limits
            .get(ip)
            .map(|record| {
                let remaining = self.ip_limit.saturating_sub(record.count);
                let reset_at = if now.duration_since(record.window_start) > self.window {
                    0
                } else {
                    self.window.saturating_sub(now.duration_since(record.window_start)).as_secs()
                };

                RateLimitStatus {
                    limit: self.ip_limit,
                    remaining,
                    reset_at,
                    is_blocked: record.blocked_until.map_or(false, |t| now < t),
                }
            })
            .unwrap_or_else(|| RateLimitStatus {
                limit: self.ip_limit,
                remaining: self.ip_limit,
                reset_at: 0,
                is_blocked: false,
            })
    }

    /// Cleanup old entries
    pub async fn cleanup(&self, max_age: Duration) {
        let now = Instant::now();

        {
            let mut user_limits = self.user_limits.write().await;
            user_limits.retain(|_key, record| {
                now.duration_since(record.window_start) < max_age
            });
        }

        {
            let mut ip_limits = self.ip_limits.write().await;
            ip_limits.retain(|_key, record| {
                now.duration_since(record.window_start) < max_age
            });
        }

        debug!("Rate limiter cleanup completed");
    }
}

/// Rate limit status information
#[derive(Debug, Clone, serde::Serialize)]
pub struct RateLimitStatus {
    pub limit: usize,
    pub remaining: usize,
    pub reset_at: u64,
    pub is_blocked: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let limiter = RateLimiter::new(3, 5, Duration::from_secs(10));

        // First 3 requests should succeed
        for i in 0..3 {
            assert!(limiter.check_user("test_user").await.unwrap());
        }

        // 4th request should fail
        assert!(limiter.check_user("test_user").await.is_err());

        // Different user should still work
        assert!(limiter.check_user("other_user").await.unwrap());
    }

    #[tokio::test]
    async fn test_rate_limiter_window_reset() {
        let limiter = RateLimiter::new(2, 5, Duration::from_millis(100));

        // Use up the limit
        assert!(limiter.check_user("test_user").await.unwrap());
        assert!(limiter.check_user("test_user").await.unwrap());
        assert!(limiter.check_user("test_user").await.is_err());

        // Wait for window to expire
        sleep(Duration::from_millis(150)).await;

        // Should work again
        assert!(limiter.check_user("test_user").await.unwrap());
    }

    #[tokio::test]
    async fn test_rate_limiter_status() {
        let limiter = RateLimiter::new(5, 10, Duration::from_secs(60));

        // Use 2 requests
        limiter.check_user("test_user").await.unwrap();
        limiter.check_user("test_user").await.unwrap();

        let status = limiter.get_user_status("test_user").await;
        assert_eq!(status.limit, 5);
        assert_eq!(status.remaining, 3);
        assert!(!status.is_blocked);
    }

    #[tokio::test]
    async fn test_rate_limiter_reset() {
        let limiter = RateLimiter::new(2, 5, Duration::from_secs(60));

        // Use up the limit
        limiter.check_user("test_user").await.unwrap();
        limiter.check_user("test_user").await.unwrap();

        // Should be blocked
        assert!(limiter.check_user("test_user").await.is_err());

        // Reset
        limiter.reset_user("test_user").await;

        // Should work again
        assert!(limiter.check_user("test_user").await.unwrap());
    }
}
