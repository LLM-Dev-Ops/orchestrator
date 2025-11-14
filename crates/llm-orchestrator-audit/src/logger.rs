use crate::models::{AuditEvent, AuditEventType, AuditResult, ResourceType};
use crate::storage::{AuditStorage, AuditStorageRef, Result};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;

/// Audit logger for recording security and operational events
pub struct AuditLogger {
    storage: AuditStorageRef,
    enabled: bool,
    previous_hash: Arc<RwLock<Option<String>>>,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new(storage: AuditStorageRef) -> Self {
        Self {
            storage,
            enabled: true,
            previous_hash: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a disabled audit logger (for testing)
    pub fn disabled() -> Self {
        Self {
            storage: Arc::new(NoOpStorage),
            enabled: false,
            previous_hash: Arc::new(RwLock::new(None)),
        }
    }

    /// Check if the audit logger is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Log an authentication attempt
    pub async fn log_auth_attempt(
        &self,
        user_id: &str,
        success: bool,
        ip_address: Option<String>,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let result = if success {
            AuditResult::Success
        } else {
            AuditResult::Failure("Authentication failed".to_string())
        };

        let event = AuditEvent::new(
            AuditEventType::Authentication,
            "User authentication attempt".to_string(),
            ResourceType::User,
            user_id.to_string(),
            result,
        )
        .with_user_id(user_id.to_string())
        .with_ip_address(ip_address.unwrap_or_default());

        self.log_event(event).await
    }

    /// Log an authorization check
    pub async fn log_authorization(
        &self,
        user_id: &str,
        permission: &str,
        resource_id: &str,
        allowed: bool,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let result = if allowed {
            AuditResult::Success
        } else {
            AuditResult::Failure(format!("Permission denied: {}", permission))
        };

        let event = AuditEvent::new(
            AuditEventType::Authorization,
            format!("Authorization check: {}", permission),
            ResourceType::User,
            resource_id.to_string(),
            result,
        )
        .with_user_id(user_id.to_string())
        .with_details(serde_json::json!({
            "permission": permission,
            "allowed": allowed,
        }));

        self.log_event(event).await
    }

    /// Log a workflow execution
    pub async fn log_workflow_execution(
        &self,
        workflow_id: &str,
        user_id: &str,
        result: AuditResult,
        duration: Duration,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let event = AuditEvent::new(
            AuditEventType::WorkflowExecution,
            "Workflow executed".to_string(),
            ResourceType::Workflow,
            workflow_id.to_string(),
            result,
        )
        .with_user_id(user_id.to_string())
        .with_details(serde_json::json!({
            "duration_ms": duration.as_millis() as u64,
        }));

        self.log_event(event).await
    }

    /// Log a workflow creation
    pub async fn log_workflow_create(
        &self,
        workflow_id: &str,
        workflow_name: &str,
        user_id: &str,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let event = AuditEvent::new(
            AuditEventType::WorkflowCreate,
            "Workflow created".to_string(),
            ResourceType::Workflow,
            workflow_id.to_string(),
            AuditResult::Success,
        )
        .with_user_id(user_id.to_string())
        .with_details(serde_json::json!({
            "workflow_name": workflow_name,
        }));

        self.log_event(event).await
    }

    /// Log a workflow update
    pub async fn log_workflow_update(
        &self,
        workflow_id: &str,
        user_id: &str,
        changes: serde_json::Value,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let event = AuditEvent::new(
            AuditEventType::WorkflowUpdate,
            "Workflow updated".to_string(),
            ResourceType::Workflow,
            workflow_id.to_string(),
            AuditResult::Success,
        )
        .with_user_id(user_id.to_string())
        .with_details(changes);

        self.log_event(event).await
    }

    /// Log a workflow deletion
    pub async fn log_workflow_delete(
        &self,
        workflow_id: &str,
        user_id: &str,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let event = AuditEvent::new(
            AuditEventType::WorkflowDelete,
            "Workflow deleted".to_string(),
            ResourceType::Workflow,
            workflow_id.to_string(),
            AuditResult::Success,
        )
        .with_user_id(user_id.to_string());

        self.log_event(event).await
    }

    /// Log secret access
    pub async fn log_secret_access(
        &self,
        secret_key: &str,
        user_id: &str,
        accessed_at: DateTime<Utc>,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let event = AuditEvent::new(
            AuditEventType::SecretAccess,
            "Secret accessed".to_string(),
            ResourceType::Secret,
            secret_key.to_string(),
            AuditResult::Success,
        )
        .with_user_id(user_id.to_string())
        .with_details(serde_json::json!({
            "accessed_at": accessed_at.to_rfc3339(),
        }));

        self.log_event(event).await
    }

    /// Log configuration change
    pub async fn log_config_change(
        &self,
        config_key: &str,
        old_value: Option<&str>,
        new_value: &str,
        changed_by: &str,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let event = AuditEvent::new(
            AuditEventType::ConfigChange,
            "Configuration changed".to_string(),
            ResourceType::Configuration,
            config_key.to_string(),
            AuditResult::Success,
        )
        .with_user_id(changed_by.to_string())
        .with_details(serde_json::json!({
            "old_value": old_value,
            "new_value": new_value,
        }));

        self.log_event(event).await
    }

    /// Log API key creation
    pub async fn log_api_key_create(
        &self,
        key_id: &str,
        user_id: &str,
        scopes: Vec<String>,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let event = AuditEvent::new(
            AuditEventType::ApiKeyCreate,
            "API key created".to_string(),
            ResourceType::ApiKey,
            key_id.to_string(),
            AuditResult::Success,
        )
        .with_user_id(user_id.to_string())
        .with_details(serde_json::json!({
            "scopes": scopes,
        }));

        self.log_event(event).await
    }

    /// Log API key revocation
    pub async fn log_api_key_revoke(
        &self,
        key_id: &str,
        user_id: &str,
        reason: &str,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let event = AuditEvent::new(
            AuditEventType::ApiKeyRevoke,
            "API key revoked".to_string(),
            ResourceType::ApiKey,
            key_id.to_string(),
            AuditResult::Success,
        )
        .with_user_id(user_id.to_string())
        .with_details(serde_json::json!({
            "reason": reason,
        }));

        self.log_event(event).await
    }

    /// Log a step execution
    pub async fn log_step_execution(
        &self,
        workflow_id: &str,
        step_id: &str,
        user_id: &str,
        result: AuditResult,
        duration: Duration,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let event = AuditEvent::new(
            AuditEventType::StepExecution,
            format!("Step executed: {}", step_id),
            ResourceType::Step,
            step_id.to_string(),
            result,
        )
        .with_user_id(user_id.to_string())
        .with_details(serde_json::json!({
            "workflow_id": workflow_id,
            "duration_ms": duration.as_millis() as u64,
        }));

        self.log_event(event).await
    }

    /// Log a generic audit event
    pub async fn log_event(&self, mut event: AuditEvent) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        // Add hash chain for tamper detection
        let prev_hash = self.previous_hash.read().clone();
        event.previous_hash = prev_hash;
        event.event_hash = Some(event.compute_hash());

        // Store the event
        self.storage.store(&event).await?;

        // Update previous hash
        *self.previous_hash.write() = event.event_hash.clone();

        tracing::debug!(
            event_id = %event.id,
            event_type = event.event_type.as_str(),
            user_id = ?event.user_id,
            "Audit event logged"
        );

        Ok(())
    }

    /// Get the storage backend
    pub fn storage(&self) -> &AuditStorageRef {
        &self.storage
    }
}

/// No-op storage for disabled audit logger
struct NoOpStorage;

#[async_trait::async_trait]
impl AuditStorage for NoOpStorage {
    async fn store(&self, _event: &AuditEvent) -> Result<()> {
        Ok(())
    }

    async fn query(&self, _filter: crate::models::AuditFilter) -> Result<Vec<AuditEvent>> {
        Ok(vec![])
    }

    async fn get(&self, _id: uuid::Uuid) -> Result<Option<AuditEvent>> {
        Ok(None)
    }

    async fn delete_older_than(&self, _cutoff: DateTime<Utc>) -> Result<u64> {
        Ok(0)
    }

    async fn count(&self, _filter: crate::models::AuditFilter) -> Result<u64> {
        Ok(0)
    }

    async fn health_check(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::{FileAuditStorage, RotationPolicy};
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_audit_logger_workflow_execution() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Arc::new(
            FileAuditStorage::new(temp_file.path().to_path_buf(), RotationPolicy::Never).unwrap(),
        );
        let logger = AuditLogger::new(storage.clone());

        logger
            .log_workflow_execution(
                "workflow-123",
                "user-456",
                AuditResult::Success,
                Duration::from_millis(500),
            )
            .await
            .unwrap();

        let filter = crate::models::AuditFilter::new();
        let events = storage.query(filter).await.unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, AuditEventType::WorkflowExecution);
        assert_eq!(events[0].resource_id, "workflow-123");
    }

    #[tokio::test]
    async fn test_audit_logger_hash_chain() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Arc::new(
            FileAuditStorage::new(temp_file.path().to_path_buf(), RotationPolicy::Never).unwrap(),
        );
        let logger = AuditLogger::new(storage.clone());

        // Log first event
        logger
            .log_auth_attempt("user-1", true, Some("192.168.1.1".to_string()))
            .await
            .unwrap();

        // Log second event
        logger
            .log_auth_attempt("user-2", true, Some("192.168.1.2".to_string()))
            .await
            .unwrap();

        let filter = crate::models::AuditFilter::new();
        let events = storage.query(filter).await.unwrap();

        assert_eq!(events.len(), 2);

        // Second event should reference first event's hash
        assert!(events[0].previous_hash.is_some());
        assert!(events[1].previous_hash.is_none()); // First event has no previous
    }

    #[tokio::test]
    async fn test_disabled_logger() {
        let logger = AuditLogger::disabled();

        assert!(!logger.is_enabled());

        // Should not error when disabled
        logger
            .log_workflow_execution(
                "workflow-123",
                "user-456",
                AuditResult::Success,
                Duration::from_millis(500),
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_audit_logger_secret_access() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Arc::new(
            FileAuditStorage::new(temp_file.path().to_path_buf(), RotationPolicy::Never).unwrap(),
        );
        let logger = AuditLogger::new(storage.clone());

        logger
            .log_secret_access("api_key", "user-123", Utc::now())
            .await
            .unwrap();

        let filter = crate::models::AuditFilter::new();
        let events = storage.query(filter).await.unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, AuditEventType::SecretAccess);
        assert_eq!(events[0].resource_type, ResourceType::Secret);
    }
}
