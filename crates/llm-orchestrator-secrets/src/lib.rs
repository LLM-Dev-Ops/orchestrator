// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Secret management for LLM Orchestrator.
//!
//! This crate provides secure secret storage and retrieval with support for:
//! - HashiCorp Vault (KV v2)
//! - AWS Secrets Manager
//! - Environment variables (fallback)
//! - In-memory caching with TTL
//!
//! # Features
//!
//! - **Multiple backends**: Vault, AWS Secrets Manager, or environment variables
//! - **Automatic caching**: Optional TTL-based caching to reduce backend calls
//! - **Secret rotation**: Support for rotating secrets without downtime
//! - **Version management**: Access historical versions of secrets (where supported)
//! - **Security**: Zero secrets in logs, secure token handling
//!
//! # Examples
//!
//! ## Using Environment Variables
//!
//! ```
//! use llm_orchestrator_secrets::{EnvSecretStore, SecretStore};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let store = EnvSecretStore::new();
//! let secret = store.get_secret("openai/api_key").await?;
//! // Reads from environment variable: OPENAI_API_KEY
//! # Ok(())
//! # }
//! ```
//!
//! ## Using HashiCorp Vault
//!
//! ```no_run
//! use llm_orchestrator_secrets::{VaultSecretStore, SecretStore};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let store = VaultSecretStore::new(
//!     "https://vault.example.com:8200".to_string(),
//!     "hvs.CAESIJ...".to_string(),
//! )?;
//!
//! let secret = store.get_secret("database/password").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Using AWS Secrets Manager
//!
//! ```no_run
//! use llm_orchestrator_secrets::{AwsSecretStore, SecretStore};
//! use aws_sdk_secretsmanager::config::Region;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let store = AwsSecretStore::new(Region::new("us-east-1")).await?;
//! let secret = store.get_secret("prod/api/key").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Using the Builder with Caching
//!
//! ```no_run
//! use llm_orchestrator_secrets::{SecretManagerBuilder, SecretStoreType};
//! use chrono::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let store = SecretManagerBuilder::new(SecretStoreType::Environment)
//!     .with_cache(Duration::minutes(10))
//!     .build()
//!     .await?;
//!
//! let secret = store.get_secret("api_key").await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Security Best Practices
//!
//! 1. **Never log secret values**: All implementations avoid logging sensitive data
//! 2. **Use Vault or AWS in production**: Environment variables are for development only
//! 3. **Enable caching cautiously**: Balance performance with security requirements
//! 4. **Rotate secrets regularly**: Use built-in rotation features
//! 5. **Use least-privilege access**: Limit secret access to what's needed
//!
//! # Performance Considerations
//!
//! - **Caching**: Reduces backend calls from ~100ms to <1ms for cached secrets
//! - **TTL**: Default 5 minutes balances freshness with performance
//! - **Cleanup**: Run `cleanup_expired()` periodically to prevent memory growth

pub mod aws;
pub mod builder;
pub mod cache;
pub mod env;
pub mod models;
pub mod traits;
pub mod vault;

// Re-export main types for convenience
pub use aws::AwsSecretStore;
pub use builder::{AwsConfig, SecretManagerBuilder, SecretStoreType, VaultConfig};
pub use cache::{CacheStats, SecretCache};
pub use env::EnvSecretStore;
pub use models::{Secret, SecretMetadata, SecretVersion};
pub use traits::{Result, SecretError, SecretStore};
pub use vault::VaultSecretStore;
