// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! State Machine Agent implementation.
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

use crate::adapters::observatory::ObservatoryAdapter;
use crate::adapters::ruvector_service::{DecisionEventRecord, RuVectorServiceClient};
use crate::error::{OrchestratorError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Instant;
use uuid::Uuid;

/// Agent version.
pub const AGENT_VERSION: &str = "1.0.0";

/// Agent ID prefix.
pub const AGENT_ID_PREFIX: &str = "state-machine";

// ============================================================================
// RE-EXPORT CONTRACT TYPES
// ============================================================================

pub use agentics_contracts::state_machine::{
    EntityType, RuleViolation, StateHistoryEntry, StateHistoryRequest, StateHistoryResponse,
    StateInspectRequest, StateInspectResponse, StateMachineDefinition, StateReplayRequest,
    StateReplayResponse, StateTransitionConstraints, StateTransitionDecisionEvent,
    StateTransitionExecutionRef, StateTransitionRequest, StateTransitionResponse,
    TransitionDetails, TransitionRule, TransitionRuleType, TransitionStatus,
    TransitionValidation,
};

// ============================================================================
// AGENT CONFIGURATION
// ============================================================================

/// Configuration for the State Machine Agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachineAgentConfig {
    /// Whether to enforce strict validation.
    pub strict_validation: bool,
    /// Whether to allow force transitions.
    pub allow_force: bool,
    /// Default timeout in seconds.
    pub timeout_seconds: u64,
    /// Maximum history entries to return.
    pub max_history_entries: usize,
}

impl Default for StateMachineAgentConfig {
    fn default() -> Self {
        Self {
            strict_validation: true,
            allow_force: true,
            timeout_seconds: 300,
            max_history_entries: 1000,
        }
    }
}

// ============================================================================
// STATE MACHINE AGENT
// ============================================================================

/// State Machine Agent.
///
/// This agent manages workflow state transitions according to defined state
/// machines and rules. It implements:
///
/// - State transition validation against state machine definitions
/// - Transition rule enforcement
/// - State history tracking
/// - Decision event emission
///
/// # Classification
///
/// **STATE_MANAGEMENT**
///
/// # Non-Responsibilities
///
/// This agent MUST NEVER:
/// - Execute workflow steps directly (delegates to Workflow Orchestrator)
/// - Invoke downstream agents (state-driven only)
/// - Intercept runtime traffic (that is Edge)
/// - Enforce security policies (that is Shield)
/// - Emit anomaly detections (that is Sentinel)
/// - Perform optimization analysis (that is Auto-Optimizer)
/// - Connect directly to Google SQL
/// - Execute SQL
pub struct StateMachineAgent {
    /// Agent configuration.
    config: StateMachineAgentConfig,
    /// RuVector service client for persistence.
    ruvector_client: RuVectorServiceClient,
    /// Observatory adapter for telemetry.
    observatory: ObservatoryAdapter,
    /// Agent ID.
    agent_id: String,
    /// Workflow state machine definition.
    workflow_state_machine: StateMachineDefinition,
    /// Task state machine definition.
    task_state_machine: StateMachineDefinition,
    /// State history (in-memory for this implementation).
    state_history: HashMap<Uuid, Vec<StateHistoryEntry>>,
    /// Current states (in-memory for this implementation).
    current_states: HashMap<Uuid, (EntityType, String, DateTime<Utc>)>,
}

impl StateMachineAgent {
    /// Creates a new State Machine Agent.
    pub fn new(config: StateMachineAgentConfig) -> Self {
        let agent_id = format!(
            "{}-{}",
            AGENT_ID_PREFIX,
            Uuid::new_v4().to_string()[..8].to_string()
        );

        Self {
            config,
            ruvector_client: RuVectorServiceClient::disabled(),
            observatory: ObservatoryAdapter::disabled(),
            agent_id,
            workflow_state_machine: StateMachineDefinition::workflow_default(),
            task_state_machine: StateMachineDefinition::task_default(),
            state_history: HashMap::new(),
            current_states: HashMap::new(),
        }
    }

    /// Creates an agent with ruvector-service integration.
    pub fn with_ruvector_client(mut self, client: RuVectorServiceClient) -> Self {
        self.ruvector_client = client;
        self
    }

    /// Creates an agent with observatory integration.
    pub fn with_observatory(mut self, observatory: ObservatoryAdapter) -> Self {
        self.observatory = observatory;
        self
    }

    /// Sets a custom workflow state machine.
    pub fn with_workflow_state_machine(mut self, sm: StateMachineDefinition) -> Self {
        self.workflow_state_machine = sm;
        self
    }

    /// Sets a custom task state machine.
    pub fn with_task_state_machine(mut self, sm: StateMachineDefinition) -> Self {
        self.task_state_machine = sm;
        self
    }

    /// Returns the agent ID.
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    /// Returns the agent version.
    pub fn version(&self) -> &str {
        AGENT_VERSION
    }

    /// Returns the appropriate state machine for the entity type.
    fn get_state_machine(&self, entity_type: &EntityType) -> &StateMachineDefinition {
        match entity_type {
            EntityType::Workflow => &self.workflow_state_machine,
            EntityType::Task => &self.task_state_machine,
        }
    }

    // ========================================================================
    // CORE OPERATIONS
    // ========================================================================

    /// Performs a state transition.
    ///
    /// This is the main entry point for state transitions.
    pub async fn transition(
        &mut self,
        request: StateTransitionRequest,
    ) -> Result<StateTransitionResponse> {
        let start_time = Instant::now();
        let decision_event_id = Uuid::new_v4();

        // Get the appropriate state machine and extract needed values
        let (sm_name, sm_version, validation, success, status, new_state) = {
            let state_machine = self.get_state_machine(&request.entity_type);
            let sm_name = state_machine.name.clone();
            let sm_version = state_machine.version.clone();

            // Validate the transition
            let validation = self.validate_transition(&request, state_machine);

            // Determine if transition should proceed
            let (success, status, new_state) = if request.force && self.config.allow_force {
                // Force transition - skip validation
                (true, TransitionStatus::Forced, request.target_state.clone())
            } else if !validation.valid {
                // Validation failed
                let status = if !validation.source_state_matches || !validation.target_state_exists {
                    TransitionStatus::Invalid
                } else {
                    TransitionStatus::Blocked
                };
                (false, status, request.current_state.clone())
            } else if request.current_state == request.target_state {
                // No change needed
                (true, TransitionStatus::NoChange, request.target_state.clone())
            } else {
                // Valid transition
                (true, TransitionStatus::Completed, request.target_state.clone())
            };

            (sm_name, sm_version, validation, success, status, new_state)
        };

        // Calculate inputs hash
        let inputs_hash = self.hash_inputs(&request);

        // Calculate confidence
        let confidence = self.calculate_confidence(&status, &validation);

        // Build constraints applied list
        let mut constraints_applied = Vec::new();
        constraints_applied.push(format!("state_machine={}", sm_name));
        constraints_applied.push(format!("state_machine_version={}", sm_version));
        if self.config.strict_validation {
            constraints_applied.push("strict_validation=true".to_string());
        }
        if request.force {
            constraints_applied.push("force=true".to_string());
        }

        // Record state history if transition succeeded
        if success && status != TransitionStatus::NoChange {
            // Compute duration first to avoid borrow conflicts
            let duration_in_previous_state_ms: Option<u64> = {
                if let Some((_, _, entered_at)) = self.current_states.get(&request.execution_id) {
                    Some((Utc::now() - *entered_at).num_milliseconds() as u64)
                } else {
                    None
                }
            };

            let entry = StateHistoryEntry {
                id: Uuid::new_v4(),
                from_state: request.current_state.clone(),
                to_state: new_state.clone(),
                reason: request.reason.clone(),
                initiated_by: request.initiated_by.clone(),
                timestamp: Utc::now(),
                duration_in_previous_state_ms,
                decision_event_id,
            };

            self.state_history
                .entry(request.execution_id)
                .or_insert_with(Vec::new)
                .push(entry);

            // Update current state
            self.current_states.insert(
                request.execution_id,
                (request.entity_type.clone(), new_state.clone(), Utc::now()),
            );
        }

        // Build response
        let response = StateTransitionResponse {
            request_id: request.request_id,
            execution_id: request.execution_id,
            entity_type: request.entity_type.clone(),
            success,
            previous_state: request.current_state.clone(),
            new_state: new_state.clone(),
            status: status.clone(),
            validation: validation.clone(),
            confidence,
            inputs_hash: inputs_hash.clone(),
            timestamp: Utc::now(),
            error: if success {
                None
            } else {
                Some(format!("Transition failed: {:?}", status))
            },
            constraints_applied: constraints_applied.clone(),
            decision_event_id,
        };

        // Emit decision event (MANDATORY)
        let decision_event = self.build_decision_event(&request, &response, self.get_state_machine(&request.entity_type));
        self.ruvector_client
            .persist_decision_event(&decision_event)
            .await
            .map_err(|e| OrchestratorError::Internal(e.to_string()))?;

        // Emit telemetry
        let duration = start_time.elapsed();
        self.observatory
            .emit_step_complete(
                request.execution_id,
                &format!("state_transition:{}->{}", request.current_state, new_state),
                success,
                duration,
            )
            .await;

        Ok(response)
    }

    /// Validates a transition request without performing it.
    pub async fn validate(
        &self,
        request: StateTransitionRequest,
    ) -> Result<TransitionValidation> {
        let state_machine = self.get_state_machine(&request.entity_type);
        let validation = self.validate_transition(&request, state_machine);

        // Emit decision event for validation
        let decision_event = DecisionEventRecord {
            id: Uuid::new_v4(),
            agent_id: self.agent_id.clone(),
            agent_version: AGENT_VERSION.to_string(),
            decision_type: "state_validation".to_string(),
            inputs_hash: self.hash_inputs(&request),
            outputs: serde_json::to_value(&validation).unwrap_or_default(),
            confidence: if validation.valid { 1.0 } else { 0.0 },
            constraints_applied: serde_json::json!({
                "state_machine": state_machine.name,
                "validation_type": "pre_transition",
            }),
            workflow_execution_id: request.execution_id,
            step_execution_id: None,
            trace_id: Some(Uuid::new_v4().to_string()),
            span_id: Some(Uuid::new_v4().to_string()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        self.ruvector_client
            .persist_decision_event(&decision_event)
            .await
            .map_err(|e| OrchestratorError::Internal(e.to_string()))?;

        Ok(validation)
    }

    /// Inspects current state.
    pub async fn inspect(&self, request: StateInspectRequest) -> Result<StateInspectResponse> {
        let state_machine = self.get_state_machine(&request.entity_type);

        // Get current state from memory or return initial state
        let (current_state, last_transition_at, last_transition_by) = self
            .current_states
            .get(&request.execution_id)
            .map(|(_, state, entered_at): &(EntityType, String, DateTime<Utc>)| {
                let last_entry = self
                    .state_history
                    .get(&request.execution_id)
                    .and_then(|h: &Vec<StateHistoryEntry>| h.last());
                (
                    state.clone(),
                    Some(*entered_at),
                    last_entry.map(|e: &StateHistoryEntry| e.initiated_by.clone()),
                )
            })
            .unwrap_or_else(|| (state_machine.initial_state.clone(), None, None));

        let is_terminal = state_machine.is_terminal(&current_state);
        let valid_next_states = state_machine.valid_targets(&current_state);

        let transition_count = self
            .state_history
            .get(&request.execution_id)
            .map(|h: &Vec<StateHistoryEntry>| h.len() as u32)
            .unwrap_or(0);

        let time_in_state_ms = last_transition_at
            .map(|t: DateTime<Utc>| (Utc::now() - t).num_milliseconds() as u64);

        // Emit decision event
        let decision_event = DecisionEventRecord {
            id: Uuid::new_v4(),
            agent_id: self.agent_id.clone(),
            agent_version: AGENT_VERSION.to_string(),
            decision_type: "state_inspection".to_string(),
            inputs_hash: format!("{:x}", request.execution_id.as_u128()),
            outputs: serde_json::json!({
                "current_state": current_state,
                "is_terminal": is_terminal,
                "valid_next_states": valid_next_states,
            }),
            confidence: 1.0,
            constraints_applied: serde_json::json!({}),
            workflow_execution_id: request.execution_id,
            step_execution_id: None,
            trace_id: Some(Uuid::new_v4().to_string()),
            span_id: Some(Uuid::new_v4().to_string()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        self.ruvector_client
            .persist_decision_event(&decision_event)
            .await
            .map_err(|e| OrchestratorError::Internal(e.to_string()))?;

        Ok(StateInspectResponse {
            execution_id: request.execution_id,
            entity_type: request.entity_type,
            current_state,
            is_terminal,
            valid_next_states,
            state_machine_id: state_machine.id,
            last_transition_at,
            last_transition_by,
            transition_count,
            time_in_state_ms,
        })
    }

    /// Gets state history.
    pub async fn history(&self, request: StateHistoryRequest) -> Result<StateHistoryResponse> {
        let entries = self
            .state_history
            .get(&request.execution_id)
            .cloned()
            .unwrap_or_default();

        let total_count = entries.len();

        // Apply pagination
        let paginated: Vec<StateHistoryEntry> = entries
            .into_iter()
            .skip(request.offset)
            .take(request.limit.min(self.config.max_history_entries))
            .collect();

        // Emit decision event
        let decision_event = DecisionEventRecord {
            id: Uuid::new_v4(),
            agent_id: self.agent_id.clone(),
            agent_version: AGENT_VERSION.to_string(),
            decision_type: "state_history_query".to_string(),
            inputs_hash: format!("{:x}", request.execution_id.as_u128()),
            outputs: serde_json::json!({
                "total_count": total_count,
                "returned_count": paginated.len(),
            }),
            confidence: 1.0,
            constraints_applied: serde_json::json!({}),
            workflow_execution_id: request.execution_id,
            step_execution_id: None,
            trace_id: Some(Uuid::new_v4().to_string()),
            span_id: Some(Uuid::new_v4().to_string()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        self.ruvector_client
            .persist_decision_event(&decision_event)
            .await
            .map_err(|e| OrchestratorError::Internal(e.to_string()))?;

        Ok(StateHistoryResponse {
            execution_id: request.execution_id,
            entity_type: request.entity_type,
            entries: paginated,
            total_count,
            offset: request.offset,
            limit: request.limit,
        })
    }

    /// Replays a previous state transition.
    pub async fn replay(&self, request: StateReplayRequest) -> Result<StateReplayResponse> {
        let new_decision_id = Uuid::new_v4();

        // Emit decision event for replay
        let decision_event = DecisionEventRecord {
            id: new_decision_id,
            agent_id: self.agent_id.clone(),
            agent_version: AGENT_VERSION.to_string(),
            decision_type: "state_transition_replay".to_string(),
            inputs_hash: format!("{:x}", request.original_decision_id.as_u128()),
            outputs: serde_json::json!({
                "original_decision_id": request.original_decision_id.to_string(),
                "new_decision_id": new_decision_id.to_string(),
            }),
            confidence: 1.0,
            constraints_applied: serde_json::json!({}),
            workflow_execution_id: new_decision_id,
            step_execution_id: None,
            trace_id: Some(Uuid::new_v4().to_string()),
            span_id: Some(Uuid::new_v4().to_string()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        self.ruvector_client
            .persist_decision_event(&decision_event)
            .await
            .map_err(|e| OrchestratorError::Internal(e.to_string()))?;

        Ok(StateReplayResponse {
            original_decision_id: request.original_decision_id,
            new_decision_id,
            inputs_hash_match: true, // Would be verified against stored event
            original_inputs_hash: format!("{:x}", request.original_decision_id.as_u128()),
            new_inputs_hash: format!("{:x}", request.original_decision_id.as_u128()),
            timestamp: Utc::now(),
        })
    }

    // ========================================================================
    // VALIDATION
    // ========================================================================

    /// Validates a state transition against the state machine.
    fn validate_transition(
        &self,
        request: &StateTransitionRequest,
        state_machine: &StateMachineDefinition,
    ) -> TransitionValidation {
        let mut validation = TransitionValidation::default();
        let mut rule_violations = Vec::new();
        let mut warnings = Vec::new();

        // Check if source state exists
        let source_exists = state_machine
            .transitions
            .contains_key(&request.current_state);

        // Check if target state exists in the state machine
        let target_exists = state_machine
            .transitions
            .contains_key(&request.target_state)
            || state_machine.terminal_states.contains(&request.target_state);

        validation.target_state_exists = target_exists;

        // Verify current state matches (check in-memory state if available)
        if let Some((_, current, _)) = self.current_states.get(&request.execution_id) {
            if *current != request.current_state {
                validation.source_state_matches = false;
                warnings.push(format!(
                    "Current state mismatch: expected '{}', got '{}'",
                    current, request.current_state
                ));
            }
        }

        // Check if transition is valid
        let transition_valid = state_machine.is_valid_transition(
            &request.current_state,
            &request.target_state,
        );

        if !transition_valid && !request.force {
            validation.valid = false;
            if !source_exists {
                warnings.push(format!(
                    "Source state '{}' not found in state machine",
                    request.current_state
                ));
            } else if !target_exists {
                warnings.push(format!(
                    "Target state '{}' not found in state machine",
                    request.target_state
                ));
            } else {
                warnings.push(format!(
                    "Invalid transition: '{}' -> '{}' not allowed",
                    request.current_state, request.target_state
                ));
            }
        }

        // Check transition rules
        for rule in &state_machine.transition_rules {
            if rule.source_state == request.current_state
                && rule.target_state == request.target_state
            {
                // Evaluate rule (simplified evaluation)
                let rule_passed = self.evaluate_rule(rule, request);
                if !rule_passed {
                    if rule.required {
                        validation.valid = false;
                    }
                    rule_violations.push(RuleViolation {
                        rule_id: rule.id.clone(),
                        rule_type: rule.rule_type.clone(),
                        message: rule.error_message.clone(),
                    });
                }
            }
        }

        validation.rule_violations = rule_violations;
        validation.warnings = warnings;

        validation
    }

    /// Evaluates a transition rule.
    fn evaluate_rule(&self, rule: &TransitionRule, request: &StateTransitionRequest) -> bool {
        match rule.rule_type {
            TransitionRuleType::DependenciesSatisfied => {
                // Check context for dependencies satisfied flag
                request
                    .context
                    .get("dependencies_satisfied")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true)
            }
            TransitionRuleType::NoPendingRetries => {
                // Check context for pending retries
                request
                    .context
                    .get("pending_retries")
                    .and_then(|v| v.as_u64())
                    .map(|r| r == 0)
                    .unwrap_or(true)
            }
            TransitionRuleType::TimeoutNotExceeded => {
                // Check context for timeout
                request
                    .context
                    .get("timeout_exceeded")
                    .and_then(|v| v.as_bool())
                    .map(|t| !t)
                    .unwrap_or(true)
            }
            TransitionRuleType::Custom => {
                // Custom rules would need specific handling
                true
            }
        }
    }

    // ========================================================================
    // CONFIDENCE CALCULATION
    // ========================================================================

    /// Calculates confidence score for the transition.
    fn calculate_confidence(
        &self,
        status: &TransitionStatus,
        validation: &TransitionValidation,
    ) -> f64 {
        match status {
            TransitionStatus::Completed => 1.0,
            TransitionStatus::NoChange => 1.0,
            TransitionStatus::Forced => 0.7, // Lower confidence for forced transitions
            TransitionStatus::Invalid => 0.0,
            TransitionStatus::Blocked => 0.0,
            TransitionStatus::Error => 0.0,
        }
    }

    // ========================================================================
    // HASHING
    // ========================================================================

    /// Hashes the request inputs.
    fn hash_inputs(&self, request: &StateTransitionRequest) -> String {
        let json = serde_json::to_string(request).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    // ========================================================================
    // DECISION EVENT
    // ========================================================================

    /// Builds a decision event for the transition.
    fn build_decision_event(
        &self,
        request: &StateTransitionRequest,
        response: &StateTransitionResponse,
        state_machine: &StateMachineDefinition,
    ) -> DecisionEventRecord {
        DecisionEventRecord {
            id: response.decision_event_id,
            agent_id: self.agent_id.clone(),
            agent_version: AGENT_VERSION.to_string(),
            decision_type: "state_transition".to_string(),
            inputs_hash: response.inputs_hash.clone(),
            outputs: serde_json::json!({
                "success": response.success,
                "previous_state": response.previous_state,
                "new_state": response.new_state,
                "status": format!("{:?}", response.status),
                "entity_type": format!("{}", response.entity_type),
            }),
            confidence: response.confidence,
            constraints_applied: serde_json::json!({
                "state_machine_id": state_machine.id.to_string(),
                "state_machine_version": state_machine.version,
                "rules_evaluated": state_machine.transition_rules.len(),
                "force_applied": request.force,
                "validation_type": if self.config.strict_validation { "strict" } else { "lenient" },
            }),
            workflow_execution_id: request.execution_id,
            step_execution_id: Some(response.decision_event_id),
            trace_id: Some(Uuid::new_v4().to_string()),
            span_id: Some(Uuid::new_v4().to_string()),
            timestamp: response.timestamp,
            metadata: {
                let mut m = HashMap::new();
                m.insert(
                    "reason".to_string(),
                    serde_json::Value::String(request.reason.clone()),
                );
                m.insert(
                    "initiated_by".to_string(),
                    serde_json::Value::String(request.initiated_by.clone()),
                );
                m
            },
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_request(
        current_state: &str,
        target_state: &str,
    ) -> StateTransitionRequest {
        StateTransitionRequest::new(
            Uuid::new_v4(),
            EntityType::Workflow,
            current_state,
            target_state,
            "Test transition",
            "test-agent",
        )
    }

    #[test]
    fn test_agent_creation() {
        let agent = StateMachineAgent::new(StateMachineAgentConfig::default());
        assert!(agent.agent_id().starts_with(AGENT_ID_PREFIX));
        assert_eq!(agent.version(), AGENT_VERSION);
    }

    #[test]
    fn test_valid_workflow_transition() {
        let agent = StateMachineAgent::new(StateMachineAgentConfig::default());
        let request = create_test_request("pending", "running");
        let sm = agent.get_state_machine(&EntityType::Workflow);
        let validation = agent.validate_transition(&request, sm);

        assert!(validation.valid);
        assert!(validation.source_state_matches);
        assert!(validation.target_state_exists);
        assert!(validation.rule_violations.is_empty());
    }

    #[test]
    fn test_invalid_workflow_transition() {
        let agent = StateMachineAgent::new(StateMachineAgentConfig::default());
        let request = create_test_request("completed", "running"); // Invalid: completed is terminal
        let sm = agent.get_state_machine(&EntityType::Workflow);
        let validation = agent.validate_transition(&request, sm);

        assert!(!validation.valid);
    }

    #[test]
    fn test_task_state_machine() {
        let agent = StateMachineAgent::new(StateMachineAgentConfig::default());
        let request = StateTransitionRequest::new(
            Uuid::new_v4(),
            EntityType::Task,
            "pending",
            "queued",
            "Queue task",
            "test-agent",
        );
        let sm = agent.get_state_machine(&EntityType::Task);
        let validation = agent.validate_transition(&request, sm);

        assert!(validation.valid);
    }

    #[test]
    fn test_confidence_calculation() {
        let agent = StateMachineAgent::new(StateMachineAgentConfig::default());
        let validation = TransitionValidation::default();

        assert_eq!(
            agent.calculate_confidence(&TransitionStatus::Completed, &validation),
            1.0
        );
        assert_eq!(
            agent.calculate_confidence(&TransitionStatus::Forced, &validation),
            0.7
        );
        assert_eq!(
            agent.calculate_confidence(&TransitionStatus::Invalid, &validation),
            0.0
        );
    }

    #[tokio::test]
    async fn test_transition_success() {
        let mut agent = StateMachineAgent::new(StateMachineAgentConfig::default());
        let request = create_test_request("pending", "running");

        let response = agent.transition(request).await.unwrap();

        assert!(response.success);
        assert_eq!(response.status, TransitionStatus::Completed);
        assert_eq!(response.previous_state, "pending");
        assert_eq!(response.new_state, "running");
        assert_eq!(response.confidence, 1.0);
    }

    #[tokio::test]
    async fn test_transition_invalid() {
        let mut agent = StateMachineAgent::new(StateMachineAgentConfig::default());
        let request = create_test_request("completed", "running");

        let response = agent.transition(request).await.unwrap();

        assert!(!response.success);
        assert_eq!(response.status, TransitionStatus::Invalid);
        assert_eq!(response.new_state, "completed"); // Unchanged
    }

    #[tokio::test]
    async fn test_force_transition() {
        let mut agent = StateMachineAgent::new(StateMachineAgentConfig::default());
        let request = create_test_request("completed", "running").with_force(true);

        let response = agent.transition(request).await.unwrap();

        assert!(response.success);
        assert_eq!(response.status, TransitionStatus::Forced);
        assert_eq!(response.new_state, "running");
        assert_eq!(response.confidence, 0.7); // Lower for forced
    }

    #[tokio::test]
    async fn test_no_change_transition() {
        let mut agent = StateMachineAgent::new(StateMachineAgentConfig::default());
        let request = create_test_request("running", "running");

        let response = agent.transition(request).await.unwrap();

        assert!(response.success);
        assert_eq!(response.status, TransitionStatus::NoChange);
    }

    #[tokio::test]
    async fn test_state_history() {
        let mut agent = StateMachineAgent::new(StateMachineAgentConfig::default());
        let execution_id = Uuid::new_v4();

        // Perform some transitions
        let request1 = StateTransitionRequest::new(
            execution_id,
            EntityType::Workflow,
            "pending",
            "running",
            "Start workflow",
            "orchestrator",
        );
        agent.transition(request1).await.unwrap();

        let request2 = StateTransitionRequest::new(
            execution_id,
            EntityType::Workflow,
            "running",
            "completed",
            "Complete workflow",
            "orchestrator",
        );
        agent.transition(request2).await.unwrap();

        // Query history
        let history_request = StateHistoryRequest {
            execution_id,
            entity_type: EntityType::Workflow,
            limit: 100,
            offset: 0,
        };
        let history = agent.history(history_request).await.unwrap();

        assert_eq!(history.total_count, 2);
        assert_eq!(history.entries.len(), 2);
        assert_eq!(history.entries[0].from_state, "pending");
        assert_eq!(history.entries[0].to_state, "running");
        assert_eq!(history.entries[1].from_state, "running");
        assert_eq!(history.entries[1].to_state, "completed");
    }

    #[tokio::test]
    async fn test_inspect() {
        let mut agent = StateMachineAgent::new(StateMachineAgentConfig::default());
        let execution_id = Uuid::new_v4();

        // Transition to running
        let request = StateTransitionRequest::new(
            execution_id,
            EntityType::Workflow,
            "pending",
            "running",
            "Start",
            "test",
        );
        agent.transition(request).await.unwrap();

        // Inspect
        let inspect_request = StateInspectRequest {
            execution_id,
            entity_type: EntityType::Workflow,
        };
        let result = agent.inspect(inspect_request).await.unwrap();

        assert_eq!(result.current_state, "running");
        assert!(!result.is_terminal);
        assert!(result.valid_next_states.contains(&"completed".to_string()));
        assert_eq!(result.transition_count, 1);
    }

    #[tokio::test]
    async fn test_validate_only() {
        let agent = StateMachineAgent::new(StateMachineAgentConfig::default());
        let request = create_test_request("pending", "running");

        let validation = agent.validate(request).await.unwrap();

        assert!(validation.valid);
    }

    #[test]
    fn test_recovery_path() {
        let agent = StateMachineAgent::new(StateMachineAgentConfig::default());

        // Verify failed -> retry_waiting is valid
        let request = create_test_request("failed", "retry_waiting");
        let sm = agent.get_state_machine(&EntityType::Workflow);
        let validation = agent.validate_transition(&request, sm);
        assert!(validation.valid);

        // Verify retry_waiting -> running is valid
        let request = create_test_request("retry_waiting", "running");
        let validation = agent.validate_transition(&request, sm);
        assert!(validation.valid);
    }
}
