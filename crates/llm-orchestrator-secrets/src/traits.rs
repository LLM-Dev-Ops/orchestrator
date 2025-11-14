// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Traits for secret store implementations.

use crate::models::{Secret, SecretMetadata, SecretVersion};
use async_trait::async_trait;
use thiserror::Error;

/// Result type for secret store operations.
pub type Result<T> = std::result::Result<T, SecretError>;

/// Errors that can occur during secret operations.
#[derive(Debug, Error)]
pub enum SecretError {
    /// Secret not found.
    #[error("Secret not found: {0}")]
    NotFound(String),

    /// Authentication failed with the secret backend.
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Permission denied for the requested operation.
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// The secret backend is unavailable.
    #[error("Backend unavailable: {0}")]
    BackendUnavailable(String),

    /// Invalid secret key or value.
    #[error("Invalid secret: {0}")]
    InvalidSecret(String),

    /// The operation is not supported by this backend.
    #[error("Operation not supported: {0}")]
    NotSupported(String),

    /// Network or communication error.
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Environment variable not found.
    #[error("Environment variable not found: {0}")]
    EnvVarNotFound(String),

    /// Generic error.
    #[error("Error: {0}")]
    Other(String),
}

/// Trait for secret storage backends.
///
/// Implementations provide different backends for storing and retrieving secrets,
/// such as HashiCorp Vault, AWS Secrets Manager, or environment variables.
#[async_trait]
pub trait SecretStore: Send + Sync {
    /// Retrieve a secret by key.
    ///
    /// # Arguments
    ///
    /// * `key` - The secret key to retrieve
    ///
    /// # Returns
    ///
    /// The secret if found, or an error if not found or inaccessible.
    async fn get_secret(&self, key: &str) -> Result<Secret>;

    /// Store a secret (if supported by the backend).
    ///
    /// # Arguments
    ///
    /// * `key` - The secret key
    /// * `value` - The secret value
    /// * `metadata` - Optional metadata for the secret
    ///
    /// # Returns
    ///
    /// Ok(()) if successful, or an error if the operation fails.
    ///
    /// # Note
    ///
    /// Some backends (like environment variables) don't support writing secrets
    /// and will return `SecretError::NotSupported`.
    async fn put_secret(
        &self,
        key: &str,
        value: &str,
        metadata: Option<SecretMetadata>,
    ) -> Result<()>;

    /// Delete a secret.
    ///
    /// # Arguments
    ///
    /// * `key` - The secret key to delete
    ///
    /// # Returns
    ///
    /// Ok(()) if successful, or an error if the operation fails.
    async fn delete_secret(&self, key: &str) -> Result<()>;

    /// List available secret keys with the given prefix.
    ///
    /// # Arguments
    ///
    /// * `prefix` - Key prefix to filter by (empty string for all secrets)
    ///
    /// # Returns
    ///
    /// A list of secret keys matching the prefix.
    async fn list_secrets(&self, prefix: &str) -> Result<Vec<String>>;

    /// Rotate a secret (invalidate old version, create new version).
    ///
    /// # Arguments
    ///
    /// * `key` - The secret key to rotate
    ///
    /// # Returns
    ///
    /// The new secret version if successful.
    ///
    /// # Note
    ///
    /// The implementation of rotation is backend-specific. Some backends may
    /// generate a new value automatically, while others may require external
    /// generation.
    async fn rotate_secret(&self, key: &str) -> Result<Secret>;

    /// Perform a health check on the secret backend.
    ///
    /// # Returns
    ///
    /// Ok(()) if the backend is healthy and accessible, or an error otherwise.
    async fn health_check(&self) -> Result<()>;

    /// Get all versions of a secret (if supported by backend).
    ///
    /// # Arguments
    ///
    /// * `key` - The secret key
    ///
    /// # Returns
    ///
    /// A list of all versions of the secret, or an error if not supported.
    async fn get_secret_versions(&self, key: &str) -> Result<Vec<SecretVersion>> {
        let _ = key;
        Err(SecretError::NotSupported(
            "Secret versioning not supported by this backend".to_string(),
        ))
    }

    /// Get a specific version of a secret (if supported by backend).
    ///
    /// # Arguments
    ///
    /// * `key` - The secret key
    /// * `version` - The version identifier
    ///
    /// # Returns
    ///
    /// The secret at the specified version, or an error if not found or not supported.
    async fn get_secret_version(&self, key: &str, version: &str) -> Result<Secret> {
        let (_, _) = (key, version);
        Err(SecretError::NotSupported(
            "Secret versioning not supported by this backend".to_string(),
        ))
    }
}
