// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Agent implementations for LLM-Orchestrator.
//!
//! This module contains agent implementations according to the
//! Agent Infrastructure Constitution.
//!
//! # Agent Types
//!
//! - **WorkflowOrchestratorAgent**: Executes multi-step workflows (WORKFLOW_EXECUTION)
//! - **DependencyResolverAgent**: Resolves task dependencies (DEPENDENCY ANALYSIS)
//! - **ParallelizationAgent**: Identifies parallelization opportunities (EXECUTION PLANNING)
//! - **StateMachineAgent**: Manages workflow state transitions (STATE_MANAGEMENT)
//! - **SwarmCoordinatorAgent**: Coordinates multi-agent swarms (MULTI-AGENT COORDINATION)

pub mod dependency_resolver;
pub mod parallelization_agent;
pub mod state_machine_agent;
pub mod swarm_coordinator_agent;
pub mod workflow_orchestrator;

pub use workflow_orchestrator::{
    WorkflowOrchestratorAgent,
    AgentConfig,
    ExecuteRequest,
    ExecuteResponse,
    ScheduleRequest,
    ScheduleResponse,
    InspectRequest,
    InspectResponse,
    ReplayRequest,
    ReplayResponse,
};

pub use dependency_resolver::{
    DependencyResolverAgent,
    DependencyResolverConfig,
    ResolveRequest as DependencyResolveRequest,
    ResolveResponse as DependencyResolveResponse,
    TaskNode,
    ResolutionConfig,
    ResolutionStatus,
    ParallelGroup,
    GraphSummary,
    DependencyIssue,
    IssueSeverity,
    IssueType,
    CriticalPath,
    ResolutionMetrics,
    InspectRequest as DependencyInspectRequest,
    InspectResponse as DependencyInspectResponse,
    ReplayRequest as DependencyReplayRequest,
    ReplayResponse as DependencyReplayResponse,
    ValidateRequest as DependencyValidateRequest,
    ValidateResponse as DependencyValidateResponse,
    AGENT_VERSION as DEPENDENCY_RESOLVER_VERSION,
    AGENT_ID_PREFIX as DEPENDENCY_RESOLVER_ID_PREFIX,
};

pub use parallelization_agent::{
    ParallelizationAgent,
    ParallelizationAgentConfig,
    InspectResponse as ParallelInspectResponse,
};

pub use state_machine_agent::{
    StateMachineAgent,
    StateMachineAgentConfig,
    EntityType,
    StateTransitionRequest,
    StateTransitionResponse,
    TransitionStatus,
    TransitionValidation,
    RuleViolation,
    StateInspectRequest,
    StateInspectResponse,
    StateHistoryRequest,
    StateHistoryResponse,
    StateHistoryEntry,
    StateReplayRequest,
    StateReplayResponse,
    StateMachineDefinition,
    TransitionRule,
    TransitionRuleType,
    AGENT_VERSION as STATE_MACHINE_AGENT_VERSION,
    AGENT_ID_PREFIX as STATE_MACHINE_AGENT_ID_PREFIX,
};

pub use swarm_coordinator_agent::{
    SwarmCoordinatorAgent,
    SwarmCoordinatorAgentConfig,
    SwarmInspectResponse,
};
