// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Secret manager builder and factory.

use crate::aws::AwsSecretStore;
use crate::cache::SecretCache;
use crate::env::EnvSecretStore;
use crate::traits::{Result, SecretError, SecretStore};
use crate::vault::VaultSecretStore;
use aws_sdk_secretsmanager::config::Region;
use chrono::Duration;
use std::sync::Arc;
use tracing::info;

/// Type of secret store backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretStoreType {
    /// HashiCorp Vault backend.
    Vault,
    /// AWS Secrets Manager backend.
    AwsSecretsManager,
    /// Environment variable backend.
    Environment,
}

/// Builder for creating configured secret stores.
///
/// # Example
///
/// ```no_run
/// use llm_orchestrator_secrets::{SecretManagerBuilder, SecretStoreType};
/// use chrono::Duration;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let store = SecretManagerBuilder::new(SecretStoreType::Environment)
///     .with_cache(Duration::minutes(10))
///     .build()
///     .await?;
///
/// let secret = store.get_secret("api_key").await?;
/// # Ok(())
/// # }
/// ```
pub struct SecretManagerBuilder {
    /// Type of secret store to create.
    store_type: SecretStoreType,
    /// Whether to enable caching.
    cache_enabled: bool,
    /// Cache TTL duration.
    cache_ttl: Duration,
    /// Vault-specific configuration.
    vault_config: Option<VaultConfig>,
    /// AWS-specific configuration.
    aws_config: Option<AwsConfig>,
    /// Environment variable prefix.
    env_prefix: Option<String>,
}

/// Configuration for HashiCorp Vault.
#[derive(Debug, Clone)]
pub struct VaultConfig {
    /// Vault server address.
    pub address: String,
    /// Authentication token.
    pub token: String,
    /// Optional namespace (Vault Enterprise).
    pub namespace: Option<String>,
    /// Mount path for KV v2 engine.
    pub mount_path: Option<String>,
}

impl VaultConfig {
    /// Create a new Vault configuration.
    pub fn new(address: String, token: String) -> Self {
        Self {
            address,
            token,
            namespace: None,
            mount_path: None,
        }
    }

    /// Set the namespace.
    pub fn with_namespace(mut self, namespace: String) -> Self {
        self.namespace = Some(namespace);
        self
    }

    /// Set the mount path.
    pub fn with_mount_path(mut self, mount_path: String) -> Self {
        self.mount_path = Some(mount_path);
        self
    }

    /// Load Vault configuration from environment variables.
    ///
    /// Reads:
    /// - `VAULT_ADDR` - Vault server address
    /// - `VAULT_TOKEN` - Authentication token
    /// - `VAULT_NAMESPACE` - Optional namespace
    /// - `VAULT_MOUNT_PATH` - Optional mount path
    pub fn from_env() -> Result<Self> {
        let address = std::env::var("VAULT_ADDR")
            .map_err(|_| SecretError::EnvVarNotFound("VAULT_ADDR".to_string()))?;

        let token = std::env::var("VAULT_TOKEN")
            .map_err(|_| SecretError::EnvVarNotFound("VAULT_TOKEN".to_string()))?;

        let namespace = std::env::var("VAULT_NAMESPACE").ok();
        let mount_path = std::env::var("VAULT_MOUNT_PATH").ok();

        Ok(Self {
            address,
            token,
            namespace,
            mount_path,
        })
    }
}

/// Configuration for AWS Secrets Manager.
#[derive(Debug, Clone)]
pub struct AwsConfig {
    /// AWS region.
    pub region: Option<Region>,
}

impl AwsConfig {
    /// Create a new AWS configuration with a specific region.
    pub fn new(region: Region) -> Self {
        Self {
            region: Some(region),
        }
    }

    /// Create configuration that will use the default region from environment.
    pub fn from_env() -> Self {
        Self { region: None }
    }
}

impl SecretManagerBuilder {
    /// Create a new secret manager builder.
    ///
    /// # Arguments
    ///
    /// * `store_type` - The type of secret store to create
    pub fn new(store_type: SecretStoreType) -> Self {
        Self {
            store_type,
            cache_enabled: false,
            cache_ttl: Duration::minutes(5),
            vault_config: None,
            aws_config: None,
            env_prefix: None,
        }
    }

    /// Enable caching with the specified TTL.
    ///
    /// # Arguments
    ///
    /// * `ttl` - Time-to-live for cache entries
    pub fn with_cache(mut self, ttl: Duration) -> Self {
        self.cache_enabled = true;
        self.cache_ttl = ttl;
        self
    }

    /// Disable caching (enabled by default if `with_cache` was called).
    pub fn without_cache(mut self) -> Self {
        self.cache_enabled = false;
        self
    }

    /// Set Vault configuration.
    ///
    /// This is required if using `SecretStoreType::Vault`.
    pub fn with_vault_config(mut self, config: VaultConfig) -> Self {
        self.vault_config = Some(config);
        self
    }

    /// Set AWS configuration.
    ///
    /// This is optional for `SecretStoreType::AwsSecretsManager`.
    /// If not provided, AWS SDK will use default configuration from environment.
    pub fn with_aws_config(mut self, config: AwsConfig) -> Self {
        self.aws_config = Some(config);
        self
    }

    /// Set environment variable prefix.
    ///
    /// This is optional for `SecretStoreType::Environment`.
    pub fn with_env_prefix(mut self, prefix: String) -> Self {
        self.env_prefix = Some(prefix);
        self
    }

    /// Build the secret store.
    ///
    /// # Returns
    ///
    /// A configured secret store wrapped in an Arc for shared access.
    pub async fn build(self) -> Result<Arc<dyn SecretStore>> {
        info!("Building secret store: {:?}", self.store_type);

        let backend: Arc<dyn SecretStore> = match self.store_type {
            SecretStoreType::Vault => {
                let config = self.vault_config.ok_or_else(|| {
                    SecretError::Other(
                        "Vault configuration required for Vault store type".to_string(),
                    )
                })?;

                let mut store = VaultSecretStore::new(config.address, config.token)?;

                if let Some(namespace) = config.namespace {
                    store = store.with_namespace(namespace);
                }

                if let Some(mount_path) = config.mount_path {
                    store = store.with_mount_path(mount_path);
                }

                Arc::new(store)
            }

            SecretStoreType::AwsSecretsManager => {
                let store = if let Some(config) = self.aws_config {
                    if let Some(region) = config.region {
                        AwsSecretStore::new(region).await?
                    } else {
                        AwsSecretStore::from_env().await?
                    }
                } else {
                    AwsSecretStore::from_env().await?
                };

                Arc::new(store)
            }

            SecretStoreType::Environment => {
                let store = if let Some(prefix) = self.env_prefix {
                    EnvSecretStore::with_prefix(prefix)
                } else {
                    EnvSecretStore::new()
                };

                Arc::new(store)
            }
        };

        // Wrap with cache if enabled
        if self.cache_enabled {
            info!(
                "Enabling cache with TTL of {} seconds",
                self.cache_ttl.num_seconds()
            );
            Ok(Arc::new(SecretCache::new(backend, self.cache_ttl)))
        } else {
            Ok(backend)
        }
    }

    /// Build with Vault configuration from environment variables.
    ///
    /// This is a convenience method for creating a Vault store from environment.
    pub async fn build_vault_from_env(cache_enabled: bool) -> Result<Arc<dyn SecretStore>> {
        let config = VaultConfig::from_env()?;

        let mut builder = Self::new(SecretStoreType::Vault).with_vault_config(config);

        if cache_enabled {
            builder = builder.with_cache(Duration::minutes(5));
        }

        builder.build().await
    }

    /// Build with AWS Secrets Manager using default configuration.
    ///
    /// This is a convenience method for creating an AWS store with defaults.
    pub async fn build_aws_default(cache_enabled: bool) -> Result<Arc<dyn SecretStore>> {
        let mut builder = Self::new(SecretStoreType::AwsSecretsManager)
            .with_aws_config(AwsConfig::from_env());

        if cache_enabled {
            builder = builder.with_cache(Duration::minutes(5));
        }

        builder.build().await
    }

    /// Build with environment variable store.
    ///
    /// This is a convenience method for creating an environment store.
    pub async fn build_env(prefix: Option<String>) -> Result<Arc<dyn SecretStore>> {
        let mut builder = Self::new(SecretStoreType::Environment);

        if let Some(p) = prefix {
            builder = builder.with_env_prefix(p);
        }

        // Environment store doesn't usually need caching since env vars are already fast
        builder.build().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_config_from_env_missing() {
        std::env::remove_var("VAULT_ADDR");
        std::env::remove_var("VAULT_TOKEN");

        let result = VaultConfig::from_env();
        assert!(result.is_err());
    }

    #[test]
    fn test_vault_config_from_env() {
        std::env::set_var("VAULT_ADDR", "http://localhost:8200");
        std::env::set_var("VAULT_TOKEN", "test-token");
        std::env::set_var("VAULT_NAMESPACE", "test-ns");

        let config = VaultConfig::from_env().unwrap();
        assert_eq!(config.address, "http://localhost:8200");
        assert_eq!(config.token, "test-token");
        assert_eq!(config.namespace, Some("test-ns".to_string()));

        std::env::remove_var("VAULT_ADDR");
        std::env::remove_var("VAULT_TOKEN");
        std::env::remove_var("VAULT_NAMESPACE");
    }

    #[tokio::test]
    async fn test_build_env_store() {
        let store = SecretManagerBuilder::build_env(None).await.unwrap();
        assert!(store.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_build_env_store_with_prefix() {
        let store = SecretManagerBuilder::build_env(Some("APP_".to_string()))
            .await
            .unwrap();
        assert!(store.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_builder_with_cache() {
        let builder = SecretManagerBuilder::new(SecretStoreType::Environment)
            .with_cache(Duration::minutes(10));

        assert!(builder.cache_enabled);
        assert_eq!(builder.cache_ttl, Duration::minutes(10));
    }

    #[tokio::test]
    async fn test_builder_without_cache() {
        let builder = SecretManagerBuilder::new(SecretStoreType::Environment)
            .with_cache(Duration::minutes(10))
            .without_cache();

        assert!(!builder.cache_enabled);
    }
}
