use crate::storage::{AuditStorageRef, Result};
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;
use tokio::time;

/// Manages retention policy for audit events
pub struct AuditRetentionManager {
    storage: AuditStorageRef,
    retention_days: u32,
}

impl AuditRetentionManager {
    /// Create a new retention manager
    pub fn new(storage: AuditStorageRef, retention_days: u32) -> Self {
        Self {
            storage,
            retention_days,
        }
    }

    /// Run cleanup of old audit events
    /// Returns the number of events deleted
    pub async fn cleanup(&self) -> Result<u64> {
        let cutoff = Utc::now() - Duration::days(self.retention_days as i64);

        tracing::info!(
            retention_days = self.retention_days,
            cutoff_date = %cutoff,
            "Running audit log cleanup"
        );

        let deleted = self.storage.delete_older_than(cutoff).await?;

        tracing::info!(
            deleted_count = deleted,
            "Audit log cleanup completed"
        );

        Ok(deleted)
    }

    /// Start background cleanup task
    /// Returns a handle that can be used to cancel the task
    pub fn start_background_cleanup(
        self: Arc<Self>,
        interval: time::Duration,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval_timer = time::interval(interval);

            loop {
                interval_timer.tick().await;

                match self.cleanup().await {
                    Ok(deleted) => {
                        tracing::debug!(deleted_count = deleted, "Background cleanup completed");
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Background cleanup failed");
                    }
                }
            }
        })
    }

    /// Get the retention period in days
    pub fn retention_days(&self) -> u32 {
        self.retention_days
    }

    /// Calculate the cutoff date for cleanup
    pub fn cutoff_date(&self) -> DateTime<Utc> {
        Utc::now() - Duration::days(self.retention_days as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::{FileAuditStorage, RotationPolicy};
    
    use crate::models::{AuditEventType, AuditFilter, AuditResult, ResourceType};
    use crate::models::AuditEvent;
    use std::time::Duration as StdDuration;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_retention_cleanup() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage: AuditStorageRef = Arc::new(
            FileAuditStorage::new(temp_file.path().to_path_buf(), RotationPolicy::Never).unwrap(),
        );

        // Create an old event (2 days old)
        let mut old_event = AuditEvent::new(
            AuditEventType::WorkflowExecution,
            "Old workflow".to_string(),
            ResourceType::Workflow,
            "workflow-old".to_string(),
            AuditResult::Success,
        );
        old_event.timestamp = Utc::now() - Duration::days(2);
        storage.store(&old_event).await.unwrap();

        // Create a recent event
        let recent_event = AuditEvent::new(
            AuditEventType::WorkflowExecution,
            "Recent workflow".to_string(),
            ResourceType::Workflow,
            "workflow-recent".to_string(),
            AuditResult::Success,
        );
        storage.store(&recent_event).await.unwrap();

        // Create retention manager with 1-day retention
        let manager = AuditRetentionManager::new(storage.clone(), 1);

        // Run cleanup
        let deleted = manager.cleanup().await.unwrap();

        assert_eq!(deleted, 1);

        // Verify only recent event remains
        let filter = AuditFilter::new();
        let events = storage.query(filter).await.unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].resource_id, "workflow-recent");
    }

    #[tokio::test]
    async fn test_retention_cutoff_date() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage: AuditStorageRef = Arc::new(
            FileAuditStorage::new(temp_file.path().to_path_buf(), RotationPolicy::Never).unwrap(),
        );

        let manager = AuditRetentionManager::new(storage, 90);

        assert_eq!(manager.retention_days(), 90);

        let cutoff = manager.cutoff_date();
        let expected_cutoff = Utc::now() - Duration::days(90);

        // Allow 1 second difference for test execution time
        let diff = (cutoff - expected_cutoff).num_seconds().abs();
        assert!(diff <= 1);
    }

    #[tokio::test]
    async fn test_background_cleanup() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage: AuditStorageRef = Arc::new(
            FileAuditStorage::new(temp_file.path().to_path_buf(), RotationPolicy::Never).unwrap(),
        );

        // Create an old event
        let mut old_event = AuditEvent::new(
            AuditEventType::WorkflowExecution,
            "Old workflow".to_string(),
            ResourceType::Workflow,
            "workflow-old".to_string(),
            AuditResult::Success,
        );
        old_event.timestamp = Utc::now() - Duration::days(2);
        storage.store(&old_event).await.unwrap();

        let manager = Arc::new(AuditRetentionManager::new(storage.clone(), 1));

        // Start background cleanup with short interval
        let handle = manager.start_background_cleanup(StdDuration::from_millis(100));

        // Wait for cleanup to run
        tokio::time::sleep(StdDuration::from_millis(200)).await;

        // Cancel background task
        handle.abort();

        // Verify event was deleted
        let filter = AuditFilter::new();
        let events = storage.query(filter).await.unwrap();

        assert_eq!(events.len(), 0);
    }
}
