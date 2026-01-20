// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Agentics Contracts - Agent schemas and types for LLM-Orchestrator.
//!
//! This crate defines the NON-NEGOTIABLE contract schemas for all agents
//! operating within the LLM-Orchestrator platform according to the
//! Agent Infrastructure Constitution.
//!
//! # Core Principles
//!
//! - All agents MUST import schemas exclusively from this crate
//! - All inputs and outputs MUST be validated against these contracts
//! - Every agent invocation MUST emit exactly ONE DecisionEvent
//! - All persistence occurs via ruvector-service client calls only
//!
//! # Agent Classifications
//!
//! Agents are classified into three categories:
//! - **WORKFLOW_EXECUTION**: Execute multi-step workflows
//! - **TASK_COORDINATION**: Coordinate task ordering and dependencies
//! - **STATE_MANAGEMENT**: Manage workflow state transitions

pub mod agent;
pub mod decision;
pub mod dependency;
pub mod parallelization;
pub mod state_machine;
pub mod swarm_coordinator;
pub mod workflow;
pub mod error;

// Re-export core types
pub use agent::{
    AgentId, AgentVersion, AgentType, AgentMetadata,
    AgentClassification, AgentCapabilities,
};
pub use decision::{
    DecisionEvent, DecisionType, DecisionOutcome,
    ConstraintsApplied, ExecutionRef,
};
pub use workflow::{
    WorkflowEnvelope, TaskEnvelope, WorkflowState,
    TaskState, TransitionRequest, TransitionResult,
};
pub use dependency::{
    DependencyResolutionRequest, DependencyResolutionResult,
    TaskDependencyNode, DependencyResolutionConfig,
    ParallelExecutionGroup, DependencyGraphSummary,
    DependencyIssue, CriticalPath, ResolutionStatus,
    DependencyResolutionDecisionEvent, DependencyExecutionRef,
    IssueSeverity, DependencyConstraint, DependencyStatus,
    DEPENDENCY_RESOLVER_AGENT_ID, DEPENDENCY_RESOLVER_AGENT_VERSION,
};
pub use parallelization::{
    ParallelizationRequest, ParallelizationResult, ParallelizationConfig,
    ParallelizationStatus, ParallelExecutionPlan, ExecutionPhase,
    ParallelTask, ParallelizationEligibility, ExecutionStrategy,
    ParallelizationSummary, OptimizationMetrics, ParallelizationIssue,
    ParallelizationIssueSeverity, ParallelizationIssueType,
    ParallelizationDecisionEvent, ParallelizationExecutionRef,
    ResourceConstraints, TaskResources, PhaseResources,
    PARALLELIZATION_AGENT_ID, PARALLELIZATION_AGENT_VERSION,
};
pub use state_machine::{
    StateMachineDefinition, TransitionRule, TransitionRuleType,
    StateTransitionRequest, StateTransitionResponse, EntityType,
    TransitionStatus, TransitionValidation, RuleViolation,
    StateInspectRequest, StateInspectResponse,
    StateHistoryRequest, StateHistoryResponse, StateHistoryEntry,
    StateReplayRequest, StateReplayResponse,
    StateTransitionDecisionEvent, TransitionDetails,
    StateTransitionConstraints, StateTransitionExecutionRef,
    STATE_MACHINE_AGENT_ID, STATE_MACHINE_AGENT_VERSION,
};
pub use swarm_coordinator::{
    SwarmCoordinationRequest, SwarmCoordinationResult, SwarmCoordinatorConfig,
    SwarmObjective, ObjectiveType, SuccessCriterion, ValidationRule,
    WorkerSpec, WorkerAgentType, WorkerTask, WorkerConfig, WorkerResourceLimits,
    ConsensusStrategy, AggregationStrategy, AggregationMode, FieldReduction, ReductionOperation,
    SwarmCoordinationStatus, AggregatedOutput, ConsensusData,
    WorkerResult, WorkerStatus, WorkerError, WorkerMetrics,
    CoordinationSummary, CoordinationIssue, CoordinationIssueSeverity, CoordinationIssueType,
    SwarmCoordinationDecisionEvent, SwarmCoordinationDecisionOutputs,
    SwarmCoordinationConstraintsApplied, SwarmCoordinationExecutionRef,
    SWARM_COORDINATOR_AGENT_ID, SWARM_COORDINATOR_AGENT_VERSION,
};
pub use error::{ContractError, ContractResult};

/// Library version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name.
pub const NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(VERSION.contains('.'));
        assert_eq!(NAME, "agentics-contracts");
    }
}
