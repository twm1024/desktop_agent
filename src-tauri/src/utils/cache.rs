// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! In-memory cache with TTL support
//!
//! Provides a thread-safe, typed cache with automatic expiration

#![allow(dead_code)]
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::debug;

/// Cache entry with TTL
#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,
    expires_at: Option<Instant>,
}

impl<V> CacheEntry<V> {
    fn new(value: V, ttl: Option<Duration>) -> Self {
        Self {
            value,
            expires_at: ttl.map(|d| Instant::now() + d),
        }
    }

    fn is_expired(&self) -> bool {
        self.expires_at.map_or(false, |t| Instant::now() > t)
    }
}

/// Thread-safe in-memory cache with TTL support
pub struct Cache<K, V> {
    entries: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    default_ttl: Option<Duration>,
    max_entries: usize,
}

impl<K, V> Cache<K, V>
where
    K: std::hash::Hash + Eq + Clone + std::fmt::Debug,
    V: Clone,
{
    /// Create a new cache with optional default TTL
    pub fn new(default_ttl: Option<Duration>, max_entries: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            default_ttl,
            max_entries,
        }
    }

    /// Create a cache with no TTL (entries never expire)
    pub fn unbounded(max_entries: usize) -> Self {
        Self::new(None, max_entries)
    }

    /// Insert a value with default TTL
    pub async fn insert(&self, key: K, value: V) {
        self.insert_with_ttl(key, value, self.default_ttl).await;
    }

    /// Insert a value with custom TTL
    pub async fn insert_with_ttl(&self, key: K, value: V, ttl: Option<Duration>) {
        let mut entries = self.entries.write().await;

        // Evict if at capacity
        if entries.len() >= self.max_entries && !entries.contains_key(&key) {
            self.evict_oldest(&mut entries);
        }

        entries.insert(key.clone(), CacheEntry::new(value, ttl));
        debug!("Cache insert: {:?}", key);
    }

    /// Get a value from cache
    pub async fn get(&self, key: &K) -> Option<V> {
        let entries = self.entries.read().await;
        entries.get(key).and_then(|entry| {
            if entry.is_expired() {
                None
            } else {
                Some(entry.value.clone())
            }
        })
    }

    /// Get or compute a value
    pub async fn get_or_insert_with<F>(&self, key: K, f: F) -> V
    where
        F: FnOnce() -> V,
    {
        if let Some(value) = self.get(&key).await {
            return value;
        }

        let value = f();
        self.insert(key, value.clone()).await;
        value
    }

    /// Get or asynchronously compute a value
    pub async fn get_or_try_insert_with<F, Fut, E>(&self, key: K, f: F) -> std::result::Result<V, E>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = std::result::Result<V, E>>,
    {
        if let Some(value) = self.get(&key).await {
            return Ok(value);
        }

        let value = f().await?;
        self.insert(key, value.clone()).await;
        Ok(value)
    }

    /// Remove a key from cache
    pub async fn remove(&self, key: &K) -> Option<V> {
        let mut entries = self.entries.write().await;
        entries.remove(key).map(|e| e.value)
    }

    /// Check if a key exists and is not expired
    pub async fn contains(&self, key: &K) -> bool {
        self.get(key).await.is_some()
    }

    /// Clear all entries
    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
        debug!("Cache cleared");
    }

    /// Remove expired entries
    pub async fn cleanup(&self) {
        let mut entries = self.entries.write().await;
        let before = entries.len();
        entries.retain(|_, entry| !entry.is_expired());
        let removed = before - entries.len();
        if removed > 0 {
            debug!("Cache cleanup: removed {} expired entries", removed);
        }
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let entries = self.entries.read().await;
        let total = entries.len();
        let expired = entries.values().filter(|e| e.is_expired()).count();
        CacheStats {
            total,
            active: total - expired,
            expired,
            max_entries: self.max_entries,
        }
    }

    fn evict_oldest(&self, entries: &mut HashMap<K, CacheEntry<V>>) {
        // Simple eviction: remove expired entries first, then remove first entry
        let expired_keys: Vec<K> = entries
            .iter()
            .filter(|(_, v)| v.is_expired())
            .map(|(k, _)| k.clone())
            .collect();

        if !expired_keys.is_empty() {
            for key in expired_keys {
                entries.remove(&key);
            }
            return;
        }

        // Remove the first entry as a fallback
        if let Some(key) = entries.keys().next().cloned() {
            entries.remove(&key);
        }
    }
}

impl<K, V> Clone for Cache<K, V> {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
            default_ttl: self.default_ttl,
            max_entries: self.max_entries,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheStats {
    pub total: usize,
    pub active: usize,
    pub expired: usize,
    pub max_entries: usize,
}

/// Multi-cache manager for different data types
pub struct CacheManager {
    /// Skill manifest cache
    pub skill_manifests: Cache<String, serde_json::Value>,
    /// Platform token cache
    pub platform_tokens: Cache<String, String>,
    /// User info cache
    pub user_info: Cache<String, serde_json::Value>,
    /// System info cache
    pub system_info: Cache<String, serde_json::Value>,
    /// General purpose cache
    pub general: Cache<String, serde_json::Value>,
}

impl CacheManager {
    pub fn new() -> Self {
        Self {
            skill_manifests: Cache::new(Some(Duration::from_secs(300)), 200),
            platform_tokens: Cache::new(Some(Duration::from_secs(7000)), 20),
            user_info: Cache::new(Some(Duration::from_secs(600)), 500),
            system_info: Cache::new(Some(Duration::from_secs(60)), 10),
            general: Cache::new(Some(Duration::from_secs(180)), 1000),
        }
    }

    /// Cleanup all caches
    pub async fn cleanup_all(&self) {
        self.skill_manifests.cleanup().await;
        self.platform_tokens.cleanup().await;
        self.user_info.cleanup().await;
        self.system_info.cleanup().await;
        self.general.cleanup().await;
    }

    /// Get stats for all caches
    pub async fn all_stats(&self) -> HashMap<String, CacheStats> {
        let mut stats = HashMap::new();
        stats.insert("skill_manifests".to_string(), self.skill_manifests.stats().await);
        stats.insert("platform_tokens".to_string(), self.platform_tokens.stats().await);
        stats.insert("user_info".to_string(), self.user_info.stats().await);
        stats.insert("system_info".to_string(), self.system_info.stats().await);
        stats.insert("general".to_string(), self.general.stats().await);
        stats
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_cache_insert_get() {
        let cache: Cache<String, String> = Cache::new(None, 100);
        cache.insert("key1".to_string(), "value1".to_string()).await;

        assert_eq!(cache.get(&"key1".to_string()).await, Some("value1".to_string()));
        assert_eq!(cache.get(&"key2".to_string()).await, None);
    }

    #[tokio::test]
    async fn test_cache_ttl() {
        let cache: Cache<String, String> = Cache::new(Some(Duration::from_millis(50)), 100);
        cache.insert("key1".to_string(), "value1".to_string()).await;

        assert_eq!(cache.get(&"key1".to_string()).await, Some("value1".to_string()));

        sleep(Duration::from_millis(100)).await;
        assert_eq!(cache.get(&"key1".to_string()).await, None);
    }

    #[tokio::test]
    async fn test_cache_get_or_insert() {
        let cache: Cache<String, String> = Cache::new(None, 100);
        let value = cache.get_or_insert_with("key1".to_string(), || "computed".to_string()).await;
        assert_eq!(value, "computed");

        // Second call should use cache
        let value = cache.get_or_insert_with("key1".to_string(), || "other".to_string()).await;
        assert_eq!(value, "computed");
    }

    #[tokio::test]
    async fn test_cache_eviction() {
        let cache: Cache<String, String> = Cache::new(None, 2);
        cache.insert("a".to_string(), "1".to_string()).await;
        cache.insert("b".to_string(), "2".to_string()).await;
        cache.insert("c".to_string(), "3".to_string()).await;

        let stats = cache.stats().await;
        assert!(stats.total <= 2);
    }

    #[tokio::test]
    async fn test_cache_cleanup() {
        let cache: Cache<String, String> = Cache::new(Some(Duration::from_millis(50)), 100);
        cache.insert("key1".to_string(), "value1".to_string()).await;
        cache.insert("key2".to_string(), "value2".to_string()).await;

        sleep(Duration::from_millis(100)).await;
        cache.cleanup().await;

        let stats = cache.stats().await;
        assert_eq!(stats.active, 0);
    }

    #[tokio::test]
    async fn test_cache_manager() {
        let manager = CacheManager::new();
        manager.skill_manifests.insert("test".to_string(), serde_json::json!({"name": "test"})).await;

        let value = manager.skill_manifests.get(&"test".to_string()).await;
        assert!(value.is_some());
    }
}
