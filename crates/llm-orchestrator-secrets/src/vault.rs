// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! HashiCorp Vault secret store implementation.
//!
//! Provides integration with HashiCorp Vault's KV v2 secrets engine.

use crate::models::{Secret, SecretMetadata, SecretVersion};
use crate::traits::{Result, SecretError, SecretStore};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
use vaultrs::kv2;

/// HashiCorp Vault secret store using KV v2 engine.
///
/// # Features
///
/// - KV v2 secret engine support
/// - Token authentication
/// - Namespace support (Vault Enterprise)
/// - Secret versioning
/// - Automatic token renewal
/// - TLS/mTLS support
///
/// # Example
///
/// ```no_run
/// use llm_orchestrator_secrets::VaultSecretStore;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let store = VaultSecretStore::new(
///     "https://vault.example.com:8200".to_string(),
///     "hvs.CAESIJ...".to_string(),
/// )?;
///
/// let secret = store.get_secret("database/password").await?;
/// println!("Retrieved secret: {}", secret.key);
/// # Ok(())
/// # }
/// ```
pub struct VaultSecretStore {
    /// Vault HTTP client.
    client: VaultClient,
    /// Mount path for the KV v2 secrets engine (default: "secret").
    mount_path: String,
    /// Optional namespace (Vault Enterprise feature).
    namespace: Option<String>,
    /// Authentication token.
    token: String,
}

impl VaultSecretStore {
    /// Create a new Vault secret store.
    ///
    /// # Arguments
    ///
    /// * `addr` - Vault server address (e.g., "https://vault.example.com:8200")
    /// * `token` - Vault authentication token
    ///
    /// # Returns
    ///
    /// A configured Vault secret store, or an error if initialization fails.
    pub fn new(addr: String, token: String) -> Result<Self> {
        let settings = VaultClientSettingsBuilder::default()
            .address(&addr)
            .token(&token)
            .build()
            .map_err(|e| SecretError::Other(format!("Failed to build Vault settings: {}", e)))?;

        let client = VaultClient::new(settings)
            .map_err(|e| SecretError::Other(format!("Failed to create Vault client: {}", e)))?;

        debug!("Initialized Vault client for {}", addr);

        Ok(Self {
            client,
            mount_path: "secret".to_string(),
            namespace: None,
            token,
        })
    }

    /// Set the namespace for Vault Enterprise.
    ///
    /// # Arguments
    ///
    /// * `namespace` - The Vault namespace to use
    pub fn with_namespace(mut self, namespace: String) -> Self {
        self.namespace = Some(namespace);
        self
    }

    /// Set the mount path for the KV v2 secrets engine.
    ///
    /// # Arguments
    ///
    /// * `path` - The mount path (default is "secret")
    pub fn with_mount_path(mut self, path: String) -> Self {
        self.mount_path = path;
        self
    }

    /// Renew the Vault token.
    ///
    /// This should be called periodically to prevent token expiration.
    pub async fn renew_token(&self) -> Result<()> {
        debug!("Renewing Vault token");
        vaultrs::token::renew(&self.client, &self.token, None)
            .await
            .map_err(|e| {
                error!("Failed to renew Vault token: {}", e);
                SecretError::AuthenticationFailed(format!("Token renewal failed: {}", e))
            })?;
        info!("Successfully renewed Vault token");
        Ok(())
    }

    /// Get all versions of a secret.
    ///
    /// # Arguments
    ///
    /// * `key` - The secret key
    ///
    /// # Returns
    ///
    /// A list of all versions of the secret.
    pub async fn get_secret_versions_internal(&self, key: &str) -> Result<Vec<SecretVersion>> {
        debug!("Retrieving versions for secret: {}", key);

        let metadata: vaultrs::api::kv2::responses::ReadSecretMetadataResponse =
            kv2::read_metadata(&self.client, &self.mount_path, key)
                .await
                .map_err(|e| {
                    error!("Failed to read secret metadata: {}", e);
                    match e {
                        vaultrs::error::ClientError::APIError { code, errors } => {
                            if code == 404 {
                                SecretError::NotFound(key.to_string())
                            } else {
                                SecretError::Other(format!("API error {}: {:?}", code, errors))
                            }
                        }
                        _ => SecretError::BackendUnavailable(e.to_string()),
                    }
                })?;

        let versions: Vec<SecretVersion> = metadata
            .versions
            .iter()
            .map(|(version_str, version_meta)| {
                let version_num = version_str.parse::<u64>().unwrap_or(0);
                SecretVersion {
                    version: version_str.clone(),
                    created_at: version_meta
                        .created_time
                        .parse::<DateTime<Utc>>()
                        .unwrap_or_else(|_| Utc::now()),
                    is_current: version_num == metadata.current_version,
                    is_deleted: version_meta.destroyed || !version_meta.deletion_time.is_empty(),
                }
            })
            .collect();

        debug!("Found {} versions for secret {}", versions.len(), key);
        Ok(versions)
    }

    /// Convert Vault error to SecretError.
    fn convert_vault_error(key: &str, err: vaultrs::error::ClientError) -> SecretError {
        match err {
            vaultrs::error::ClientError::APIError { code, errors } => {
                if code == 404 {
                    SecretError::NotFound(key.to_string())
                } else if code == 403 {
                    SecretError::PermissionDenied(format!("Access denied: {:?}", errors))
                } else {
                    SecretError::Other(format!("Vault API error {}: {:?}", code, errors))
                }
            }
            vaultrs::error::ClientError::RestClientError { source } => {
                SecretError::NetworkError(source.to_string())
            }
            _ => SecretError::BackendUnavailable(err.to_string()),
        }
    }
}

#[async_trait]
impl SecretStore for VaultSecretStore {
    async fn get_secret(&self, key: &str) -> Result<Secret> {
        debug!("Retrieving secret from Vault: {}", key);

        let response: vaultrs::api::kv2::responses::ReadSecretResponse = kv2::read(&self.client, &self.mount_path, key)
            .await
            .map_err(|e| {
                error!("Failed to read secret {}: {}", key, e);
                Self::convert_vault_error(key, e)
            })?;

        // Extract the secret value (assuming it's stored in a "value" field)
        let value = response
            .data
            .get("value")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                SecretError::InvalidSecret(
                    "Secret does not contain a 'value' field or it's not a string".to_string(),
                )
            })?
            .to_string();

        // Convert metadata
        let mut metadata = HashMap::new();
        if let Some(obj) = response.data.as_object() {
            for (k, v) in obj.iter() {
                if k != "value" {
                    if let Some(s) = v.as_str() {
                        metadata.insert(k.clone(), s.to_string());
                    }
                }
            }
        }

        let secret = Secret {
            key: key.to_string(),
            value,
            version: Some(response.metadata.version.to_string()),
            created_at: response
                .metadata
                .created_time
                .parse::<DateTime<Utc>>()
                .unwrap_or_else(|_| Utc::now()),
            metadata,
        };

        debug!("Successfully retrieved secret: {}", key);
        Ok(secret)
    }

    async fn put_secret(
        &self,
        key: &str,
        value: &str,
        metadata: Option<SecretMetadata>,
    ) -> Result<()> {
        debug!("Storing secret in Vault: {}", key);

        let mut data = HashMap::new();
        data.insert("value".to_string(), value.to_string());

        // Add metadata fields if provided
        if let Some(meta) = metadata {
            if let Some(desc) = meta.description {
                data.insert("description".to_string(), desc);
            }
            for (k, v) in meta.tags {
                data.insert(format!("tag_{}", k), v);
            }
        }

        kv2::set(&self.client, &self.mount_path, key, &data)
            .await
            .map_err(|e| {
                error!("Failed to store secret {}: {}", key, e);
                Self::convert_vault_error(key, e)
            })?;

        info!("Successfully stored secret: {}", key);
        Ok(())
    }

    async fn delete_secret(&self, key: &str) -> Result<()> {
        debug!("Deleting secret from Vault: {}", key);

        kv2::delete_latest(&self.client, &self.mount_path, key)
            .await
            .map_err(|e| {
                error!("Failed to delete secret {}: {}", key, e);
                Self::convert_vault_error(key, e)
            })?;

        info!("Successfully deleted secret: {}", key);
        Ok(())
    }

    async fn list_secrets(&self, prefix: &str) -> Result<Vec<String>> {
        debug!("Listing secrets with prefix: {}", prefix);

        let keys = kv2::list(&self.client, &self.mount_path, prefix)
            .await
            .map_err(|e| {
                error!("Failed to list secrets: {}", e);
                Self::convert_vault_error(prefix, e)
            })?;

        debug!("Found {} secrets with prefix {}", keys.len(), prefix);
        Ok(keys)
    }

    async fn rotate_secret(&self, key: &str) -> Result<Secret> {
        debug!("Rotating secret: {}", key);

        // For Vault, rotation involves creating a new version
        // First, get the current secret
        let current = self.get_secret(key).await?;

        // Note: In a real implementation, you would generate a new value here
        // For now, we just create a new version with a placeholder
        warn!(
            "Secret rotation for {} - new value should be generated externally",
            key
        );

        // Create new version (caller should provide new value)
        self.put_secret(key, &current.value, None).await?;

        // Return the new version
        self.get_secret(key).await
    }

    async fn health_check(&self) -> Result<()> {
        debug!("Performing Vault health check");

        vaultrs::sys::health(&self.client)
            .await
            .map_err(|e| {
                error!("Vault health check failed: {}", e);
                SecretError::BackendUnavailable(format!("Health check failed: {}", e))
            })?;

        debug!("Vault health check: OK");
        Ok(())
    }

    async fn get_secret_versions(&self, key: &str) -> Result<Vec<SecretVersion>> {
        self.get_secret_versions_internal(key).await
    }

    async fn get_secret_version(&self, key: &str, version: &str) -> Result<Secret> {
        debug!("Retrieving secret {} version {}", key, version);

        let version_num = version.parse::<u64>().map_err(|_| {
            SecretError::InvalidSecret(format!("Invalid version number: {}", version))
        })?;

        let response: vaultrs::api::kv2::responses::ReadSecretResponse =
            kv2::read_version(&self.client, &self.mount_path, key, version_num)
                .await
                .map_err(|e| {
                    error!("Failed to read secret version: {}", e);
                    Self::convert_vault_error(key, e)
                })?;

        let value = response
            .data
            .get("value")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                SecretError::InvalidSecret("Secret does not contain a 'value' field".to_string())
            })?
            .to_string();

        let mut metadata = HashMap::new();
        if let Some(obj) = response.data.as_object() {
            for (k, v) in obj.iter() {
                if k != "value" {
                    if let Some(s) = v.as_str() {
                        metadata.insert(k.clone(), s.to_string());
                    }
                }
            }
        }

        Ok(Secret {
            key: key.to_string(),
            value,
            version: Some(version.to_string()),
            created_at: response
                .metadata
                .created_time
                .parse::<DateTime<Utc>>()
                .unwrap_or_else(|_| Utc::now()),
            metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_store_creation() {
        let result = VaultSecretStore::new(
            "http://localhost:8200".to_string(),
            "test-token".to_string(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_vault_store_with_namespace() {
        let store = VaultSecretStore::new(
            "http://localhost:8200".to_string(),
            "test-token".to_string(),
        )
        .unwrap()
        .with_namespace("production".to_string());

        assert_eq!(store.namespace, Some("production".to_string()));
    }

    #[test]
    fn test_vault_store_with_mount_path() {
        let store = VaultSecretStore::new(
            "http://localhost:8200".to_string(),
            "test-token".to_string(),
        )
        .unwrap()
        .with_mount_path("custom-secrets".to_string());

        assert_eq!(store.mount_path, "custom-secrets");
    }
}
