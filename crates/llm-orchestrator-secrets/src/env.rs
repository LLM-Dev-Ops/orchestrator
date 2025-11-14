// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Environment variable secret store implementation.
//!
//! This provides a simple fallback mechanism for reading secrets from
//! environment variables. It's useful for development and testing, but
//! should not be used in production for sensitive secrets.

use crate::models::{Secret, SecretMetadata};
use crate::traits::{Result, SecretError, SecretStore};
use async_trait::async_trait;
use std::env;
use tracing::{debug, warn};

/// Environment variable-based secret store.
///
/// Maps secret keys to environment variable names by converting the key
/// to uppercase and replacing slashes and dashes with underscores.
///
/// # Examples
///
/// - `openai/api_key` → `OPENAI_API_KEY`
/// - `anthropic/api-key` → `ANTHROPIC_API_KEY`
/// - `db/password` → `DB_PASSWORD`
///
/// # Security Considerations
///
/// - Environment variables can be visible in process listings
/// - They may be logged or exposed through error messages
/// - Use this store only for development or non-sensitive secrets
/// - For production, use HashiCorp Vault or AWS Secrets Manager
pub struct EnvSecretStore {
    /// Optional prefix to add to all environment variable names.
    prefix: Option<String>,
}

impl EnvSecretStore {
    /// Create a new environment variable secret store.
    pub fn new() -> Self {
        Self { prefix: None }
    }

    /// Create a new environment variable secret store with a prefix.
    ///
    /// The prefix will be prepended to all environment variable names.
    /// For example, with prefix "APP_", the key "db/password" will map
    /// to the environment variable "APP_DB_PASSWORD".
    pub fn with_prefix(prefix: String) -> Self {
        Self {
            prefix: Some(prefix),
        }
    }

    /// Convert a secret key to an environment variable name.
    ///
    /// # Arguments
    ///
    /// * `key` - The secret key (e.g., "openai/api_key")
    ///
    /// # Returns
    ///
    /// The corresponding environment variable name (e.g., "OPENAI_API_KEY")
    fn key_to_env_var(&self, key: &str) -> String {
        let normalized = key
            .replace(['/', '-'], "_")
            .to_uppercase();

        if let Some(ref prefix) = self.prefix {
            format!("{}{}", prefix, normalized)
        } else {
            normalized
        }
    }
}

impl Default for EnvSecretStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SecretStore for EnvSecretStore {
    async fn get_secret(&self, key: &str) -> Result<Secret> {
        let env_var = self.key_to_env_var(key);
        debug!("Retrieving secret from environment variable: {}", env_var);

        match env::var(&env_var) {
            Ok(value) => {
                if value.is_empty() {
                    warn!("Environment variable {} is empty", env_var);
                    return Err(SecretError::NotFound(format!(
                        "Environment variable {} is empty",
                        env_var
                    )));
                }

                debug!("Successfully retrieved secret from {}", env_var);
                Ok(Secret::new(key.to_string(), value)
                    .add_metadata("source".to_string(), "environment".to_string())
                    .add_metadata("env_var".to_string(), env_var))
            }
            Err(_) => {
                warn!("Environment variable not found: {}", env_var);
                Err(SecretError::EnvVarNotFound(env_var))
            }
        }
    }

    async fn put_secret(
        &self,
        _key: &str,
        _value: &str,
        _metadata: Option<SecretMetadata>,
    ) -> Result<()> {
        Err(SecretError::NotSupported(
            "Environment variable store is read-only".to_string(),
        ))
    }

    async fn delete_secret(&self, _key: &str) -> Result<()> {
        Err(SecretError::NotSupported(
            "Environment variable store is read-only".to_string(),
        ))
    }

    async fn list_secrets(&self, prefix: &str) -> Result<Vec<String>> {
        // We can't reliably list all environment variables that match our pattern,
        // so we return an error indicating this operation is not supported.
        let _ = prefix;
        Err(SecretError::NotSupported(
            "Listing secrets not supported for environment variable store".to_string(),
        ))
    }

    async fn rotate_secret(&self, _key: &str) -> Result<Secret> {
        Err(SecretError::NotSupported(
            "Secret rotation not supported for environment variable store".to_string(),
        ))
    }

    async fn health_check(&self) -> Result<()> {
        // Environment variables are always available, so health check always succeeds
        debug!("Environment variable secret store health check: OK");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_to_env_var() {
        let store = EnvSecretStore::new();
        assert_eq!(store.key_to_env_var("openai/api_key"), "OPENAI_API_KEY");
        assert_eq!(store.key_to_env_var("anthropic/api-key"), "ANTHROPIC_API_KEY");
        assert_eq!(store.key_to_env_var("db/password"), "DB_PASSWORD");
        assert_eq!(store.key_to_env_var("simple_key"), "SIMPLE_KEY");
    }

    #[test]
    fn test_key_to_env_var_with_prefix() {
        let store = EnvSecretStore::with_prefix("APP_".to_string());
        assert_eq!(store.key_to_env_var("db/password"), "APP_DB_PASSWORD");
        assert_eq!(store.key_to_env_var("api_key"), "APP_API_KEY");
    }

    #[tokio::test]
    async fn test_get_secret_success() {
        env::set_var("TEST_SECRET_KEY", "test_value");

        let store = EnvSecretStore::new();
        let result = store.get_secret("test/secret/key").await;

        assert!(result.is_ok());
        let secret = result.unwrap();
        assert_eq!(secret.value, "test_value");
        assert_eq!(secret.key, "test/secret/key");

        env::remove_var("TEST_SECRET_KEY");
    }

    #[tokio::test]
    async fn test_get_secret_not_found() {
        let store = EnvSecretStore::new();
        let result = store.get_secret("nonexistent/key").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SecretError::EnvVarNotFound(_) => {}
            _ => panic!("Expected EnvVarNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_get_secret_empty_value() {
        env::set_var("EMPTY_SECRET", "");

        let store = EnvSecretStore::new();
        let result = store.get_secret("empty/secret").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SecretError::NotFound(_) => {}
            _ => panic!("Expected NotFound error for empty value"),
        }

        env::remove_var("EMPTY_SECRET");
    }

    #[tokio::test]
    async fn test_put_secret_not_supported() {
        let store = EnvSecretStore::new();
        let result = store.put_secret("test/key", "value", None).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SecretError::NotSupported(_) => {}
            _ => panic!("Expected NotSupported error"),
        }
    }

    #[tokio::test]
    async fn test_health_check() {
        let store = EnvSecretStore::new();
        let result = store.health_check().await;
        assert!(result.is_ok());
    }
}
