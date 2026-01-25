// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! DecisionEvent schema for agent invocation tracking.
//!
//! Every agent invocation MUST emit exactly ONE DecisionEvent to ruvector-service.

use crate::agent::{AgentId, AgentVersion};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

/// A decision event emitted by an agent invocation.
///
/// This is the MANDATORY output for every agent invocation.
/// The schema MUST include all required fields as per the constitution.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DecisionEvent {
    /// Unique identifier for this decision event.
    pub id: Uuid,

    /// Agent identifier that made this decision.
    pub agent_id: AgentId,

    /// Agent version that made this decision.
    pub agent_version: AgentVersion,

    /// Type of decision made.
    pub decision_type: DecisionType,

    /// SHA-256 hash of the inputs for reproducibility.
    #[validate(length(equal = 64))]
    pub inputs_hash: String,

    /// Decision outputs (machine-readable).
    pub outputs: DecisionOutputs,

    /// Execution certainty score (0.0 to 1.0).
    #[validate(range(min = 0.0, max = 1.0))]
    pub confidence: f64,

    /// Constraints applied during execution.
    pub constraints_applied: ConstraintsApplied,

    /// Reference to the execution context.
    pub execution_ref: ExecutionRef,

    /// Timestamp of the decision (UTC).
    pub timestamp: DateTime<Utc>,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl DecisionEvent {
    /// Creates a new decision event with required fields.
    pub fn new(
        agent_id: AgentId,
        agent_version: AgentVersion,
        decision_type: DecisionType,
        inputs_hash: String,
        execution_ref: ExecutionRef,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            agent_id,
            agent_version,
            decision_type,
            inputs_hash,
            outputs: DecisionOutputs::default(),
            confidence: 0.0,
            constraints_applied: ConstraintsApplied::default(),
            execution_ref,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Sets the outputs for this decision.
    pub fn with_outputs(mut self, outputs: DecisionOutputs) -> Self {
        self.outputs = outputs;
        self
    }

    /// Sets the confidence score.
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Sets the constraints applied.
    pub fn with_constraints(mut self, constraints: ConstraintsApplied) -> Self {
        self.constraints_applied = constraints;
        self
    }

    /// Adds metadata to the event.
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Validates the decision event.
    pub fn validate(&self) -> Result<(), crate::error::ContractError> {
        <Self as Validate>::validate(self)
            .map_err(|e| crate::error::ContractError::Validation(e.to_string()))
    }
}

/// Decision type semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionType {
    /// Execute a workflow step or sequence.
    Execute,
    /// Transition workflow state.
    Transition,
    /// Schedule a task for later execution.
    Schedule,
    /// Retry a failed operation.
    Retry,
    /// Skip a step due to condition.
    Skip,
    /// Cancel an in-progress operation.
    Cancel,
    /// Complete a workflow or task.
    Complete,
    /// Fail a workflow or task.
    Fail,

    // Phase 3 Layer 1 Signal Types
    /// Execution strategy signal - routing and coordination (Phase 3).
    ExecutionStrategySignal,
    /// Optimization signal - performance optimization (Phase 3).
    OptimizationSignal,
    /// Incident signal - escalation and incidents (Phase 3).
    IncidentSignal,
}

impl std::fmt::Display for DecisionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Execute => write!(f, "execute"),
            Self::Transition => write!(f, "transition"),
            Self::Schedule => write!(f, "schedule"),
            Self::Retry => write!(f, "retry"),
            Self::Skip => write!(f, "skip"),
            Self::Cancel => write!(f, "cancel"),
            Self::Complete => write!(f, "complete"),
            Self::Fail => write!(f, "fail"),
            // Phase 3 Signal Types
            Self::ExecutionStrategySignal => write!(f, "execution_strategy_signal"),
            Self::OptimizationSignal => write!(f, "optimization_signal"),
            Self::IncidentSignal => write!(f, "incident_signal"),
        }
    }
}

/// Decision outputs structure.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecisionOutputs {
    /// Primary output value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<serde_json::Value>,

    /// Secondary outputs keyed by name.
    #[serde(default)]
    pub secondary: HashMap<String, serde_json::Value>,

    /// Outcome status.
    pub outcome: DecisionOutcome,

    /// Error details if outcome is failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorDetails>,

    /// Artifacts generated (references, not data).
    #[serde(default)]
    pub artifacts: Vec<ArtifactRef>,
}

/// Decision outcome status.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionOutcome {
    /// Decision executed successfully.
    Success,
    /// Decision failed with error.
    Failure,
    /// Decision is pending execution.
    #[default]
    Pending,
    /// Decision was skipped.
    Skipped,
    /// Decision is in progress.
    InProgress,
}

/// Error details for failed decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetails {
    /// Error code.
    pub code: String,
    /// Human-readable error message.
    pub message: String,
    /// Error category.
    pub category: ErrorCategory,
    /// Whether the error is retryable.
    pub retryable: bool,
    /// Stack trace (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<String>,
}

/// Error category classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// Validation error in inputs.
    Validation,
    /// Execution error during processing.
    Execution,
    /// Timeout during execution.
    Timeout,
    /// Dependency error (downstream agent failure).
    Dependency,
    /// Resource error (quota, limits).
    Resource,
    /// Internal error.
    Internal,
}

/// Reference to an artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRef {
    /// Artifact identifier.
    pub id: String,
    /// Artifact type.
    pub artifact_type: String,
    /// Storage location (ruvector-service key).
    pub location: String,
    /// Content hash for integrity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
}

/// Constraints applied during execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConstraintsApplied {
    /// Workflow constraints.
    pub workflow: WorkflowConstraints,
    /// Dependency constraints.
    pub dependencies: DependencyConstraints,
    /// Retry constraints.
    pub retry: RetryConstraints,
    /// Resource constraints.
    pub resources: ResourceConstraints,
}

/// Workflow-level constraints.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowConstraints {
    /// Workflow timeout applied (seconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
    /// Maximum concurrent steps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_concurrency: Option<usize>,
    /// Whether strict ordering is enforced.
    pub strict_ordering: bool,
    /// Execution strategy used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_strategy: Option<String>,
}

/// Dependency constraints.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyConstraints {
    /// Number of dependencies resolved.
    pub dependencies_resolved: usize,
    /// Dependencies that blocked execution.
    #[serde(default)]
    pub blocked_by: Vec<String>,
    /// Whether all dependencies were satisfied.
    pub all_satisfied: bool,
}

/// Retry constraints.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RetryConstraints {
    /// Current retry attempt number.
    pub attempt: u32,
    /// Maximum retry attempts configured.
    pub max_attempts: u32,
    /// Backoff strategy used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backoff_strategy: Option<String>,
    /// Next retry delay (milliseconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_delay_ms: Option<u64>,
}

/// Resource constraints.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceConstraints {
    /// Memory limit applied (bytes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_limit_bytes: Option<u64>,
    /// CPU limit applied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_limit: Option<f64>,
    /// Token limit for LLM calls.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_limit: Option<u64>,
}

/// Execution reference for tracing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRef {
    /// Workflow execution ID.
    pub workflow_execution_id: Uuid,
    /// Step execution ID (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_execution_id: Option<Uuid>,
    /// Parent execution reference (for nested workflows).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_ref: Option<Box<ExecutionRef>>,
    /// Trace ID for distributed tracing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// Span ID for distributed tracing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
}

impl ExecutionRef {
    /// Creates a new execution reference.
    pub fn new(workflow_execution_id: Uuid) -> Self {
        Self {
            workflow_execution_id,
            step_execution_id: None,
            parent_ref: None,
            trace_id: Some(Uuid::new_v4().to_string()),
            span_id: Some(Uuid::new_v4().to_string()),
        }
    }

    /// Adds a step execution ID.
    pub fn with_step(mut self, step_id: Uuid) -> Self {
        self.step_execution_id = Some(step_id);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{AgentId, AgentVersion};

    #[test]
    fn test_decision_event_creation() {
        let agent_id = AgentId::from_string("test-agent");
        let agent_version = AgentVersion::new(1, 0, 0);
        let execution_ref = ExecutionRef::new(Uuid::new_v4());

        let event = DecisionEvent::new(
            agent_id.clone(),
            agent_version,
            DecisionType::Execute,
            "0".repeat(64),
            execution_ref,
        )
        .with_confidence(0.95)
        .with_metadata("test", serde_json::json!("value"));

        assert_eq!(event.agent_id, agent_id);
        assert_eq!(event.confidence, 0.95);
        assert!(event.metadata.contains_key("test"));
    }

    #[test]
    fn test_decision_type_display() {
        assert_eq!(DecisionType::Execute.to_string(), "execute");
        assert_eq!(DecisionType::Transition.to_string(), "transition");
        assert_eq!(DecisionType::Retry.to_string(), "retry");
    }

    #[test]
    fn test_confidence_clamping() {
        let agent_id = AgentId::from_string("test");
        let agent_version = AgentVersion::new(1, 0, 0);
        let execution_ref = ExecutionRef::new(Uuid::new_v4());

        let event = DecisionEvent::new(
            agent_id,
            agent_version,
            DecisionType::Execute,
            "0".repeat(64),
            execution_ref,
        )
        .with_confidence(1.5); // Should be clamped to 1.0

        assert_eq!(event.confidence, 1.0);
    }
}
