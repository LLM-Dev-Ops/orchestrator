use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents an audit event in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique identifier for this audit event
    pub id: Uuid,

    /// Timestamp when the event occurred
    pub timestamp: DateTime<Utc>,

    /// Type of audit event
    pub event_type: AuditEventType,

    /// ID of the user who performed the action (if applicable)
    pub user_id: Option<String>,

    /// Human-readable description of the action
    pub action: String,

    /// Type of resource affected
    pub resource_type: ResourceType,

    /// Unique identifier of the resource
    pub resource_id: String,

    /// Result of the action
    pub result: AuditResult,

    /// Additional details about the event (structured data)
    pub details: serde_json::Value,

    /// IP address of the client (if applicable)
    pub ip_address: Option<String>,

    /// User agent string from the client (if applicable)
    pub user_agent: Option<String>,

    /// Request ID for correlation across systems
    pub request_id: Option<String>,

    /// Hash of the previous audit event for tamper detection
    pub previous_hash: Option<String>,

    /// Hash of this audit event
    pub event_hash: Option<String>,
}

impl AuditEvent {
    /// Create a new audit event
    pub fn new(
        event_type: AuditEventType,
        action: String,
        resource_type: ResourceType,
        resource_id: String,
        result: AuditResult,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type,
            user_id: None,
            action,
            resource_type,
            resource_id,
            result,
            details: serde_json::Value::Null,
            ip_address: None,
            user_agent: None,
            request_id: None,
            previous_hash: None,
            event_hash: None,
        }
    }

    /// Set the user ID for this event
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Set additional details for this event
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = details;
        self
    }

    /// Set the IP address for this event
    pub fn with_ip_address(mut self, ip_address: String) -> Self {
        self.ip_address = Some(ip_address);
        self
    }

    /// Set the user agent for this event
    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    /// Set the request ID for this event
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// Compute the hash of this audit event for tamper detection
    pub fn compute_hash(&self) -> String {
        use sha2::{Digest, Sha256};

        let data = format!(
            "{}|{}|{}|{}|{}|{}|{}|{}",
            self.id,
            self.timestamp.to_rfc3339(),
            self.event_type.as_str(),
            self.action,
            self.resource_type.as_str(),
            self.resource_id,
            self.result.as_str(),
            self.previous_hash.as_deref().unwrap_or(""),
        );

        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        hex::encode(hasher.finalize())
    }
}

/// Type of audit event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditEventType {
    /// Authentication attempt (login, token validation)
    Authentication,

    /// Authorization check (permission validation)
    Authorization,

    /// Workflow execution
    WorkflowExecution,

    /// Workflow created
    WorkflowCreate,

    /// Workflow updated
    WorkflowUpdate,

    /// Workflow deleted
    WorkflowDelete,

    /// Secret accessed
    SecretAccess,

    /// Configuration changed
    ConfigChange,

    /// API key created
    ApiKeyCreate,

    /// API key revoked
    ApiKeyRevoke,

    /// Step execution
    StepExecution,

    /// System event
    SystemEvent,
}

impl AuditEventType {
    /// Get the string representation of the event type
    pub fn as_str(&self) -> &str {
        match self {
            Self::Authentication => "authentication",
            Self::Authorization => "authorization",
            Self::WorkflowExecution => "workflow_execution",
            Self::WorkflowCreate => "workflow_create",
            Self::WorkflowUpdate => "workflow_update",
            Self::WorkflowDelete => "workflow_delete",
            Self::SecretAccess => "secret_access",
            Self::ConfigChange => "config_change",
            Self::ApiKeyCreate => "api_key_create",
            Self::ApiKeyRevoke => "api_key_revoke",
            Self::StepExecution => "step_execution",
            Self::SystemEvent => "system_event",
        }
    }
}

/// Type of resource affected by the audit event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResourceType {
    /// Workflow resource
    Workflow,

    /// User resource
    User,

    /// API key resource
    ApiKey,

    /// Secret resource
    Secret,

    /// Configuration resource
    Configuration,

    /// Step resource
    Step,

    /// System resource
    System,
}

impl ResourceType {
    /// Get the string representation of the resource type
    pub fn as_str(&self) -> &str {
        match self {
            Self::Workflow => "workflow",
            Self::User => "user",
            Self::ApiKey => "api_key",
            Self::Secret => "secret",
            Self::Configuration => "configuration",
            Self::Step => "step",
            Self::System => "system",
        }
    }
}

/// Result of an audit action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditResult {
    /// Action succeeded
    Success,

    /// Action failed with an error message
    Failure(String),

    /// Action partially succeeded
    PartialSuccess,
}

impl AuditResult {
    /// Get the string representation of the result
    pub fn as_str(&self) -> &str {
        match self {
            Self::Success => "success",
            Self::Failure(_) => "failure",
            Self::PartialSuccess => "partial_success",
        }
    }

    /// Check if the result indicates success
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    /// Get the error message if this is a failure
    pub fn error_message(&self) -> Option<&str> {
        match self {
            Self::Failure(msg) => Some(msg),
            _ => None,
        }
    }
}

/// Filter for querying audit events
#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    /// Filter by user ID
    pub user_id: Option<String>,

    /// Filter by event type
    pub event_type: Option<AuditEventType>,

    /// Filter by resource type
    pub resource_type: Option<ResourceType>,

    /// Filter by resource ID
    pub resource_id: Option<String>,

    /// Filter by start time (inclusive)
    pub start_time: Option<DateTime<Utc>>,

    /// Filter by end time (inclusive)
    pub end_time: Option<DateTime<Utc>>,

    /// Filter by result
    pub result: Option<AuditResult>,

    /// Maximum number of results to return
    pub limit: usize,

    /// Number of results to skip
    pub offset: usize,
}

impl AuditFilter {
    /// Create a new empty filter with default limit
    pub fn new() -> Self {
        Self {
            limit: 100,
            ..Default::default()
        }
    }

    /// Set the user ID filter
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Set the event type filter
    pub fn with_event_type(mut self, event_type: AuditEventType) -> Self {
        self.event_type = Some(event_type);
        self
    }

    /// Set the resource type filter
    pub fn with_resource_type(mut self, resource_type: ResourceType) -> Self {
        self.resource_type = Some(resource_type);
        self
    }

    /// Set the resource ID filter
    pub fn with_resource_id(mut self, resource_id: String) -> Self {
        self.resource_id = Some(resource_id);
        self
    }

    /// Set the time range filter
    pub fn with_time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.start_time = Some(start);
        self.end_time = Some(end);
        self
    }

    /// Set the result filter
    pub fn with_result(mut self, result: AuditResult) -> Self {
        self.result = Some(result);
        self
    }

    /// Set the limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Set the offset
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_creation() {
        let event = AuditEvent::new(
            AuditEventType::WorkflowExecution,
            "Execute workflow".to_string(),
            ResourceType::Workflow,
            "workflow-123".to_string(),
            AuditResult::Success,
        );

        assert_eq!(event.event_type, AuditEventType::WorkflowExecution);
        assert_eq!(event.action, "Execute workflow");
        assert_eq!(event.resource_type, ResourceType::Workflow);
        assert_eq!(event.resource_id, "workflow-123");
        assert!(event.result.is_success());
    }

    #[test]
    fn test_audit_event_builder() {
        let event = AuditEvent::new(
            AuditEventType::Authentication,
            "Login".to_string(),
            ResourceType::User,
            "user-456".to_string(),
            AuditResult::Success,
        )
        .with_user_id("user-456".to_string())
        .with_ip_address("192.168.1.1".to_string())
        .with_request_id("req-789".to_string());

        assert_eq!(event.user_id, Some("user-456".to_string()));
        assert_eq!(event.ip_address, Some("192.168.1.1".to_string()));
        assert_eq!(event.request_id, Some("req-789".to_string()));
    }

    #[test]
    fn test_audit_event_hash() {
        let event = AuditEvent::new(
            AuditEventType::WorkflowExecution,
            "Execute workflow".to_string(),
            ResourceType::Workflow,
            "workflow-123".to_string(),
            AuditResult::Success,
        );

        let hash1 = event.compute_hash();
        let hash2 = event.compute_hash();

        // Hash should be deterministic
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 produces 64 hex characters
    }

    #[test]
    fn test_audit_result_methods() {
        let success = AuditResult::Success;
        assert!(success.is_success());
        assert_eq!(success.error_message(), None);

        let failure = AuditResult::Failure("Error message".to_string());
        assert!(!failure.is_success());
        assert_eq!(failure.error_message(), Some("Error message"));
    }

    #[test]
    fn test_audit_filter_builder() {
        let filter = AuditFilter::new()
            .with_user_id("user-123".to_string())
            .with_event_type(AuditEventType::WorkflowExecution)
            .with_limit(50);

        assert_eq!(filter.user_id, Some("user-123".to_string()));
        assert_eq!(filter.event_type, Some(AuditEventType::WorkflowExecution));
        assert_eq!(filter.limit, 50);
    }
}
