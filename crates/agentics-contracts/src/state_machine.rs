// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! State Machine Agent contract schemas.
//!
//! # Agent Contract
//!
//! **Classification**: STATE_MANAGEMENT
//!
//! **Purpose**: Manage workflow state transitions according to defined state machines
//! and rules. Enforce valid workflow state transitions, validate state change requests,
//! track current and next workflow states, and emit authoritative state transition events.
//!
//! ## What This Agent MAY Do
//!
//! - Enforce valid workflow state transitions
//! - Validate state change requests against transition rules
//! - Track current and next workflow states
//! - Emit authoritative state transition events
//! - Maintain state machine definitions
//! - Provide state history queries
//!
//! ## What This Agent MUST NOT Do
//!
//! - Execute workflow steps directly (delegates to Workflow Orchestrator)
//! - Invoke downstream agents (state-driven only)
//! - Intercept runtime traffic directly (that is Edge)
//! - Enforce security policies (that is Shield)
//! - Emit anomaly detections (that is Sentinel)
//! - Perform optimization analysis (that is Auto-Optimizer)
//! - Connect directly to Google SQL
//! - Execute SQL statements
//! - Modify workflow schemas dynamically
//!
//! ## CLI Endpoints
//!
//! - `state-machine transition` - Request a state transition
//! - `state-machine validate` - Validate a transition request
//! - `state-machine inspect` - Inspect current state
//! - `state-machine replay` - Replay a previous state transition
//! - `state-machine history` - Get state transition history
//!
//! ## decision_type
//!
//! `"state_transition"`

use crate::agent::{AgentCapabilities, AgentClassification, AgentId, AgentMetadata, AgentType, AgentVersion};
use crate::workflow::{TaskState, WorkflowState};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use validator::Validate;

/// Agent ID for State Machine Agent.
pub const STATE_MACHINE_AGENT_ID: &str = "state-machine-agent";

/// Agent version for State Machine Agent.
pub const STATE_MACHINE_AGENT_VERSION: &str = "1.0.0";

// ============================================================================
// STATE MACHINE DEFINITIONS
// ============================================================================

/// Definition of a state machine for workflow or task state management.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct StateMachineDefinition {
    /// Unique identifier for this state machine definition.
    pub id: Uuid,

    /// Human-readable name.
    #[validate(length(min = 1, max = 256))]
    pub name: String,

    /// Version of this state machine definition.
    pub version: String,

    /// The initial state for new workflows/tasks.
    pub initial_state: String,

    /// Terminal states (workflow/task is complete).
    pub terminal_states: HashSet<String>,

    /// Valid state transitions.
    /// Key: source state, Value: list of valid target states.
    pub transitions: HashMap<String, Vec<String>>,

    /// Transition rules/guards.
    pub transition_rules: Vec<TransitionRule>,

    /// Description of this state machine.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Created timestamp.
    pub created_at: DateTime<Utc>,

    /// Last updated timestamp.
    pub updated_at: DateTime<Utc>,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for StateMachineDefinition {
    fn default() -> Self {
        Self::workflow_default()
    }
}

impl StateMachineDefinition {
    /// Creates the default workflow state machine definition.
    ///
    /// Valid transitions:
    /// - Pending -> Scheduled, Running, Cancelled
    /// - Scheduled -> Running, Cancelled
    /// - Running -> Paused, Completed, Failed, Cancelled, RetryWaiting
    /// - Paused -> Running, Cancelled
    /// - RetryWaiting -> Running, Failed, Cancelled
    /// - Completed -> (terminal)
    /// - Failed -> (terminal, but can transition to RetryWaiting for recovery)
    /// - Cancelled -> (terminal)
    pub fn workflow_default() -> Self {
        let mut transitions = HashMap::new();

        transitions.insert(
            "pending".to_string(),
            vec!["scheduled".to_string(), "running".to_string(), "cancelled".to_string()],
        );
        transitions.insert(
            "scheduled".to_string(),
            vec!["running".to_string(), "cancelled".to_string()],
        );
        transitions.insert(
            "running".to_string(),
            vec![
                "paused".to_string(),
                "completed".to_string(),
                "failed".to_string(),
                "cancelled".to_string(),
                "retry_waiting".to_string(),
            ],
        );
        transitions.insert(
            "paused".to_string(),
            vec!["running".to_string(), "cancelled".to_string()],
        );
        transitions.insert(
            "retry_waiting".to_string(),
            vec!["running".to_string(), "failed".to_string(), "cancelled".to_string()],
        );
        transitions.insert(
            "failed".to_string(),
            vec!["retry_waiting".to_string()], // Recovery path
        );

        let mut terminal_states = HashSet::new();
        terminal_states.insert("completed".to_string());
        terminal_states.insert("cancelled".to_string());
        // Note: "failed" is semi-terminal - can transition to retry_waiting for recovery

        let now = Utc::now();

        Self {
            id: Uuid::new_v4(),
            name: "Workflow State Machine".to_string(),
            version: "1.0.0".to_string(),
            initial_state: "pending".to_string(),
            terminal_states,
            transitions,
            transition_rules: Vec::new(),
            description: Some("Default workflow state machine with standard transitions".to_string()),
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    /// Creates the default task state machine definition.
    ///
    /// Valid transitions:
    /// - Pending -> Queued, Running, Skipped, Cancelled
    /// - Queued -> Running, Cancelled
    /// - Running -> Completed, Failed, Cancelled, RetryWaiting
    /// - RetryWaiting -> Running, Failed, Cancelled
    /// - Completed -> (terminal)
    /// - Failed -> RetryWaiting (recovery path)
    /// - Skipped -> (terminal)
    /// - Cancelled -> (terminal)
    pub fn task_default() -> Self {
        let mut transitions = HashMap::new();

        transitions.insert(
            "pending".to_string(),
            vec![
                "queued".to_string(),
                "running".to_string(),
                "skipped".to_string(),
                "cancelled".to_string(),
            ],
        );
        transitions.insert(
            "queued".to_string(),
            vec!["running".to_string(), "cancelled".to_string()],
        );
        transitions.insert(
            "running".to_string(),
            vec![
                "completed".to_string(),
                "failed".to_string(),
                "cancelled".to_string(),
                "retry_waiting".to_string(),
            ],
        );
        transitions.insert(
            "retry_waiting".to_string(),
            vec!["running".to_string(), "failed".to_string(), "cancelled".to_string()],
        );
        transitions.insert(
            "failed".to_string(),
            vec!["retry_waiting".to_string()], // Recovery path
        );

        let mut terminal_states = HashSet::new();
        terminal_states.insert("completed".to_string());
        terminal_states.insert("skipped".to_string());
        terminal_states.insert("cancelled".to_string());

        let now = Utc::now();

        Self {
            id: Uuid::new_v4(),
            name: "Task State Machine".to_string(),
            version: "1.0.0".to_string(),
            initial_state: "pending".to_string(),
            terminal_states,
            transitions,
            transition_rules: Vec::new(),
            description: Some("Default task state machine with standard transitions".to_string()),
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    /// Checks if a transition from source to target is valid.
    pub fn is_valid_transition(&self, source: &str, target: &str) -> bool {
        self.transitions
            .get(source)
            .map(|targets| targets.contains(&target.to_string()))
            .unwrap_or(false)
    }

    /// Returns all valid target states from a given source state.
    pub fn valid_targets(&self, source: &str) -> Vec<String> {
        self.transitions.get(source).cloned().unwrap_or_default()
    }

    /// Checks if a state is terminal.
    pub fn is_terminal(&self, state: &str) -> bool {
        self.terminal_states.contains(state)
    }
}

/// A transition rule/guard that must be satisfied for a transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionRule {
    /// Rule identifier.
    pub id: String,

    /// Source state this rule applies to.
    pub source_state: String,

    /// Target state this rule applies to.
    pub target_state: String,

    /// Rule type.
    pub rule_type: TransitionRuleType,

    /// Rule condition (depends on rule_type).
    pub condition: serde_json::Value,

    /// Error message if rule fails.
    pub error_message: String,

    /// Whether this rule is required (blocks transition) or advisory (warning).
    pub required: bool,
}

/// Types of transition rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionRuleType {
    /// All dependencies must be satisfied.
    DependenciesSatisfied,
    /// No pending retries.
    NoPendingRetries,
    /// Timeout not exceeded.
    TimeoutNotExceeded,
    /// Custom condition.
    Custom,
}

// ============================================================================
// TRANSITION REQUEST/RESPONSE
// ============================================================================

/// Request to transition a workflow or task state.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct StateTransitionRequest {
    /// Unique request identifier.
    pub request_id: Uuid,

    /// Workflow or task execution ID.
    pub execution_id: Uuid,

    /// Entity type (workflow or task).
    pub entity_type: EntityType,

    /// Current state (for validation).
    pub current_state: String,

    /// Target state.
    pub target_state: String,

    /// Reason for the transition.
    #[validate(length(min = 1, max = 1024))]
    pub reason: String,

    /// Transition initiator (agent ID, system, user).
    pub initiated_by: String,

    /// Force transition (skip validation).
    #[serde(default)]
    pub force: bool,

    /// Additional context for the transition.
    #[serde(default)]
    pub context: HashMap<String, serde_json::Value>,

    /// Request timestamp.
    pub timestamp: DateTime<Utc>,
}

impl StateTransitionRequest {
    /// Creates a new state transition request.
    pub fn new(
        execution_id: Uuid,
        entity_type: EntityType,
        current_state: impl Into<String>,
        target_state: impl Into<String>,
        reason: impl Into<String>,
        initiated_by: impl Into<String>,
    ) -> Self {
        Self {
            request_id: Uuid::new_v4(),
            execution_id,
            entity_type,
            current_state: current_state.into(),
            target_state: target_state.into(),
            reason: reason.into(),
            initiated_by: initiated_by.into(),
            force: false,
            context: HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    /// Sets the force flag.
    pub fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    /// Adds context.
    pub fn with_context(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.context.insert(key.into(), value);
        self
    }
}

/// Entity type for state management.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    /// Workflow-level state.
    Workflow,
    /// Task-level state.
    Task,
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Workflow => write!(f, "workflow"),
            Self::Task => write!(f, "task"),
        }
    }
}

/// Response to a state transition request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransitionResponse {
    /// Request ID.
    pub request_id: Uuid,

    /// Execution ID.
    pub execution_id: Uuid,

    /// Entity type.
    pub entity_type: EntityType,

    /// Whether transition succeeded.
    pub success: bool,

    /// Previous state.
    pub previous_state: String,

    /// New state (same as previous if failed).
    pub new_state: String,

    /// Transition status.
    pub status: TransitionStatus,

    /// Validation results.
    pub validation: TransitionValidation,

    /// Confidence score (execution certainty).
    pub confidence: f64,

    /// Inputs hash for reproducibility.
    pub inputs_hash: String,

    /// Transition timestamp.
    pub timestamp: DateTime<Utc>,

    /// Error message if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Constraints applied.
    pub constraints_applied: Vec<String>,

    /// Decision event ID (for audit trail).
    pub decision_event_id: Uuid,
}

/// Status of a state transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionStatus {
    /// Transition completed successfully.
    Completed,
    /// Transition was invalid.
    Invalid,
    /// Transition was blocked by a rule.
    Blocked,
    /// Transition was forced (skipped validation).
    Forced,
    /// State already in target state.
    NoChange,
    /// Error occurred.
    Error,
}

impl TransitionStatus {
    /// Returns true if transition was successful.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Completed | Self::Forced | Self::NoChange)
    }
}

/// Validation results for a transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionValidation {
    /// Whether the transition is valid according to state machine.
    pub valid: bool,

    /// Whether the source state matches current state.
    pub source_state_matches: bool,

    /// Whether target state exists in state machine.
    pub target_state_exists: bool,

    /// Rule violations.
    pub rule_violations: Vec<RuleViolation>,

    /// Warnings (non-blocking issues).
    pub warnings: Vec<String>,
}

impl Default for TransitionValidation {
    fn default() -> Self {
        Self {
            valid: true,
            source_state_matches: true,
            target_state_exists: true,
            rule_violations: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

/// A rule violation during transition validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleViolation {
    /// Rule ID.
    pub rule_id: String,

    /// Rule type.
    pub rule_type: TransitionRuleType,

    /// Error message.
    pub message: String,
}

// ============================================================================
// INSPECT/HISTORY TYPES
// ============================================================================

/// Request to inspect current state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateInspectRequest {
    /// Execution ID to inspect.
    pub execution_id: Uuid,

    /// Entity type.
    pub entity_type: EntityType,
}

/// Response to state inspection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateInspectResponse {
    /// Execution ID.
    pub execution_id: Uuid,

    /// Entity type.
    pub entity_type: EntityType,

    /// Current state.
    pub current_state: String,

    /// Whether state is terminal.
    pub is_terminal: bool,

    /// Valid next states.
    pub valid_next_states: Vec<String>,

    /// State machine definition ID.
    pub state_machine_id: Uuid,

    /// Last transition timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_transition_at: Option<DateTime<Utc>>,

    /// Last transition initiator.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_transition_by: Option<String>,

    /// Total transitions count.
    pub transition_count: u32,

    /// Time in current state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_in_state_ms: Option<u64>,
}

/// Request for state history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateHistoryRequest {
    /// Execution ID.
    pub execution_id: Uuid,

    /// Entity type.
    pub entity_type: EntityType,

    /// Maximum entries to return.
    #[serde(default = "default_history_limit")]
    pub limit: usize,

    /// Offset for pagination.
    #[serde(default)]
    pub offset: usize,
}

fn default_history_limit() -> usize {
    100
}

/// Response with state history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateHistoryResponse {
    /// Execution ID.
    pub execution_id: Uuid,

    /// Entity type.
    pub entity_type: EntityType,

    /// History entries.
    pub entries: Vec<StateHistoryEntry>,

    /// Total entries available.
    pub total_count: usize,

    /// Current offset.
    pub offset: usize,

    /// Limit used.
    pub limit: usize,
}

/// A single state history entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateHistoryEntry {
    /// Entry ID.
    pub id: Uuid,

    /// Previous state.
    pub from_state: String,

    /// New state.
    pub to_state: String,

    /// Reason for transition.
    pub reason: String,

    /// Who initiated the transition.
    pub initiated_by: String,

    /// Transition timestamp.
    pub timestamp: DateTime<Utc>,

    /// Duration in previous state (milliseconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_in_previous_state_ms: Option<u64>,

    /// Associated decision event ID.
    pub decision_event_id: Uuid,
}

// ============================================================================
// REPLAY TYPES
// ============================================================================

/// Request to replay a state transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateReplayRequest {
    /// Original decision event ID to replay.
    pub original_decision_id: Uuid,
}

/// Response from replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateReplayResponse {
    /// Original decision ID.
    pub original_decision_id: Uuid,

    /// New decision ID.
    pub new_decision_id: Uuid,

    /// Whether inputs hash matches.
    pub inputs_hash_match: bool,

    /// Original inputs hash.
    pub original_inputs_hash: String,

    /// New inputs hash.
    pub new_inputs_hash: String,

    /// Replay timestamp.
    pub timestamp: DateTime<Utc>,
}

// ============================================================================
// DECISION EVENT
// ============================================================================

/// Decision event specific to state transitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransitionDecisionEvent {
    /// Event ID.
    pub id: Uuid,

    /// Agent ID.
    pub agent_id: String,

    /// Agent version.
    pub agent_version: String,

    /// Decision type (always "state_transition").
    pub decision_type: String,

    /// Inputs hash.
    pub inputs_hash: String,

    /// Transition details.
    pub transition: TransitionDetails,

    /// Confidence score.
    pub confidence: f64,

    /// Constraints applied.
    pub constraints_applied: StateTransitionConstraints,

    /// Execution reference.
    pub execution_ref: StateTransitionExecutionRef,

    /// Event timestamp.
    pub timestamp: DateTime<Utc>,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Details of the state transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionDetails {
    /// Execution ID.
    pub execution_id: Uuid,

    /// Entity type.
    pub entity_type: EntityType,

    /// Previous state.
    pub from_state: String,

    /// New state.
    pub to_state: String,

    /// Transition status.
    pub status: TransitionStatus,

    /// Reason for transition.
    pub reason: String,

    /// Who initiated.
    pub initiated_by: String,
}

/// Constraints applied during state transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransitionConstraints {
    /// State machine used.
    pub state_machine_id: Uuid,

    /// State machine version.
    pub state_machine_version: String,

    /// Rules evaluated.
    pub rules_evaluated: usize,

    /// Rules passed.
    pub rules_passed: usize,

    /// Whether force was used.
    pub force_applied: bool,

    /// Validation performed.
    pub validation_type: String,
}

/// Execution reference for state transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransitionExecutionRef {
    /// Workflow execution ID.
    pub workflow_execution_id: Uuid,

    /// Task execution ID (if task-level transition).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_execution_id: Option<Uuid>,

    /// Trace ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,

    /// Span ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
}

// ============================================================================
// AGENT METADATA
// ============================================================================

impl AgentMetadata {
    /// Creates metadata for the State Machine Agent.
    ///
    /// Classification: STATE_MANAGEMENT
    ///
    /// The State Machine Agent manages workflow state transitions according
    /// to defined state machines and rules. It enforces valid workflow state
    /// transitions, validates state change requests, tracks current and next
    /// workflow states, and emits authoritative state transition events.
    pub fn state_machine() -> Self {
        let mut allowed_invokers = HashSet::new();
        allowed_invokers.insert("workflow-orchestrator".to_string());
        allowed_invokers.insert("task-scheduler".to_string());
        allowed_invokers.insert("retry-recovery".to_string());
        allowed_invokers.insert("governance-systems".to_string());

        Self {
            id: AgentId::new("state-machine"),
            version: AgentVersion::new(1, 0, 0),
            agent_type: AgentType::StateManager,
            classification: AgentClassification::StateManagement,
            name: "State Machine Agent".to_string(),
            description: Some(
                "Manage workflow state transitions according to defined state machines and rules. \
                Enforces valid workflow state transitions, validates state change requests, \
                tracks current and next workflow states, and emits authoritative state transition events."
                    .to_string(),
            ),
            capabilities: AgentCapabilities::state_manager(),
            allowed_invokers,
            cli_endpoint: "state-machine".to_string(),
            non_responsibilities: vec![
                "Execute workflow steps directly (delegates to Workflow Orchestrator)".to_string(),
                "Invoke downstream agents (state-driven only)".to_string(),
                "Intercept runtime traffic directly (that is Edge)".to_string(),
                "Enforce security policies (that is Shield)".to_string(),
                "Emit anomaly detections (that is Sentinel)".to_string(),
                "Perform optimization analysis (that is Auto-Optimizer)".to_string(),
                "Connect directly to Google SQL".to_string(),
                "Execute SQL statements".to_string(),
                "Modify workflow schemas dynamically".to_string(),
            ],
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_state_machine_default() {
        let sm = StateMachineDefinition::workflow_default();

        assert_eq!(sm.initial_state, "pending");
        assert!(sm.is_valid_transition("pending", "running"));
        assert!(sm.is_valid_transition("running", "completed"));
        assert!(sm.is_valid_transition("running", "failed"));
        assert!(!sm.is_valid_transition("completed", "running")); // Terminal
        assert!(sm.is_terminal("completed"));
        assert!(sm.is_terminal("cancelled"));
        assert!(!sm.is_terminal("running"));
    }

    #[test]
    fn test_task_state_machine_default() {
        let sm = StateMachineDefinition::task_default();

        assert_eq!(sm.initial_state, "pending");
        assert!(sm.is_valid_transition("pending", "queued"));
        assert!(sm.is_valid_transition("queued", "running"));
        assert!(sm.is_valid_transition("running", "completed"));
        assert!(sm.is_terminal("completed"));
        assert!(sm.is_terminal("skipped"));
        assert!(!sm.is_terminal("running"));
    }

    #[test]
    fn test_valid_targets() {
        let sm = StateMachineDefinition::workflow_default();

        let targets = sm.valid_targets("running");
        assert!(targets.contains(&"completed".to_string()));
        assert!(targets.contains(&"failed".to_string()));
        assert!(targets.contains(&"paused".to_string()));
    }

    #[test]
    fn test_state_transition_request() {
        let request = StateTransitionRequest::new(
            Uuid::new_v4(),
            EntityType::Workflow,
            "running",
            "completed",
            "Workflow completed successfully",
            "workflow-orchestrator",
        )
        .with_force(false)
        .with_context("steps_executed", serde_json::json!(5));

        assert_eq!(request.current_state, "running");
        assert_eq!(request.target_state, "completed");
        assert!(!request.force);
        assert!(request.context.contains_key("steps_executed"));
    }

    #[test]
    fn test_transition_status_is_success() {
        assert!(TransitionStatus::Completed.is_success());
        assert!(TransitionStatus::Forced.is_success());
        assert!(TransitionStatus::NoChange.is_success());
        assert!(!TransitionStatus::Invalid.is_success());
        assert!(!TransitionStatus::Blocked.is_success());
        assert!(!TransitionStatus::Error.is_success());
    }

    #[test]
    fn test_entity_type_display() {
        assert_eq!(EntityType::Workflow.to_string(), "workflow");
        assert_eq!(EntityType::Task.to_string(), "task");
    }

    #[test]
    fn test_state_machine_agent_metadata() {
        let metadata = AgentMetadata::state_machine();

        assert_eq!(metadata.agent_type, AgentType::StateManager);
        assert_eq!(metadata.classification, AgentClassification::StateManagement);
        assert_eq!(metadata.name, "State Machine Agent");
        assert_eq!(metadata.cli_endpoint, "state-machine");

        // Verify capabilities
        assert!(!metadata.capabilities.execute_workflows);
        assert!(!metadata.capabilities.invoke_downstream_agents);
        assert!(!metadata.capabilities.manage_task_dependencies);
        assert!(!metadata.capabilities.trigger_retries);
        assert!(!metadata.capabilities.coordinate_parallel_execution);
        assert!(metadata.capabilities.transition_workflow_states);

        // Verify allowed invokers
        assert!(metadata.allowed_invokers.contains("workflow-orchestrator"));
        assert!(metadata.allowed_invokers.contains("task-scheduler"));
        assert!(metadata.allowed_invokers.contains("retry-recovery"));

        // Verify non-responsibilities are defined
        assert!(!metadata.non_responsibilities.is_empty());
        assert!(metadata
            .non_responsibilities
            .iter()
            .any(|r| r.contains("Execute workflow steps directly")));
        assert!(metadata
            .non_responsibilities
            .iter()
            .any(|r| r.contains("Connect directly to Google SQL")));
    }

    #[test]
    fn test_recovery_path() {
        let sm = StateMachineDefinition::workflow_default();

        // Verify failed -> retry_waiting is valid (recovery path)
        assert!(sm.is_valid_transition("failed", "retry_waiting"));
        // Verify retry_waiting -> running is valid
        assert!(sm.is_valid_transition("retry_waiting", "running"));
    }
}
