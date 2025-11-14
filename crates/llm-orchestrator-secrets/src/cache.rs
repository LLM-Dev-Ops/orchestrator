// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! In-memory secret cache with TTL.
//!
//! Provides a caching layer for secret stores to reduce backend calls
//! and improve performance.

use crate::models::{Secret, SecretMetadata, SecretVersion};
use crate::traits::{Result, SecretStore};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, trace};

/// Cached secret with expiration.
#[derive(Debug, Clone)]
struct CachedSecret {
    /// The cached secret.
    secret: Secret,
    /// When this cache entry expires.
    expires_at: DateTime<Utc>,
}

impl CachedSecret {
    /// Check if this cache entry is expired.
    fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }
}

/// Secret cache wrapper that adds TTL-based caching to any SecretStore.
///
/// # Features
///
/// - Configurable TTL (default: 300 seconds / 5 minutes)
/// - Thread-safe caching with read-write locks
/// - Automatic expiration checking
/// - Manual cache invalidation
/// - Cache statistics tracking
///
/// # Example
///
/// ```no_run
/// use llm_orchestrator_secrets::{SecretCache, EnvSecretStore};
/// use std::sync::Arc;
/// use chrono::Duration;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let backend = Arc::new(EnvSecretStore::new());
/// let cache = SecretCache::new(backend, Duration::minutes(5));
///
/// // First call hits the backend
/// let secret1 = cache.get("api_key").await?;
///
/// // Second call within TTL uses cache
/// let secret2 = cache.get("api_key").await?;
/// # Ok(())
/// # }
/// ```
pub struct SecretCache<S: SecretStore + ?Sized> {
    /// The underlying secret store backend.
    backend: Arc<S>,
    /// Cache storage.
    cache: Arc<RwLock<HashMap<String, CachedSecret>>>,
    /// Time-to-live for cached secrets.
    ttl: Duration,
    /// Cache statistics.
    stats: Arc<RwLock<CacheStats>>,
}

/// Cache statistics for monitoring.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Total number of cache hits.
    pub hits: u64,
    /// Total number of cache misses.
    pub misses: u64,
    /// Total number of expired entries encountered.
    pub expirations: u64,
    /// Total number of manual invalidations.
    pub invalidations: u64,
}

impl CacheStats {
    /// Calculate the cache hit rate as a percentage.
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }

    /// Get total number of cache accesses.
    pub fn total_accesses(&self) -> u64 {
        self.hits + self.misses
    }
}

impl<S: SecretStore + ?Sized> SecretCache<S> {
    /// Create a new secret cache.
    ///
    /// # Arguments
    ///
    /// * `backend` - The underlying secret store
    /// * `ttl` - Time-to-live for cached entries
    pub fn new(backend: Arc<S>, ttl: Duration) -> Self {
        debug!("Creating secret cache with TTL of {} seconds", ttl.num_seconds());
        Self {
            backend,
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl,
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Create a new secret cache with default TTL (5 minutes).
    pub fn with_default_ttl(backend: Arc<S>) -> Self {
        Self::new(backend, Duration::minutes(5))
    }

    /// Get a secret, using cache if available and not expired.
    pub async fn get(&self, key: &str) -> Result<Secret> {
        trace!("Cache lookup for key: {}", key);

        // Try to get from cache first
        {
            let cache_guard = self.cache.read();
            if let Some(cached) = cache_guard.get(key) {
                if !cached.is_expired() {
                    debug!("Cache hit for key: {}", key);
                    self.stats.write().hits += 1;
                    return Ok(cached.secret.clone());
                } else {
                    debug!("Cache entry expired for key: {}", key);
                    self.stats.write().expirations += 1;
                    // Entry is expired, fall through to fetch from backend
                }
            } else {
                debug!("Cache miss for key: {}", key);
                self.stats.write().misses += 1;
            }
        }

        // Not in cache or expired, fetch from backend
        let secret = self.backend.get_secret(key).await?;

        // Store in cache
        {
            let mut cache_guard = self.cache.write();
            let expires_at = Utc::now() + self.ttl;
            cache_guard.insert(
                key.to_string(),
                CachedSecret {
                    secret: secret.clone(),
                    expires_at,
                },
            );
            debug!("Cached secret {} until {}", key, expires_at);
        }

        Ok(secret)
    }

    /// Invalidate a specific cache entry.
    ///
    /// # Arguments
    ///
    /// * `key` - The secret key to invalidate
    pub fn invalidate(&self, key: &str) {
        let mut cache_guard = self.cache.write();
        if cache_guard.remove(key).is_some() {
            debug!("Invalidated cache entry for key: {}", key);
            self.stats.write().invalidations += 1;
        }
    }

    /// Clear all cached entries.
    pub fn clear(&self) {
        let mut cache_guard = self.cache.write();
        let count = cache_guard.len();
        cache_guard.clear();
        debug!("Cleared {} cache entries", count);
        self.stats.write().invalidations += count as u64;
    }

    /// Remove expired entries from the cache.
    ///
    /// This is useful for periodic cleanup to prevent memory growth.
    pub fn cleanup_expired(&self) {
        let mut cache_guard = self.cache.write();
        let before_count = cache_guard.len();
        cache_guard.retain(|key, cached| {
            let is_valid = !cached.is_expired();
            if !is_valid {
                trace!("Removing expired cache entry: {}", key);
            }
            is_valid
        });
        let removed = before_count - cache_guard.len();
        if removed > 0 {
            debug!("Cleaned up {} expired cache entries", removed);
            self.stats.write().expirations += removed as u64;
        }
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        self.stats.read().clone()
    }

    /// Get the number of entries currently in the cache.
    pub fn size(&self) -> usize {
        self.cache.read().len()
    }

    /// Get the TTL duration.
    pub fn ttl(&self) -> Duration {
        self.ttl
    }
}

#[async_trait]
impl<S: SecretStore + ?Sized> SecretStore for SecretCache<S> {
    async fn get_secret(&self, key: &str) -> Result<Secret> {
        self.get(key).await
    }

    async fn put_secret(
        &self,
        key: &str,
        value: &str,
        metadata: Option<SecretMetadata>,
    ) -> Result<()> {
        // Invalidate cache for this key
        self.invalidate(key);

        // Forward to backend
        self.backend.put_secret(key, value, metadata).await
    }

    async fn delete_secret(&self, key: &str) -> Result<()> {
        // Invalidate cache for this key
        self.invalidate(key);

        // Forward to backend
        self.backend.delete_secret(key).await
    }

    async fn list_secrets(&self, prefix: &str) -> Result<Vec<String>> {
        // List operations are not cached
        self.backend.list_secrets(prefix).await
    }

    async fn rotate_secret(&self, key: &str) -> Result<Secret> {
        // Invalidate cache for this key
        self.invalidate(key);

        // Forward to backend
        self.backend.rotate_secret(key).await
    }

    async fn health_check(&self) -> Result<()> {
        self.backend.health_check().await
    }

    async fn get_secret_versions(&self, key: &str) -> Result<Vec<SecretVersion>> {
        // Version listing is not cached
        self.backend.get_secret_versions(key).await
    }

    async fn get_secret_version(&self, key: &str, version: &str) -> Result<Secret> {
        // Versioned secrets are not cached (they are immutable)
        self.backend.get_secret_version(key, version).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::EnvSecretStore;
    use std::env;

    #[tokio::test]
    async fn test_cache_hit() {
        env::set_var("TEST_CACHE_KEY", "test_value");

        let backend = Arc::new(EnvSecretStore::new());
        let cache = SecretCache::new(backend, Duration::minutes(5));

        // First access - should be a miss
        let secret1 = cache.get("test/cache/key").await.unwrap();
        assert_eq!(secret1.value, "test_value");

        let stats1 = cache.stats();
        assert_eq!(stats1.misses, 1);
        assert_eq!(stats1.hits, 0);

        // Second access - should be a hit
        let secret2 = cache.get("test/cache/key").await.unwrap();
        assert_eq!(secret2.value, "test_value");

        let stats2 = cache.stats();
        assert_eq!(stats2.misses, 1);
        assert_eq!(stats2.hits, 1);

        env::remove_var("TEST_CACHE_KEY");
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        env::set_var("TEST_EXPIRE_KEY", "expire_value");

        let backend = Arc::new(EnvSecretStore::new());
        let cache = SecretCache::new(backend, Duration::milliseconds(100));

        // First access
        let _ = cache.get("test/expire/key").await.unwrap();

        // Wait for expiration
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        // Second access after expiration - should be treated as miss
        let _ = cache.get("test/expire/key").await.unwrap();

        let stats = cache.stats();
        assert_eq!(stats.misses, 2); // Both should be misses
        assert_eq!(stats.expirations, 1);

        env::remove_var("TEST_EXPIRE_KEY");
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        env::set_var("TEST_INVALIDATE_KEY", "invalidate_value");

        let backend = Arc::new(EnvSecretStore::new());
        let cache = SecretCache::new(backend, Duration::minutes(5));

        // First access
        let _ = cache.get("test/invalidate/key").await.unwrap();

        // Invalidate
        cache.invalidate("test/invalidate/key");

        // Second access - should be a miss due to invalidation
        let _ = cache.get("test/invalidate/key").await.unwrap();

        let stats = cache.stats();
        assert_eq!(stats.misses, 2);
        assert_eq!(stats.invalidations, 1);

        env::remove_var("TEST_INVALIDATE_KEY");
    }

    #[tokio::test]
    async fn test_cache_clear() {
        env::set_var("TEST_CLEAR_KEY1", "value1");
        env::set_var("TEST_CLEAR_KEY2", "value2");

        let backend = Arc::new(EnvSecretStore::new());
        let cache = SecretCache::new(backend, Duration::minutes(5));

        // Access multiple keys
        let _ = cache.get("test/clear/key1").await.unwrap();
        let _ = cache.get("test/clear/key2").await.unwrap();

        assert_eq!(cache.size(), 2);

        // Clear cache
        cache.clear();

        assert_eq!(cache.size(), 0);

        let stats = cache.stats();
        assert_eq!(stats.invalidations, 2);

        env::remove_var("TEST_CLEAR_KEY1");
        env::remove_var("TEST_CLEAR_KEY2");
    }

    #[tokio::test]
    async fn test_cache_stats_hit_rate() {
        env::set_var("TEST_STATS_KEY", "stats_value");

        let backend = Arc::new(EnvSecretStore::new());
        let cache = SecretCache::new(backend, Duration::minutes(5));

        // 1 miss
        let _ = cache.get("test/stats/key").await.unwrap();
        // 3 hits
        let _ = cache.get("test/stats/key").await.unwrap();
        let _ = cache.get("test/stats/key").await.unwrap();
        let _ = cache.get("test/stats/key").await.unwrap();

        let stats = cache.stats();
        assert_eq!(stats.total_accesses(), 4);
        assert_eq!(stats.hit_rate(), 75.0); // 3 hits out of 4 total

        env::remove_var("TEST_STATS_KEY");
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        env::set_var("TEST_CLEANUP_KEY1", "value1");
        env::set_var("TEST_CLEANUP_KEY2", "value2");

        let backend = Arc::new(EnvSecretStore::new());
        let cache = SecretCache::new(backend, Duration::milliseconds(100));

        // Access keys
        let _ = cache.get("test/cleanup/key1").await.unwrap();
        let _ = cache.get("test/cleanup/key2").await.unwrap();

        assert_eq!(cache.size(), 2);

        // Wait for expiration
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        // Cleanup expired entries
        cache.cleanup_expired();

        assert_eq!(cache.size(), 0);

        env::remove_var("TEST_CLEANUP_KEY1");
        env::remove_var("TEST_CLEANUP_KEY2");
    }
}
