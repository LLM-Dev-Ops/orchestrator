// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration tests for secret management.

use chrono::Duration;
use llm_orchestrator_secrets::{
    EnvSecretStore, SecretCache, SecretManagerBuilder, SecretStore, SecretStoreType,
};
use std::env;
use std::sync::Arc;

#[tokio::test]
async fn test_env_store_basic_operations() {
    // Set up test environment variables
    env::set_var("TEST_API_KEY", "test_secret_value");
    env::set_var("TEST_DB_PASSWORD", "db_pass_123");

    let store = EnvSecretStore::new();

    // Test get_secret
    let secret = store.get_secret("test/api/key").await;
    assert!(secret.is_ok());
    let secret = secret.unwrap();
    assert_eq!(secret.value, "test_secret_value");
    assert_eq!(secret.key, "test/api/key");

    // Test metadata
    assert_eq!(
        secret.metadata.get("source"),
        Some(&"environment".to_string())
    );

    // Test not found
    let result = store.get_secret("nonexistent/key").await;
    assert!(result.is_err());

    // Test operations not supported
    assert!(store.put_secret("test/key", "value", None).await.is_err());
    assert!(store.delete_secret("test/key").await.is_err());
    assert!(store.list_secrets("test").await.is_err());
    assert!(store.rotate_secret("test/key").await.is_err());

    // Test health check
    assert!(store.health_check().await.is_ok());

    // Cleanup
    env::remove_var("TEST_API_KEY");
    env::remove_var("TEST_DB_PASSWORD");
}

#[tokio::test]
async fn test_env_store_with_prefix() {
    env::set_var("APP_DATABASE_URL", "postgres://localhost");

    let store = EnvSecretStore::with_prefix("APP_".to_string());

    let secret = store.get_secret("database/url").await;
    assert!(secret.is_ok());
    assert_eq!(secret.unwrap().value, "postgres://localhost");

    env::remove_var("APP_DATABASE_URL");
}

#[tokio::test]
async fn test_cache_basic_functionality() {
    env::set_var("CACHE_TEST_KEY", "cached_value");

    let backend = Arc::new(EnvSecretStore::new());
    let cache = SecretCache::new(backend, Duration::minutes(5));

    // First access - cache miss
    let secret1 = cache.get("cache/test/key").await.unwrap();
    assert_eq!(secret1.value, "cached_value");

    let stats1 = cache.stats();
    assert_eq!(stats1.misses, 1);
    assert_eq!(stats1.hits, 0);

    // Second access - cache hit
    let secret2 = cache.get("cache/test/key").await.unwrap();
    assert_eq!(secret2.value, "cached_value");

    let stats2 = cache.stats();
    assert_eq!(stats2.hits, 1);
    assert_eq!(stats2.misses, 1);

    // Test cache size
    assert_eq!(cache.size(), 1);

    // Test invalidation
    cache.invalidate("cache/test/key");
    assert_eq!(cache.size(), 0);

    env::remove_var("CACHE_TEST_KEY");
}

#[tokio::test]
async fn test_cache_expiration() {
    env::set_var("EXPIRE_TEST_KEY", "expire_value");

    let backend = Arc::new(EnvSecretStore::new());
    let cache = SecretCache::new(backend, Duration::milliseconds(100));

    // First access
    let _ = cache.get("expire/test/key").await.unwrap();
    assert_eq!(cache.size(), 1);

    // Wait for expiration
    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    // Access again - should fetch from backend due to expiration
    let _ = cache.get("expire/test/key").await.unwrap();

    let stats = cache.stats();
    // Both accesses should be misses (second one due to expiration)
    assert_eq!(stats.misses, 2);
    assert_eq!(stats.expirations, 1);

    env::remove_var("EXPIRE_TEST_KEY");
}

#[tokio::test]
async fn test_cache_cleanup() {
    env::set_var("CLEANUP_KEY1", "value1");
    env::set_var("CLEANUP_KEY2", "value2");
    env::set_var("CLEANUP_KEY3", "value3");

    let backend = Arc::new(EnvSecretStore::new());
    let cache = SecretCache::new(backend, Duration::milliseconds(100));

    // Access multiple keys
    let _ = cache.get("cleanup/key1").await.unwrap();
    let _ = cache.get("cleanup/key2").await.unwrap();
    let _ = cache.get("cleanup/key3").await.unwrap();

    assert_eq!(cache.size(), 3);

    // Wait for expiration
    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    // Cleanup expired entries
    cache.cleanup_expired();
    assert_eq!(cache.size(), 0);

    env::remove_var("CLEANUP_KEY1");
    env::remove_var("CLEANUP_KEY2");
    env::remove_var("CLEANUP_KEY3");
}

#[tokio::test]
async fn test_cache_stats() {
    env::set_var("STATS_KEY", "stats_value");

    let backend = Arc::new(EnvSecretStore::new());
    let cache = SecretCache::new(backend, Duration::minutes(5));

    // 1 miss, 4 hits
    let _ = cache.get("stats/key").await.unwrap();
    let _ = cache.get("stats/key").await.unwrap();
    let _ = cache.get("stats/key").await.unwrap();
    let _ = cache.get("stats/key").await.unwrap();
    let _ = cache.get("stats/key").await.unwrap();

    let stats = cache.stats();
    assert_eq!(stats.total_accesses(), 5);
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.hits, 4);
    assert_eq!(stats.hit_rate(), 80.0);

    env::remove_var("STATS_KEY");
}

#[tokio::test]
async fn test_builder_environment_store() {
    env::set_var("BUILDER_TEST_KEY", "builder_value");

    let store = SecretManagerBuilder::new(SecretStoreType::Environment)
        .build()
        .await
        .unwrap();

    let secret = store.get_secret("builder/test/key").await.unwrap();
    assert_eq!(secret.value, "builder_value");

    env::remove_var("BUILDER_TEST_KEY");
}

#[tokio::test]
async fn test_builder_with_cache() {
    env::set_var("BUILDER_CACHE_KEY", "cached_builder_value");

    let store = SecretManagerBuilder::new(SecretStoreType::Environment)
        .with_cache(Duration::minutes(10))
        .build()
        .await
        .unwrap();

    // First access
    let secret1 = store.get_secret("builder/cache/key").await.unwrap();
    assert_eq!(secret1.value, "cached_builder_value");

    // Second access - should be cached
    let secret2 = store.get_secret("builder/cache/key").await.unwrap();
    assert_eq!(secret2.value, "cached_builder_value");

    env::remove_var("BUILDER_CACHE_KEY");
}

#[tokio::test]
async fn test_builder_env_convenience() {
    env::set_var("CONVENIENCE_KEY", "convenience_value");

    let store = SecretManagerBuilder::build_env(None).await.unwrap();

    let secret = store.get_secret("convenience/key").await.unwrap();
    assert_eq!(secret.value, "convenience_value");

    env::remove_var("CONVENIENCE_KEY");
}

#[tokio::test]
async fn test_builder_env_with_prefix() {
    env::set_var("APP_PREFIX_KEY", "prefix_value");

    let store = SecretManagerBuilder::build_env(Some("APP_".to_string()))
        .await
        .unwrap();

    let secret = store.get_secret("prefix/key").await.unwrap();
    assert_eq!(secret.value, "prefix_value");

    env::remove_var("APP_PREFIX_KEY");
}

#[tokio::test]
async fn test_secret_metadata() {
    env::set_var("METADATA_KEY", "metadata_value");

    let store = EnvSecretStore::new();
    let secret = store.get_secret("metadata/key").await.unwrap();

    // Check metadata
    assert!(secret.metadata.contains_key("source"));
    assert!(secret.metadata.contains_key("env_var"));
    assert_eq!(secret.metadata.get("source").unwrap(), "environment");
    assert_eq!(secret.metadata.get("env_var").unwrap(), "METADATA_KEY");

    env::remove_var("METADATA_KEY");
}

#[tokio::test]
async fn test_multiple_keys_with_cache() {
    env::set_var("MULTI_KEY1", "value1");
    env::set_var("MULTI_KEY2", "value2");
    env::set_var("MULTI_KEY3", "value3");

    let backend = Arc::new(EnvSecretStore::new());
    let cache = SecretCache::new(backend, Duration::minutes(5));

    // Access multiple different keys
    let secret1 = cache.get("multi/key1").await.unwrap();
    let secret2 = cache.get("multi/key2").await.unwrap();
    let secret3 = cache.get("multi/key3").await.unwrap();

    assert_eq!(secret1.value, "value1");
    assert_eq!(secret2.value, "value2");
    assert_eq!(secret3.value, "value3");

    // All should be cached
    assert_eq!(cache.size(), 3);

    // Access again - all should be cache hits
    let _ = cache.get("multi/key1").await.unwrap();
    let _ = cache.get("multi/key2").await.unwrap();
    let _ = cache.get("multi/key3").await.unwrap();

    let stats = cache.stats();
    assert_eq!(stats.misses, 3);
    assert_eq!(stats.hits, 3);

    // Clear cache
    cache.clear();
    assert_eq!(cache.size(), 0);

    env::remove_var("MULTI_KEY1");
    env::remove_var("MULTI_KEY2");
    env::remove_var("MULTI_KEY3");
}

#[tokio::test]
async fn test_empty_env_var_error() {
    env::set_var("EMPTY_KEY", "");

    let store = EnvSecretStore::new();
    let result = store.get_secret("empty/key").await;

    assert!(result.is_err());
    // Should return NotFound error for empty values
    assert!(result.is_err());

    env::remove_var("EMPTY_KEY");
}

#[tokio::test]
async fn test_health_check_always_ok() {
    let store = EnvSecretStore::new();

    // Health check should always succeed for env store
    let result = store.health_check().await;
    assert!(result.is_ok());
}
