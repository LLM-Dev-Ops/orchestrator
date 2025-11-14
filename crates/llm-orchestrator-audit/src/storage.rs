use crate::models::{AuditEvent, AuditFilter};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// Error type for audit storage operations
#[derive(Debug, Error)]
pub enum StorageError {
    /// Database error
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// File I/O error
    #[error("File I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Event not found
    #[error("Audit event not found: {0}")]
    EventNotFound(Uuid),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Connection error
    #[error("Connection error: {0}")]
    ConnectionError(String),
}

pub type Result<T> = std::result::Result<T, StorageError>;

/// Trait for storing and retrieving audit events
#[async_trait]
pub trait AuditStorage: Send + Sync {
    /// Store an audit event
    async fn store(&self, event: &AuditEvent) -> Result<()>;

    /// Query audit events with a filter
    async fn query(&self, filter: AuditFilter) -> Result<Vec<AuditEvent>>;

    /// Get a specific audit event by ID
    async fn get(&self, id: Uuid) -> Result<Option<AuditEvent>>;

    /// Delete audit events older than the specified cutoff time
    /// Returns the number of events deleted
    async fn delete_older_than(&self, cutoff: DateTime<Utc>) -> Result<u64>;

    /// Count audit events matching the filter
    async fn count(&self, filter: AuditFilter) -> Result<u64>;

    /// Check if the storage backend is healthy
    async fn health_check(&self) -> Result<()>;
}

/// Type alias for Arc-wrapped AuditStorage
pub type AuditStorageRef = Arc<dyn AuditStorage>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AuditEventType, AuditResult, ResourceType};

    // Mock implementation for testing
    struct MockStorage;

    #[async_trait]
    impl AuditStorage for MockStorage {
        async fn store(&self, _event: &AuditEvent) -> Result<()> {
            Ok(())
        }

        async fn query(&self, _filter: AuditFilter) -> Result<Vec<AuditEvent>> {
            Ok(vec![])
        }

        async fn get(&self, _id: Uuid) -> Result<Option<AuditEvent>> {
            Ok(None)
        }

        async fn delete_older_than(&self, _cutoff: DateTime<Utc>) -> Result<u64> {
            Ok(0)
        }

        async fn count(&self, _filter: AuditFilter) -> Result<u64> {
            Ok(0)
        }

        async fn health_check(&self) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_mock_storage() {
        let storage: AuditStorageRef = Arc::new(MockStorage);

        let event = AuditEvent::new(
            AuditEventType::WorkflowExecution,
            "Test".to_string(),
            ResourceType::Workflow,
            "test-123".to_string(),
            AuditResult::Success,
        );

        assert!(storage.store(&event).await.is_ok());
        assert!(storage.health_check().await.is_ok());
    }
}
