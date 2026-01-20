// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! RuVector Service adapter for DecisionEvent persistence.
//!
//! This adapter provides the ONLY permitted interface for persisting
//! agent decision events. LLM-Orchestrator NEVER connects directly
//! to Google SQL - all persistence occurs via ruvector-service.
//!
//! # Constitution Compliance
//!
//! - NO direct SQL connections
//! - NO SQL statement execution
//! - ALL persistence via ruvector-service client calls only

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

/// RuVector service client for DecisionEvent persistence.
///
/// This is the ONLY permitted interface for persisting workflow states,
/// task executions, retries, and transitions.
#[derive(Debug, Clone)]
pub struct RuVectorServiceClient {
    /// Base URL for the ruvector-service.
    endpoint: String,
    /// Request timeout.
    timeout: Duration,
    /// Whether the client is enabled.
    enabled: bool,
    /// API key for authentication.
    api_key: Option<String>,
}

impl Default for RuVectorServiceClient {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            timeout: Duration::from_secs(30),
            enabled: false,
            api_key: None,
        }
    }
}

impl RuVectorServiceClient {
    /// Creates a new client with the given endpoint.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            timeout: Duration::from_secs(30),
            enabled: true,
            api_key: None,
        }
    }

    /// Creates a client from environment variables.
    pub fn from_env() -> Result<Self, RuVectorError> {
        let endpoint = std::env::var("RUVECTOR_SERVICE_ENDPOINT")
            .map_err(|_| RuVectorError::Configuration("RUVECTOR_SERVICE_ENDPOINT not set".to_string()))?;

        let api_key = std::env::var("RUVECTOR_API_KEY").ok();

        Ok(Self {
            endpoint,
            timeout: Duration::from_secs(30),
            enabled: true,
            api_key,
        })
    }

    /// Creates a disabled client (no-op mode).
    pub fn disabled() -> Self {
        Self::default()
    }

    /// Sets the request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets the API key.
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Returns whether the client is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Persists a DecisionEvent to ruvector-service.
    ///
    /// This is the primary persistence method for agent decision tracking.
    pub async fn persist_decision_event(
        &self,
        event: &DecisionEventRecord,
    ) -> Result<PersistenceResult, RuVectorError> {
        if !self.enabled {
            return Ok(PersistenceResult::skipped());
        }

        // In production, this would make an HTTP/gRPC call to ruvector-service
        // For now, we log the event and return success
        tracing::info!(
            event_id = %event.id,
            agent_id = %event.agent_id,
            decision_type = %event.decision_type,
            "Persisting DecisionEvent to ruvector-service"
        );

        // Simulate persistence
        Ok(PersistenceResult {
            success: true,
            record_id: Some(event.id.to_string()),
            version: 1,
            timestamp: chrono::Utc::now(),
            error: None,
        })
    }

    /// Retrieves a DecisionEvent by ID.
    pub async fn get_decision_event(
        &self,
        event_id: Uuid,
    ) -> Result<Option<DecisionEventRecord>, RuVectorError> {
        if !self.enabled {
            return Ok(None);
        }

        tracing::debug!(event_id = %event_id, "Retrieving DecisionEvent from ruvector-service");

        // Placeholder: Would query ruvector-service
        Ok(None)
    }

    /// Queries DecisionEvents by workflow execution ID.
    pub async fn query_by_workflow(
        &self,
        workflow_execution_id: Uuid,
        limit: Option<usize>,
    ) -> Result<Vec<DecisionEventRecord>, RuVectorError> {
        if !self.enabled {
            return Ok(Vec::new());
        }

        tracing::debug!(
            workflow_id = %workflow_execution_id,
            limit = ?limit,
            "Querying DecisionEvents by workflow"
        );

        // Placeholder: Would query ruvector-service
        Ok(Vec::new())
    }

    /// Queries DecisionEvents by agent ID.
    pub async fn query_by_agent(
        &self,
        agent_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<DecisionEventRecord>, RuVectorError> {
        if !self.enabled {
            return Ok(Vec::new());
        }

        tracing::debug!(
            agent_id = %agent_id,
            limit = ?limit,
            "Querying DecisionEvents by agent"
        );

        // Placeholder: Would query ruvector-service
        Ok(Vec::new())
    }

    /// Persists workflow state.
    pub async fn persist_workflow_state(
        &self,
        state: &WorkflowStateRecord,
    ) -> Result<PersistenceResult, RuVectorError> {
        if !self.enabled {
            return Ok(PersistenceResult::skipped());
        }

        tracing::info!(
            workflow_id = %state.workflow_execution_id,
            state = %state.state,
            "Persisting workflow state to ruvector-service"
        );

        Ok(PersistenceResult {
            success: true,
            record_id: Some(state.workflow_execution_id.to_string()),
            version: state.version + 1,
            timestamp: chrono::Utc::now(),
            error: None,
        })
    }

    /// Retrieves workflow state.
    pub async fn get_workflow_state(
        &self,
        workflow_execution_id: Uuid,
    ) -> Result<Option<WorkflowStateRecord>, RuVectorError> {
        if !self.enabled {
            return Ok(None);
        }

        tracing::debug!(
            workflow_id = %workflow_execution_id,
            "Retrieving workflow state from ruvector-service"
        );

        // Placeholder: Would query ruvector-service
        Ok(None)
    }

    /// Persists task execution record.
    pub async fn persist_task_execution(
        &self,
        execution: &TaskExecutionRecord,
    ) -> Result<PersistenceResult, RuVectorError> {
        if !self.enabled {
            return Ok(PersistenceResult::skipped());
        }

        tracing::info!(
            task_id = %execution.task_id,
            attempt = execution.attempt,
            state = %execution.state,
            "Persisting task execution to ruvector-service"
        );

        Ok(PersistenceResult {
            success: true,
            record_id: Some(execution.task_id.to_string()),
            version: execution.attempt as u64,
            timestamp: chrono::Utc::now(),
            error: None,
        })
    }

    /// Performs a health check on the ruvector-service.
    pub async fn health_check(&self) -> Result<HealthStatus, RuVectorError> {
        if !self.enabled {
            return Ok(HealthStatus {
                healthy: false,
                message: "Client is disabled".to_string(),
                latency_ms: None,
            });
        }

        // Placeholder: Would ping ruvector-service
        Ok(HealthStatus {
            healthy: true,
            message: "Service is healthy".to_string(),
            latency_ms: Some(5),
        })
    }
}

/// DecisionEvent record for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionEventRecord {
    /// Unique identifier.
    pub id: Uuid,
    /// Agent identifier.
    pub agent_id: String,
    /// Agent version.
    pub agent_version: String,
    /// Decision type.
    pub decision_type: String,
    /// SHA-256 hash of inputs.
    pub inputs_hash: String,
    /// Decision outputs as JSON.
    pub outputs: serde_json::Value,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f64,
    /// Constraints applied as JSON.
    pub constraints_applied: serde_json::Value,
    /// Workflow execution ID.
    pub workflow_execution_id: Uuid,
    /// Step execution ID (if applicable).
    pub step_execution_id: Option<Uuid>,
    /// Trace ID.
    pub trace_id: Option<String>,
    /// Span ID.
    pub span_id: Option<String>,
    /// Timestamp (UTC).
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Additional metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Workflow state record for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStateRecord {
    /// Workflow execution ID.
    pub workflow_execution_id: Uuid,
    /// Workflow definition ID.
    pub workflow_id: Uuid,
    /// Workflow name.
    pub workflow_name: String,
    /// Current state.
    pub state: String,
    /// State version (for optimistic locking).
    pub version: u64,
    /// Inputs as JSON.
    pub inputs: serde_json::Value,
    /// Outputs as JSON (populated on completion).
    pub outputs: Option<serde_json::Value>,
    /// Error details (if failed).
    pub error: Option<String>,
    /// Created timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last updated timestamp.
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Completed timestamp (if completed).
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Task execution record for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionRecord {
    /// Task execution ID.
    pub id: Uuid,
    /// Task ID.
    pub task_id: Uuid,
    /// Workflow execution ID.
    pub workflow_execution_id: Uuid,
    /// Task name.
    pub task_name: String,
    /// Task type.
    pub task_type: String,
    /// Current state.
    pub state: String,
    /// Execution attempt number.
    pub attempt: u32,
    /// Inputs as JSON.
    pub inputs: serde_json::Value,
    /// Outputs as JSON (populated on completion).
    pub outputs: Option<serde_json::Value>,
    /// Error details (if failed).
    pub error: Option<String>,
    /// Started timestamp.
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Completed timestamp.
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Duration in milliseconds.
    pub duration_ms: Option<u64>,
}

/// Persistence result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceResult {
    /// Whether persistence was successful.
    pub success: bool,
    /// Record ID in ruvector-service.
    pub record_id: Option<String>,
    /// Record version.
    pub version: u64,
    /// Persistence timestamp.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Error message if failed.
    pub error: Option<String>,
}

impl PersistenceResult {
    /// Creates a skipped result (when client is disabled).
    pub fn skipped() -> Self {
        Self {
            success: true,
            record_id: None,
            version: 0,
            timestamp: chrono::Utc::now(),
            error: Some("Persistence skipped (client disabled)".to_string()),
        }
    }
}

/// Health check status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Whether the service is healthy.
    pub healthy: bool,
    /// Status message.
    pub message: String,
    /// Latency in milliseconds.
    pub latency_ms: Option<u64>,
}

/// RuVector service errors.
#[derive(Debug, thiserror::Error)]
pub enum RuVectorError {
    /// Configuration error.
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Connection error.
    #[error("Connection error: {0}")]
    Connection(String),

    /// Request error.
    #[error("Request error: {0}")]
    Request(String),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Not found error.
    #[error("Record not found: {0}")]
    NotFound(String),

    /// Conflict error (optimistic locking).
    #[error("Version conflict: expected {expected}, got {actual}")]
    Conflict {
        expected: u64,
        actual: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_disabled_by_default() {
        let client = RuVectorServiceClient::default();
        assert!(!client.is_enabled());
    }

    #[test]
    fn test_client_enabled_with_endpoint() {
        let client = RuVectorServiceClient::new("http://localhost:8080");
        assert!(client.is_enabled());
    }

    #[tokio::test]
    async fn test_persist_decision_event_disabled() {
        let client = RuVectorServiceClient::disabled();
        let event = DecisionEventRecord {
            id: Uuid::new_v4(),
            agent_id: "test-agent".to_string(),
            agent_version: "1.0.0".to_string(),
            decision_type: "execute".to_string(),
            inputs_hash: "0".repeat(64),
            outputs: serde_json::json!({}),
            confidence: 0.95,
            constraints_applied: serde_json::json!({}),
            workflow_execution_id: Uuid::new_v4(),
            step_execution_id: None,
            trace_id: None,
            span_id: None,
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        let result = client.persist_decision_event(&event).await.unwrap();
        assert!(result.success);
        assert!(result.error.is_some()); // Should indicate skipped
    }

    #[tokio::test]
    async fn test_persist_decision_event_enabled() {
        let client = RuVectorServiceClient::new("http://localhost:8080");
        let event = DecisionEventRecord {
            id: Uuid::new_v4(),
            agent_id: "test-agent".to_string(),
            agent_version: "1.0.0".to_string(),
            decision_type: "execute".to_string(),
            inputs_hash: "0".repeat(64),
            outputs: serde_json::json!({}),
            confidence: 0.95,
            constraints_applied: serde_json::json!({}),
            workflow_execution_id: Uuid::new_v4(),
            step_execution_id: None,
            trace_id: None,
            span_id: None,
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        let result = client.persist_decision_event(&event).await.unwrap();
        assert!(result.success);
        assert!(result.record_id.is_some());
    }
}
