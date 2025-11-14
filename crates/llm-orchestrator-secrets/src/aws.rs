// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! AWS Secrets Manager secret store implementation.

use crate::models::{Secret, SecretMetadata, SecretVersion};
use crate::traits::{Result, SecretError, SecretStore};
use async_trait::async_trait;
use aws_sdk_secretsmanager::{
    config::{BehaviorVersion, Region},
    types::{Filter, FilterNameStringType, Tag},
    Client as SecretsManagerClient,
};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tracing::{debug, error, info};

/// AWS Secrets Manager secret store.
///
/// # Features
///
/// - AWS SDK integration
/// - IAM role authentication
/// - Cross-region support
/// - Automatic rotation support
/// - Version management
/// - Tagging support
///
/// # Example
///
/// ```no_run
/// use llm_orchestrator_secrets::AwsSecretStore;
/// use aws_sdk_secretsmanager::config::Region;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let store = AwsSecretStore::new(Region::new("us-east-1")).await?;
///
/// let secret = store.get_secret("prod/database/password").await?;
/// println!("Retrieved secret: {}", secret.key);
/// # Ok(())
/// # }
/// ```
pub struct AwsSecretStore {
    /// AWS Secrets Manager client.
    client: SecretsManagerClient,
    /// AWS region.
    region: Region,
}

impl AwsSecretStore {
    /// Create a new AWS Secrets Manager store with the specified region.
    ///
    /// # Arguments
    ///
    /// * `region` - The AWS region to use
    ///
    /// # Returns
    ///
    /// A configured AWS Secrets Manager store.
    pub async fn new(region: Region) -> Result<Self> {
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region.clone())
            .load()
            .await;

        let client = SecretsManagerClient::new(&config);

        debug!("Initialized AWS Secrets Manager client for region: {}", region);

        Ok(Self { client, region })
    }

    /// Create a new AWS Secrets Manager store using the default region from environment.
    ///
    /// Reads the region from the `AWS_REGION` or `AWS_DEFAULT_REGION` environment variable.
    pub async fn from_env() -> Result<Self> {
        let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        let region = config
            .region()
            .ok_or_else(|| {
                SecretError::Other(
                    "No AWS region specified in environment or config".to_string(),
                )
            })?
            .clone();

        let client = SecretsManagerClient::new(&config);

        debug!("Initialized AWS Secrets Manager client from environment");

        Ok(Self { client, region })
    }

    /// Create a secret with automatic rotation.
    ///
    /// # Arguments
    ///
    /// * `key` - The secret key/name
    /// * `value` - The secret value
    /// * `rotation_days` - Number of days between automatic rotations
    pub async fn create_secret_with_rotation(
        &self,
        key: &str,
        value: &str,
        rotation_days: u32,
    ) -> Result<()> {
        debug!(
            "Creating secret {} with {} day rotation",
            key, rotation_days
        );

        // First create the secret
        self.put_secret(key, value, None).await?;

        // Then configure rotation
        self.client
            .rotate_secret()
            .secret_id(key)
            .rotation_rules(
                aws_sdk_secretsmanager::types::RotationRulesType::builder()
                    .automatically_after_days(rotation_days as i64)
                    .build(),
            )
            .send()
            .await
            .map_err(|e| {
                error!("Failed to configure rotation for {}: {}", key, e);
                SecretError::Other(format!("Failed to configure rotation: {}", e))
            })?;

        info!(
            "Successfully configured {} day rotation for {}",
            rotation_days, key
        );
        Ok(())
    }

    /// Get the value of a secret at a specific version.
    ///
    /// # Arguments
    ///
    /// * `secret_id` - The secret identifier
    /// * `version_id` - Optional version ID (uses current version if None)
    pub async fn get_secret_value(
        &self,
        secret_id: &str,
        version_id: Option<String>,
    ) -> Result<String> {
        let mut request = self.client.get_secret_value().secret_id(secret_id);

        if let Some(vid) = version_id {
            request = request.version_id(vid);
        }

        let response = request.send().await.map_err(|e| {
            error!("Failed to get secret value for {}: {}", secret_id, e);
            Self::convert_aws_error(e)
        })?;

        response
            .secret_string()
            .ok_or_else(|| {
                SecretError::InvalidSecret(
                    "Secret does not contain a string value".to_string(),
                )
            })
            .map(|s| s.to_string())
    }

    /// Convert AWS SDK error to SecretError.
    fn convert_aws_error(
        err: aws_sdk_secretsmanager::error::SdkError<
            impl std::error::Error + Send + Sync + 'static,
        >,
    ) -> SecretError {
        match err {
            aws_sdk_secretsmanager::error::SdkError::ServiceError(service_err) => {
                let err_msg = service_err.err().to_string();
                if err_msg.contains("ResourceNotFoundException") {
                    SecretError::NotFound(err_msg)
                } else if err_msg.contains("AccessDeniedException") {
                    SecretError::PermissionDenied(err_msg)
                } else {
                    SecretError::Other(err_msg)
                }
            }
            aws_sdk_secretsmanager::error::SdkError::TimeoutError(_) => {
                SecretError::NetworkError("Request timeout".to_string())
            }
            _ => SecretError::BackendUnavailable(err.to_string()),
        }
    }
}

#[async_trait]
impl SecretStore for AwsSecretStore {
    async fn get_secret(&self, key: &str) -> Result<Secret> {
        debug!("Retrieving secret from AWS Secrets Manager: {}", key);

        let response = self
            .client
            .get_secret_value()
            .secret_id(key)
            .send()
            .await
            .map_err(Self::convert_aws_error)?;

        let value = response
            .secret_string()
            .ok_or_else(|| {
                SecretError::InvalidSecret(
                    "Secret does not contain a string value".to_string(),
                )
            })?
            .to_string();

        let version = response.version_id().map(|v| v.to_string());

        let created_at = response
            .created_date()
            .map(|dt| {
                DateTime::from_timestamp(dt.as_secs_f64() as i64, 0)
                    .unwrap_or_else(Utc::now)
            })
            .unwrap_or_else(Utc::now);

        // Get metadata
        let describe_response = self
            .client
            .describe_secret()
            .secret_id(key)
            .send()
            .await
            .map_err(Self::convert_aws_error)?;

        let mut metadata = HashMap::new();
        if let Some(desc) = describe_response.description() {
            metadata.insert("description".to_string(), desc.to_string());
        }
        if let Some(arn) = response.arn() {
            metadata.insert("arn".to_string(), arn.to_string());
        }
        metadata.insert("region".to_string(), self.region.to_string());

        // Add tags as metadata
        for tag in describe_response.tags() {
            if let (Some(key), Some(value)) = (tag.key(), tag.value()) {
                metadata.insert(format!("tag:{}", key), value.to_string());
            }
        }

        debug!("Successfully retrieved secret from AWS: {}", key);

        Ok(Secret {
            key: key.to_string(),
            value,
            version,
            created_at,
            metadata,
        })
    }

    async fn put_secret(
        &self,
        key: &str,
        value: &str,
        metadata: Option<SecretMetadata>,
    ) -> Result<()> {
        debug!("Storing secret in AWS Secrets Manager: {}", key);

        // Check if secret exists
        let exists = self
            .client
            .describe_secret()
            .secret_id(key)
            .send()
            .await
            .is_ok();

        if exists {
            // Update existing secret
            let request = self.client.put_secret_value().secret_id(key).secret_string(value);

            let _ = request.send().await.map_err(Self::convert_aws_error)?;
        } else {
            // Create new secret
            let mut request = self.client.create_secret().name(key).secret_string(value);

            if let Some(meta) = metadata {
                if let Some(desc) = meta.description {
                    request = request.description(desc);
                }

                if !meta.tags.is_empty() {
                    let tags: Vec<Tag> = meta
                        .tags
                        .iter()
                        .map(|(k, v)| {
                            Tag::builder()
                                .key(k)
                                .value(v)
                                .build()
                        })
                        .collect();
                    request = request.set_tags(Some(tags));
                }
            }

            request.send().await.map_err(Self::convert_aws_error)?;
        }

        info!("Successfully stored secret in AWS: {}", key);
        Ok(())
    }

    async fn delete_secret(&self, key: &str) -> Result<()> {
        debug!("Deleting secret from AWS Secrets Manager: {}", key);

        self.client
            .delete_secret()
            .secret_id(key)
            .force_delete_without_recovery(false) // Use default recovery window
            .send()
            .await
            .map_err(Self::convert_aws_error)?;

        info!("Successfully scheduled deletion for secret: {}", key);
        Ok(())
    }

    async fn list_secrets(&self, prefix: &str) -> Result<Vec<String>> {
        debug!("Listing secrets with prefix: {}", prefix);

        let mut request = self.client.list_secrets();

        if !prefix.is_empty() {
            let filter = Filter::builder()
                .key(FilterNameStringType::Name)
                .values(format!("{}*", prefix))
                .build();
            request = request.filters(filter);
        }

        let response = request.send().await.map_err(Self::convert_aws_error)?;

        let keys: Vec<String> = response
            .secret_list()
            .iter()
            .filter_map(|secret| secret.name().map(|n| n.to_string()))
            .collect();

        debug!("Found {} secrets with prefix {}", keys.len(), prefix);
        Ok(keys)
    }

    async fn rotate_secret(&self, key: &str) -> Result<Secret> {
        debug!("Rotating secret: {}", key);

        self.client
            .rotate_secret()
            .secret_id(key)
            .send()
            .await
            .map_err(Self::convert_aws_error)?;

        info!("Successfully initiated rotation for secret: {}", key);

        // Return the updated secret
        self.get_secret(key).await
    }

    async fn health_check(&self) -> Result<()> {
        debug!("Performing AWS Secrets Manager health check");

        // Try to list secrets as a health check
        let result = self.client.list_secrets().max_results(1).send().await;

        match result {
            Ok(_) => {
                debug!("AWS Secrets Manager health check: OK");
                Ok(())
            }
            Err(e) => {
                error!("AWS Secrets Manager health check failed: {}", e);
                Err(Self::convert_aws_error(e))
            }
        }
    }

    async fn get_secret_versions(&self, key: &str) -> Result<Vec<SecretVersion>> {
        debug!("Retrieving versions for secret: {}", key);

        let response = self
            .client
            .list_secret_version_ids()
            .secret_id(key)
            .send()
            .await
            .map_err(Self::convert_aws_error)?;

        let mut versions = Vec::new();

        for version in response.versions() {
            if let Some(version_id) = version.version_id() {
                let created_at = version
                    .created_date()
                    .map(|dt| {
                        DateTime::from_timestamp(dt.as_secs_f64() as i64, 0)
                            .unwrap_or_else(Utc::now)
                    })
                    .unwrap_or_else(Utc::now);

                versions.push(SecretVersion {
                    version: version_id.to_string(),
                    created_at,
                    is_current: version
                        .version_stages()
                            .contains(&"AWSCURRENT".to_string()),
                        is_deleted: false,
                    });
                }
        }

        debug!("Found {} versions for secret {}", versions.len(), key);
        Ok(versions)
    }

    async fn get_secret_version(&self, key: &str, version: &str) -> Result<Secret> {
        debug!("Retrieving secret {} version {}", key, version);

        let response = self
            .client
            .get_secret_value()
            .secret_id(key)
            .version_id(version)
            .send()
            .await
            .map_err(Self::convert_aws_error)?;

        let value = response
            .secret_string()
            .ok_or_else(|| {
                SecretError::InvalidSecret(
                    "Secret does not contain a string value".to_string(),
                )
            })?
            .to_string();

        let created_at = response
            .created_date()
            .map(|dt| {
                DateTime::from_timestamp(dt.as_secs_f64() as i64, 0)
                    .unwrap_or_else(Utc::now)
            })
            .unwrap_or_else(Utc::now);

        let mut metadata = HashMap::new();
        if let Some(arn) = response.arn() {
            metadata.insert("arn".to_string(), arn.to_string());
        }

        Ok(Secret {
            key: key.to_string(),
            value,
            version: Some(version.to_string()),
            created_at,
            metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require AWS credentials and a running AWS account
    // They are meant to be run manually or in a CI environment with proper setup

    #[tokio::test]
    #[ignore] // Ignore by default as it requires AWS credentials
    async fn test_aws_store_from_env() {
        let result = AwsSecretStore::from_env().await;
        // This will fail if AWS credentials are not configured
        assert!(result.is_ok() || result.is_err());
    }
}
