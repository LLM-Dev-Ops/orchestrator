// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Data Vault adapter for persisting workflow artifacts and intermediate results.
//!
//! This adapter provides a thin integration layer to consume secure storage
//! services from LLM-Data-Vault without modifying core workflow logic.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Metadata for a stored artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    /// Unique identifier for the artifact.
    pub id: Uuid,
    /// Workflow execution ID that produced this artifact.
    pub workflow_id: Uuid,
    /// Step ID that produced this artifact.
    pub step_id: String,
    /// Artifact name/key.
    pub name: String,
    /// Content type (MIME type).
    pub content_type: String,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Expiration timestamp (if any).
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Whether the artifact is encrypted.
    pub encrypted: bool,
    /// Custom metadata tags.
    pub tags: HashMap<String, String>,
}

/// Result of a storage operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageResult {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Artifact ID (if applicable).
    pub artifact_id: Option<Uuid>,
    /// Error message (if failed).
    pub error: Option<String>,
    /// Storage location reference.
    pub location: Option<String>,
}

/// Adapter for consuming secure storage from LLM-Data-Vault.
///
/// This adapter enables the orchestrator to persist workflow artifacts
/// and intermediate results securely.
#[derive(Debug, Clone)]
pub struct DataVaultAdapter {
    /// Base URL for the data vault service.
    endpoint: String,
    /// Default encryption setting.
    encrypt_by_default: bool,
    /// Default TTL for artifacts (in seconds).
    default_ttl_seconds: Option<u64>,
    /// Whether the adapter is enabled.
    enabled: bool,
}

impl Default for DataVaultAdapter {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            encrypt_by_default: true,
            default_ttl_seconds: None,
            enabled: false,
        }
    }
}

impl DataVaultAdapter {
    /// Creates a new data vault adapter with the given endpoint.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            encrypt_by_default: true,
            default_ttl_seconds: None,
            enabled: true,
        }
    }

    /// Creates an adapter with custom settings.
    pub fn with_options(
        endpoint: impl Into<String>,
        encrypt_by_default: bool,
        default_ttl_seconds: Option<u64>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            encrypt_by_default,
            default_ttl_seconds,
            enabled: true,
        }
    }

    /// Creates a disabled adapter (no-op mode).
    pub fn disabled() -> Self {
        Self::default()
    }

    /// Returns whether the adapter is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Returns the configured endpoint.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Stores a workflow artifact.
    ///
    /// Persists the artifact data to the data vault with optional encryption.
    pub async fn store_artifact(
        &self,
        workflow_id: Uuid,
        step_id: &str,
        name: &str,
        data: &[u8],
        content_type: &str,
    ) -> StorageResult {
        if !self.enabled {
            return StorageResult {
                success: false,
                artifact_id: None,
                error: Some("Data vault adapter is disabled".to_string()),
                location: None,
            };
        }

        // Placeholder: In production, this would call llm-data-vault client
        let artifact_id = Uuid::new_v4();

        StorageResult {
            success: true,
            artifact_id: Some(artifact_id),
            error: None,
            location: Some(format!("vault://{}/{}/{}", workflow_id, step_id, name)),
        }
    }

    /// Stores intermediate results as JSON.
    ///
    /// Convenience method for storing step outputs.
    pub async fn store_intermediate_result(
        &self,
        workflow_id: Uuid,
        step_id: &str,
        result: &serde_json::Value,
    ) -> StorageResult {
        if !self.enabled {
            return StorageResult {
                success: false,
                artifact_id: None,
                error: Some("Data vault adapter is disabled".to_string()),
                location: None,
            };
        }

        let data = serde_json::to_vec(result).unwrap_or_default();
        self.store_artifact(
            workflow_id,
            step_id,
            "result.json",
            &data,
            "application/json",
        )
        .await
    }

    /// Retrieves artifact metadata.
    pub async fn get_artifact_metadata(&self, artifact_id: Uuid) -> Option<ArtifactMetadata> {
        if !self.enabled {
            return None;
        }

        // Placeholder: Would retrieve metadata from data vault
        None
    }

    /// Retrieves artifact data.
    pub async fn get_artifact_data(&self, artifact_id: Uuid) -> Option<Vec<u8>> {
        if !self.enabled {
            return None;
        }

        // Placeholder: Would retrieve data from data vault
        None
    }

    /// Lists artifacts for a workflow execution.
    pub async fn list_workflow_artifacts(&self, workflow_id: Uuid) -> Vec<ArtifactMetadata> {
        if !self.enabled {
            return Vec::new();
        }

        // Placeholder: Would list artifacts from data vault
        Vec::new()
    }

    /// Deletes an artifact.
    pub async fn delete_artifact(&self, artifact_id: Uuid) -> StorageResult {
        if !self.enabled {
            return StorageResult {
                success: false,
                artifact_id: Some(artifact_id),
                error: Some("Data vault adapter is disabled".to_string()),
                location: None,
            };
        }

        // Placeholder: Would delete from data vault
        StorageResult {
            success: true,
            artifact_id: Some(artifact_id),
            error: None,
            location: None,
        }
    }

    /// Cleans up expired artifacts for a workflow.
    pub async fn cleanup_expired(&self, workflow_id: Uuid) -> u64 {
        if !self.enabled {
            return 0;
        }

        // Placeholder: Would trigger cleanup in data vault
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_disabled_by_default() {
        let adapter = DataVaultAdapter::default();
        assert!(!adapter.is_enabled());
    }

    #[test]
    fn test_adapter_enabled_with_endpoint() {
        let adapter = DataVaultAdapter::new("http://localhost:8082");
        assert!(adapter.is_enabled());
        assert_eq!(adapter.endpoint(), "http://localhost:8082");
    }

    #[tokio::test]
    async fn test_store_when_disabled() {
        let adapter = DataVaultAdapter::disabled();
        let result = adapter
            .store_artifact(Uuid::new_v4(), "step1", "test.txt", b"data", "text/plain")
            .await;
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_store_when_enabled() {
        let adapter = DataVaultAdapter::new("http://localhost:8082");
        let workflow_id = Uuid::new_v4();
        let result = adapter
            .store_artifact(workflow_id, "step1", "test.txt", b"data", "text/plain")
            .await;
        assert!(result.success);
        assert!(result.artifact_id.is_some());
        assert!(result.location.is_some());
    }
}
