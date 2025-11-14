use crate::models::{ApiKey, ApiKeyInfo, AuthError, AuthResult};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use rand::{distributions::Alphanumeric, Rng};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

/// API key prefix for easy identification
const API_KEY_PREFIX: &str = "llm_orch_";

/// API key manager for creating and managing API keys
pub struct ApiKeyManager {
    /// Backend store for API keys
    store: Arc<dyn ApiKeyStore>,
}

impl ApiKeyManager {
    /// Create a new API key manager
    pub fn new(store: Arc<dyn ApiKeyStore>) -> Self {
        Self { store }
    }

    /// Create a new API key for a user
    ///
    /// # Arguments
    /// * `user_id` - The user ID who owns this key
    /// * `scopes` - Permissions/scopes for this key
    /// * `name` - Optional name for the key
    /// * `expires_in_days` - Optional expiration in days
    ///
    /// # Returns
    /// The created API key (including the raw key, shown only once)
    pub async fn create_key(
        &self,
        user_id: &str,
        scopes: Vec<String>,
        name: Option<String>,
        expires_in_days: Option<i64>,
    ) -> AuthResult<ApiKey> {
        // Generate a secure random key
        let raw_key = Self::generate_raw_key();
        let key_with_prefix = format!("{}{}", API_KEY_PREFIX, raw_key);

        // Hash the key for storage
        let key_hash = Self::hash_key(&key_with_prefix);

        // Calculate expiration
        let expires_at = expires_in_days.map(|days| Utc::now() + Duration::days(days));

        let api_key = ApiKey {
            id: Uuid::new_v4().to_string(),
            key: key_with_prefix.clone(),
            key_hash,
            user_id: user_id.to_string(),
            scopes,
            created_at: Utc::now(),
            expires_at,
            name,
        };

        self.store.create_key(&api_key).await?;

        Ok(api_key)
    }

    /// Lookup and validate an API key
    ///
    /// # Arguments
    /// * `key` - The raw API key to lookup
    ///
    /// # Returns
    /// API key information if valid and not expired
    pub async fn lookup_key(&self, key: &str) -> AuthResult<ApiKeyInfo> {
        let key_hash = Self::hash_key(key);
        let key_info = self
            .store
            .lookup_key(&key_hash)
            .await?
            .ok_or(AuthError::ApiKeyNotFound)?;

        // Check if expired
        if let Some(expires_at) = key_info.expires_at {
            if Utc::now() > expires_at {
                return Err(AuthError::ApiKeyExpired);
            }
        }

        // Update last used timestamp
        self.store.update_last_used(&key_info.id).await?;

        // Fetch updated info with new last_used_at timestamp
        self.store
            .lookup_key(&key_hash)
            .await?
            .ok_or(AuthError::ApiKeyNotFound)
    }

    /// Revoke an API key
    ///
    /// # Arguments
    /// * `key_id` - The ID of the key to revoke
    pub async fn revoke_key(&self, key_id: &str) -> AuthResult<()> {
        self.store.revoke_key(key_id).await
    }

    /// List all API keys for a user
    ///
    /// # Arguments
    /// * `user_id` - The user ID to list keys for
    pub async fn list_keys(&self, user_id: &str) -> AuthResult<Vec<ApiKeyInfo>> {
        self.store.list_keys(user_id).await
    }

    /// Generate a secure random API key
    fn generate_raw_key() -> String {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(48)
            .map(char::from)
            .collect()
    }

    /// Hash an API key using SHA-256
    fn hash_key(key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// Trait for API key storage backends
#[async_trait]
pub trait ApiKeyStore: Send + Sync {
    /// Create a new API key
    async fn create_key(&self, key: &ApiKey) -> AuthResult<()>;

    /// Lookup an API key by its hash
    async fn lookup_key(&self, key_hash: &str) -> AuthResult<Option<ApiKeyInfo>>;

    /// Revoke an API key
    async fn revoke_key(&self, key_id: &str) -> AuthResult<()>;

    /// List all API keys for a user
    async fn list_keys(&self, user_id: &str) -> AuthResult<Vec<ApiKeyInfo>>;

    /// Update last used timestamp
    async fn update_last_used(&self, key_id: &str) -> AuthResult<()>;
}

/// In-memory API key store (for testing and simple deployments)
pub struct InMemoryApiKeyStore {
    keys: Arc<dashmap::DashMap<String, ApiKeyInfo>>,
    user_keys: Arc<dashmap::DashMap<String, Vec<String>>>,
}

impl InMemoryApiKeyStore {
    pub fn new() -> Self {
        Self {
            keys: Arc::new(dashmap::DashMap::new()),
            user_keys: Arc::new(dashmap::DashMap::new()),
        }
    }
}

impl Default for InMemoryApiKeyStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ApiKeyStore for InMemoryApiKeyStore {
    async fn create_key(&self, key: &ApiKey) -> AuthResult<()> {
        let key_info = ApiKeyInfo {
            id: key.id.clone(),
            key_hash: key.key_hash.clone(),
            user_id: key.user_id.clone(),
            scopes: key.scopes.clone(),
            created_at: key.created_at,
            expires_at: key.expires_at,
            name: key.name.clone(),
            last_used_at: None,
        };

        self.keys.insert(key.key_hash.clone(), key_info);

        // Track user keys
        self.user_keys
            .entry(key.user_id.clone())
            .or_default()
            .push(key.id.clone());

        Ok(())
    }

    async fn lookup_key(&self, key_hash: &str) -> AuthResult<Option<ApiKeyInfo>> {
        Ok(self.keys.get(key_hash).map(|entry| entry.value().clone()))
    }

    async fn revoke_key(&self, key_id: &str) -> AuthResult<()> {
        // Find and remove the key
        self.keys.retain(|_k, v| v.id != key_id);

        // Remove from user keys
        for mut entry in self.user_keys.iter_mut() {
            entry.value_mut().retain(|id| id != key_id);
        }

        Ok(())
    }

    async fn list_keys(&self, user_id: &str) -> AuthResult<Vec<ApiKeyInfo>> {
        let key_ids = self
            .user_keys
            .get(user_id)
            .map(|entry| entry.value().clone())
            .unwrap_or_default();

        let mut keys = Vec::new();
        for entry in self.keys.iter() {
            if key_ids.contains(&entry.value().id) {
                keys.push(entry.value().clone());
            }
        }

        Ok(keys)
    }

    async fn update_last_used(&self, key_id: &str) -> AuthResult<()> {
        for mut entry in self.keys.iter_mut() {
            if entry.value().id == key_id {
                entry.value_mut().last_used_at = Some(Utc::now());
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_manager() -> ApiKeyManager {
        ApiKeyManager::new(Arc::new(InMemoryApiKeyStore::new()))
    }

    #[tokio::test]
    async fn test_create_api_key() {
        let manager = create_test_manager().await;

        let key = manager
            .create_key(
                "user123",
                vec!["workflow:read".to_string(), "workflow:execute".to_string()],
                Some("Test Key".to_string()),
                Some(30),
            )
            .await
            .unwrap();

        assert!(key.key.starts_with(API_KEY_PREFIX));
        assert_eq!(key.user_id, "user123");
        assert_eq!(key.scopes.len(), 2);
        assert_eq!(key.name, Some("Test Key".to_string()));
        assert!(key.expires_at.is_some());
    }

    #[tokio::test]
    async fn test_lookup_valid_key() {
        let manager = create_test_manager().await;

        let key = manager
            .create_key("user123", vec!["workflow:read".to_string()], None, None)
            .await
            .unwrap();

        let looked_up = manager.lookup_key(&key.key).await.unwrap();

        assert_eq!(looked_up.user_id, "user123");
        assert_eq!(looked_up.scopes, vec!["workflow:read"]);
        assert!(looked_up.last_used_at.is_some());
    }

    #[tokio::test]
    async fn test_lookup_invalid_key() {
        let manager = create_test_manager().await;

        let result = manager.lookup_key("invalid_key").await;
        assert!(matches!(result, Err(AuthError::ApiKeyNotFound)));
    }

    #[tokio::test]
    async fn test_lookup_expired_key() {
        let manager = create_test_manager().await;

        let key = manager
            .create_key("user123", vec!["workflow:read".to_string()], None, None)
            .await
            .unwrap();

        // Manually expire the key
        let store = Arc::new(InMemoryApiKeyStore::new());
        let expired_key = ApiKeyInfo {
            id: key.id.clone(),
            key_hash: key.key_hash.clone(),
            user_id: key.user_id.clone(),
            scopes: key.scopes.clone(),
            created_at: key.created_at,
            expires_at: Some(Utc::now() - Duration::days(1)),
            name: key.name.clone(),
            last_used_at: None,
        };

        let manager_with_expired = ApiKeyManager::new(store.clone());
        store
            .keys
            .insert(expired_key.key_hash.clone(), expired_key);

        let result = manager_with_expired.lookup_key(&key.key).await;
        assert!(matches!(result, Err(AuthError::ApiKeyExpired)));
    }

    #[tokio::test]
    async fn test_revoke_key() {
        let manager = create_test_manager().await;

        let key = manager
            .create_key("user123", vec!["workflow:read".to_string()], None, None)
            .await
            .unwrap();

        manager.revoke_key(&key.id).await.unwrap();

        let result = manager.lookup_key(&key.key).await;
        assert!(matches!(result, Err(AuthError::ApiKeyNotFound)));
    }

    #[tokio::test]
    async fn test_list_user_keys() {
        let manager = create_test_manager().await;

        manager
            .create_key("user123", vec!["workflow:read".to_string()], None, None)
            .await
            .unwrap();

        manager
            .create_key("user123", vec!["workflow:write".to_string()], None, None)
            .await
            .unwrap();

        manager
            .create_key("user456", vec!["workflow:read".to_string()], None, None)
            .await
            .unwrap();

        let user123_keys = manager.list_keys("user123").await.unwrap();
        assert_eq!(user123_keys.len(), 2);

        let user456_keys = manager.list_keys("user456").await.unwrap();
        assert_eq!(user456_keys.len(), 1);
    }

    #[tokio::test]
    async fn test_key_hash_consistency() {
        let key1 = "test_key_123";
        let hash1 = ApiKeyManager::hash_key(key1);
        let hash2 = ApiKeyManager::hash_key(key1);

        assert_eq!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_different_keys_different_hashes() {
        let hash1 = ApiKeyManager::hash_key("key1");
        let hash2 = ApiKeyManager::hash_key("key2");

        assert_ne!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_generated_keys_are_unique() {
        let key1 = ApiKeyManager::generate_raw_key();
        let key2 = ApiKeyManager::generate_raw_key();

        assert_ne!(key1, key2);
        assert_eq!(key1.len(), 48);
        assert_eq!(key2.len(), 48);
    }
}
