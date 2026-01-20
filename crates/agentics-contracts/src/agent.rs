// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Agent identity and capability schemas.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;
use validator::Validate;

/// Unique identifier for an agent instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl AgentId {
    /// Creates a new agent ID with a given prefix and unique suffix.
    pub fn new(prefix: &str) -> Self {
        Self(format!("{}-{}", prefix, Uuid::new_v4().to_string()[..8].to_string()))
    }

    /// Creates an agent ID from a string.
    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Semantic version for agent implementations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
pub struct AgentVersion {
    /// Major version (breaking changes).
    pub major: u32,
    /// Minor version (new features).
    pub minor: u32,
    /// Patch version (bug fixes).
    pub patch: u32,
    /// Optional pre-release tag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prerelease: Option<String>,
}

impl AgentVersion {
    /// Creates a new version.
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            prerelease: None,
        }
    }

    /// Creates a version from semver string.
    pub fn parse(version: &str) -> Result<Self, crate::error::ContractError> {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() < 3 {
            return Err(crate::error::ContractError::Validation(
                format!("Invalid version format: {}", version)
            ));
        }

        let major = parts[0].parse().map_err(|_| {
            crate::error::ContractError::Validation(format!("Invalid major version: {}", parts[0]))
        })?;
        let minor = parts[1].parse().map_err(|_| {
            crate::error::ContractError::Validation(format!("Invalid minor version: {}", parts[1]))
        })?;

        // Handle patch with optional prerelease
        let (patch_str, prerelease) = if let Some(idx) = parts[2].find('-') {
            (&parts[2][..idx], Some(parts[2][idx + 1..].to_string()))
        } else {
            (parts[2], None)
        };

        let patch = patch_str.parse().map_err(|_| {
            crate::error::ContractError::Validation(format!("Invalid patch version: {}", patch_str))
        })?;

        Ok(Self {
            major,
            minor,
            patch,
            prerelease,
        })
    }
}

impl std::fmt::Display for AgentVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref pre) = self.prerelease {
            write!(f, "{}.{}.{}-{}", self.major, self.minor, self.patch, pre)
        } else {
            write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
        }
    }
}

/// Agent type classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    /// Workflow execution agent.
    WorkflowOrchestrator,
    /// Task coordination agent.
    TaskCoordinator,
    /// State management agent.
    StateManager,
    /// Downstream invocation agent.
    DownstreamInvoker,
    /// Recovery agent.
    RecoveryManager,
    /// Custom agent type.
    Custom(String),
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorkflowOrchestrator => write!(f, "workflow_orchestrator"),
            Self::TaskCoordinator => write!(f, "task_coordinator"),
            Self::StateManager => write!(f, "state_manager"),
            Self::DownstreamInvoker => write!(f, "downstream_invoker"),
            Self::RecoveryManager => write!(f, "recovery_manager"),
            Self::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

/// Agent classification according to the constitution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentClassification {
    /// Execute multi-step workflows.
    WorkflowExecution,
    /// Coordinate task ordering and dependencies.
    TaskCoordination,
    /// Manage workflow state transitions.
    StateManagement,
}

impl std::fmt::Display for AgentClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorkflowExecution => write!(f, "WORKFLOW_EXECUTION"),
            Self::TaskCoordination => write!(f, "TASK_COORDINATION"),
            Self::StateManagement => write!(f, "STATE_MANAGEMENT"),
        }
    }
}

/// Agent capabilities as defined in the constitution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentCapabilities {
    /// Agent MAY execute workflows.
    pub execute_workflows: bool,
    /// Agent MAY invoke downstream agents.
    pub invoke_downstream_agents: bool,
    /// Agent MAY manage task dependencies.
    pub manage_task_dependencies: bool,
    /// Agent MAY trigger retries.
    pub trigger_retries: bool,
    /// Agent MAY coordinate parallel execution.
    pub coordinate_parallel_execution: bool,
    /// Agent MAY transition workflow states.
    pub transition_workflow_states: bool,
}

impl AgentCapabilities {
    /// Creates capabilities for a workflow orchestrator.
    pub fn workflow_orchestrator() -> Self {
        Self {
            execute_workflows: true,
            invoke_downstream_agents: true,
            manage_task_dependencies: true,
            trigger_retries: true,
            coordinate_parallel_execution: true,
            transition_workflow_states: true,
        }
    }

    /// Creates capabilities for a task coordinator.
    pub fn task_coordinator() -> Self {
        Self {
            execute_workflows: false,
            invoke_downstream_agents: false,
            manage_task_dependencies: true,
            trigger_retries: false,
            coordinate_parallel_execution: true,
            transition_workflow_states: false,
        }
    }

    /// Creates capabilities for a state manager.
    pub fn state_manager() -> Self {
        Self {
            execute_workflows: false,
            invoke_downstream_agents: false,
            manage_task_dependencies: false,
            trigger_retries: false,
            coordinate_parallel_execution: false,
            transition_workflow_states: true,
        }
    }

    /// Creates capabilities for a recovery manager (Retry & Recovery Agent).
    ///
    /// The recovery manager can:
    /// - Trigger retries based on failure analysis
    /// - Manage task dependencies for retry eligibility
    /// - Transition workflow/task states (Failed → RetryWaiting)
    ///
    /// The recovery manager CANNOT:
    /// - Execute workflows directly
    /// - Invoke downstream agents (state-driven only)
    /// - Coordinate parallel execution
    pub fn recovery_manager() -> Self {
        Self {
            execute_workflows: false,
            invoke_downstream_agents: false,
            manage_task_dependencies: true,
            trigger_retries: true,
            coordinate_parallel_execution: false,
            transition_workflow_states: true,
        }
    }

    /// Creates capabilities for a parallelization planner (Parallelization Agent).
    ///
    /// The parallelization planner can:
    /// - Analyze task independence and shared constraints
    /// - Manage task dependencies for parallel grouping
    /// - Coordinate parallel execution planning
    ///
    /// The parallelization planner CANNOT:
    /// - Execute workflows directly
    /// - Invoke downstream agents
    /// - Trigger retries
    /// - Transition workflow states
    pub fn parallelization_planner() -> Self {
        Self {
            execute_workflows: false,
            invoke_downstream_agents: false,
            manage_task_dependencies: true,
            trigger_retries: false,
            coordinate_parallel_execution: true,
            transition_workflow_states: false,
        }
    }

    /// Creates capabilities for a swarm coordinator (Swarm Coordinator Agent).
    ///
    /// The swarm coordinator can:
    /// - Invoke downstream agents (spawn workers)
    /// - Manage task dependencies (worker coordination)
    /// - Coordinate parallel execution (fan-out)
    /// - Transition workflow states (swarm lifecycle)
    ///
    /// The swarm coordinator CANNOT:
    /// - Execute workflows directly (delegates to spawned agents)
    /// - Trigger retries directly (handled by workers)
    pub fn swarm_coordinator() -> Self {
        Self {
            execute_workflows: false,
            invoke_downstream_agents: true,
            manage_task_dependencies: true,
            trigger_retries: false,
            coordinate_parallel_execution: true,
            transition_workflow_states: true,
        }
    }
}

/// Agent metadata for registration and discovery.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AgentMetadata {
    /// Unique agent identifier.
    pub id: AgentId,
    /// Agent version.
    pub version: AgentVersion,
    /// Agent type.
    pub agent_type: AgentType,
    /// Agent classification.
    pub classification: AgentClassification,
    /// Human-readable name.
    #[validate(length(min = 1, max = 256))]
    pub name: String,
    /// Description of agent purpose.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Agent capabilities.
    pub capabilities: AgentCapabilities,
    /// Systems that MAY invoke this agent.
    pub allowed_invokers: HashSet<String>,
    /// CLI endpoint name.
    pub cli_endpoint: String,
    /// Explicit non-responsibilities (what this agent MUST NOT do).
    pub non_responsibilities: Vec<String>,
}

impl AgentMetadata {
    /// Creates metadata for the Workflow Orchestrator Agent.
    pub fn workflow_orchestrator() -> Self {
        let mut allowed_invokers = HashSet::new();
        allowed_invokers.insert("llm-edge-agent".to_string());
        allowed_invokers.insert("llm-incident-manager".to_string());
        allowed_invokers.insert("governance-systems".to_string());

        Self {
            id: AgentId::new("workflow-orchestrator"),
            version: AgentVersion::new(0, 1, 0),
            agent_type: AgentType::WorkflowOrchestrator,
            classification: AgentClassification::WorkflowExecution,
            name: "Workflow Orchestrator Agent".to_string(),
            description: Some(
                "Execute and coordinate multi-step workflows across agents in a deterministic and state-aware manner.".to_string()
            ),
            capabilities: AgentCapabilities::workflow_orchestrator(),
            allowed_invokers,
            cli_endpoint: "agent".to_string(),
            non_responsibilities: vec![
                "Intercept runtime traffic directly (that is Edge)".to_string(),
                "Enforce security policies (that is Shield)".to_string(),
                "Emit anomaly detections (that is Sentinel)".to_string(),
                "Perform optimization analysis (that is Auto-Optimizer)".to_string(),
                "Modify schemas dynamically".to_string(),
                "Connect directly to Google SQL".to_string(),
                "Execute SQL statements".to_string(),
            ],
        }
    }

    /// Creates metadata for the Retry & Recovery Agent.
    ///
    /// Classification: EXECUTION RECOVERY (maps to TASK_COORDINATION)
    ///
    /// The Retry & Recovery Agent determines safe retry, backoff, or recovery
    /// strategies following task or workflow failure. It analyzes failure
    /// conditions, applies retry policies, and emits recovery decisions.
    pub fn recovery_manager() -> Self {
        let mut allowed_invokers = HashSet::new();
        allowed_invokers.insert("workflow-orchestrator".to_string());
        allowed_invokers.insert("llm-incident-manager".to_string());
        allowed_invokers.insert("task-scheduler".to_string());
        allowed_invokers.insert("governance-systems".to_string());

        Self {
            id: AgentId::new("retry-recovery"),
            version: AgentVersion::new(0, 1, 0),
            agent_type: AgentType::RecoveryManager,
            classification: AgentClassification::TaskCoordination,
            name: "Retry & Recovery Agent".to_string(),
            description: Some(
                "Determine safe retry, backoff, or recovery strategies following task or workflow failure. \
                Analyzes failure conditions and retry eligibility, applies retry policies and backoff strategies, \
                recommends recovery paths or workflow termination, and emits retry or recovery decisions.".to_string()
            ),
            capabilities: AgentCapabilities::recovery_manager(),
            allowed_invokers,
            cli_endpoint: "recovery".to_string(),
            non_responsibilities: vec![
                "Execute workflow steps directly (delegates to Workflow Orchestrator)".to_string(),
                "Modify task inputs or outputs".to_string(),
                "Change provider configurations".to_string(),
                "Intercept runtime traffic directly (that is Edge)".to_string(),
                "Enforce security policies (that is Shield)".to_string(),
                "Emit anomaly detections (that is Sentinel)".to_string(),
                "Perform optimization analysis (that is Auto-Optimizer)".to_string(),
                "Connect directly to Google SQL".to_string(),
                "Execute SQL statements".to_string(),
                "Modify workflow schemas dynamically".to_string(),
                "Invoke other agents directly (state-driven only)".to_string(),
            ],
        }
    }

    /// Creates metadata for the Parallelization Agent.
    ///
    /// Classification: EXECUTION PLANNING (maps to TASK_COORDINATION)
    ///
    /// The Parallelization Agent identifies tasks that can execute concurrently
    /// to improve workflow throughput. It analyzes task independence and shared
    /// constraints, determines safe parallel execution groups, emits parallel
    /// execution plans, and optimizes execution efficiency without altering logic.
    pub fn parallelization_planner() -> Self {
        let mut allowed_invokers = HashSet::new();
        allowed_invokers.insert("workflow-orchestrator".to_string());
        allowed_invokers.insert("dependency-resolver".to_string());
        allowed_invokers.insert("task-scheduler".to_string());
        allowed_invokers.insert("governance-systems".to_string());

        Self {
            id: AgentId::new("parallelization"),
            version: AgentVersion::new(0, 1, 0),
            agent_type: AgentType::TaskCoordinator,
            classification: AgentClassification::TaskCoordination,
            name: "Parallelization Agent".to_string(),
            description: Some(
                "Identify tasks that can execute concurrently to improve workflow throughput. \
                Analyzes task independence and shared constraints, determines safe parallel \
                execution groups, emits parallel execution plans, and optimizes execution \
                efficiency without altering logic.".to_string()
            ),
            capabilities: AgentCapabilities::parallelization_planner(),
            allowed_invokers,
            cli_endpoint: "parallel".to_string(),
            non_responsibilities: vec![
                "Execute workflow steps directly (delegates to Workflow Orchestrator)".to_string(),
                "Modify task inputs or outputs".to_string(),
                "Change provider configurations".to_string(),
                "Intercept runtime traffic directly (that is Edge)".to_string(),
                "Enforce security policies (that is Shield)".to_string(),
                "Emit anomaly detections (that is Sentinel)".to_string(),
                "Perform optimization analysis (that is Auto-Optimizer)".to_string(),
                "Connect directly to Google SQL".to_string(),
                "Execute SQL statements".to_string(),
                "Modify workflow schemas dynamically".to_string(),
                "Invoke other agents directly (state-driven only)".to_string(),
                "Trigger retries or recovery operations".to_string(),
            ],
        }
    }

    /// Creates metadata for the Swarm Coordinator Agent.
    ///
    /// Classification: MULTI-AGENT COORDINATION (maps to TASK_COORDINATION)
    ///
    /// The Swarm Coordinator Agent coordinates fan-out / fan-in execution across
    /// multiple agents working toward a shared objective. It spawns and manages
    /// parallel agent executions, aggregates results from multiple agents,
    /// resolves conflicts or convergence conditions, and emits unified swarm outcomes.
    pub fn swarm_coordinator() -> Self {
        let mut allowed_invokers = HashSet::new();
        allowed_invokers.insert("workflow-orchestrator".to_string());
        allowed_invokers.insert("llm-edge-agent".to_string());
        allowed_invokers.insert("llm-incident-manager".to_string());
        allowed_invokers.insert("governance-systems".to_string());

        Self {
            id: AgentId::new("swarm-coordinator"),
            version: AgentVersion::new(0, 1, 0),
            agent_type: AgentType::TaskCoordinator,
            classification: AgentClassification::TaskCoordination,
            name: "Swarm Coordinator Agent".to_string(),
            description: Some(
                "Coordinate fan-out / fan-in execution across multiple agents working \
                toward a shared objective. Spawns and manages parallel agent executions, \
                aggregates results from multiple agents, resolves conflicts or convergence \
                conditions, and emits unified swarm outcomes.".to_string()
            ),
            capabilities: AgentCapabilities::swarm_coordinator(),
            allowed_invokers,
            cli_endpoint: "swarm".to_string(),
            non_responsibilities: vec![
                "Execute workflow steps directly (delegates to spawned agents)".to_string(),
                "Modify task inputs or outputs produced by workers".to_string(),
                "Change provider configurations".to_string(),
                "Intercept runtime traffic directly (that is Edge)".to_string(),
                "Enforce security policies (that is Shield)".to_string(),
                "Emit anomaly detections (that is Sentinel)".to_string(),
                "Perform optimization analysis (that is Auto-Optimizer)".to_string(),
                "Connect directly to Google SQL".to_string(),
                "Execute SQL statements".to_string(),
                "Modify workflow schemas dynamically".to_string(),
                "Generate or evaluate model quality (that is Analytics/Observatory)".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_id_creation() {
        let id = AgentId::new("test");
        assert!(id.0.starts_with("test-"));
        assert_eq!(id.0.len(), 13); // "test-" + 8 chars
    }

    #[test]
    fn test_agent_version_parsing() {
        let version = AgentVersion::parse("1.2.3").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        assert!(version.prerelease.is_none());

        let version = AgentVersion::parse("1.0.0-alpha").unwrap();
        assert_eq!(version.prerelease, Some("alpha".to_string()));
    }

    #[test]
    fn test_workflow_orchestrator_metadata() {
        let metadata = AgentMetadata::workflow_orchestrator();
        assert_eq!(metadata.classification, AgentClassification::WorkflowExecution);
        assert!(metadata.capabilities.execute_workflows);
        assert!(!metadata.non_responsibilities.is_empty());
    }

    #[test]
    fn test_recovery_manager_metadata() {
        let metadata = AgentMetadata::recovery_manager();
        assert_eq!(metadata.agent_type, AgentType::RecoveryManager);
        assert_eq!(metadata.classification, AgentClassification::TaskCoordination);
        assert_eq!(metadata.name, "Retry & Recovery Agent");
        assert_eq!(metadata.cli_endpoint, "recovery");

        // Verify capabilities
        assert!(!metadata.capabilities.execute_workflows);
        assert!(!metadata.capabilities.invoke_downstream_agents);
        assert!(metadata.capabilities.manage_task_dependencies);
        assert!(metadata.capabilities.trigger_retries);
        assert!(!metadata.capabilities.coordinate_parallel_execution);
        assert!(metadata.capabilities.transition_workflow_states);

        // Verify allowed invokers
        assert!(metadata.allowed_invokers.contains("workflow-orchestrator"));
        assert!(metadata.allowed_invokers.contains("llm-incident-manager"));
        assert!(metadata.allowed_invokers.contains("task-scheduler"));

        // Verify non-responsibilities are defined
        assert!(!metadata.non_responsibilities.is_empty());
        assert!(metadata.non_responsibilities.iter().any(|r| r.contains("Execute workflow steps directly")));
        assert!(metadata.non_responsibilities.iter().any(|r| r.contains("Connect directly to Google SQL")));
    }

    #[test]
    fn test_parallelization_planner_metadata() {
        let metadata = AgentMetadata::parallelization_planner();
        assert_eq!(metadata.agent_type, AgentType::TaskCoordinator);
        assert_eq!(metadata.classification, AgentClassification::TaskCoordination);
        assert_eq!(metadata.name, "Parallelization Agent");
        assert_eq!(metadata.cli_endpoint, "parallel");

        // Verify capabilities
        assert!(!metadata.capabilities.execute_workflows);
        assert!(!metadata.capabilities.invoke_downstream_agents);
        assert!(metadata.capabilities.manage_task_dependencies);
        assert!(!metadata.capabilities.trigger_retries);
        assert!(metadata.capabilities.coordinate_parallel_execution);
        assert!(!metadata.capabilities.transition_workflow_states);

        // Verify allowed invokers
        assert!(metadata.allowed_invokers.contains("workflow-orchestrator"));
        assert!(metadata.allowed_invokers.contains("dependency-resolver"));

        // Verify non-responsibilities are defined
        assert!(!metadata.non_responsibilities.is_empty());
        assert!(metadata.non_responsibilities.iter().any(|r| r.contains("Execute workflow steps")));
        assert!(metadata.non_responsibilities.iter().any(|r| r.contains("Connect directly to Google SQL")));
    }

    #[test]
    fn test_parallelization_planner_capabilities() {
        let capabilities = AgentCapabilities::parallelization_planner();
        assert!(!capabilities.execute_workflows);
        assert!(!capabilities.invoke_downstream_agents);
        assert!(capabilities.manage_task_dependencies);
        assert!(!capabilities.trigger_retries);
        assert!(capabilities.coordinate_parallel_execution);
        assert!(!capabilities.transition_workflow_states);
    }

    #[test]
    fn test_recovery_manager_capabilities() {
        let capabilities = AgentCapabilities::recovery_manager();
        assert!(!capabilities.execute_workflows);
        assert!(!capabilities.invoke_downstream_agents);
        assert!(capabilities.manage_task_dependencies);
        assert!(capabilities.trigger_retries);
        assert!(!capabilities.coordinate_parallel_execution);
        assert!(capabilities.transition_workflow_states);
    }

    #[test]
    fn test_swarm_coordinator_metadata() {
        let metadata = AgentMetadata::swarm_coordinator();
        assert_eq!(metadata.agent_type, AgentType::TaskCoordinator);
        assert_eq!(metadata.classification, AgentClassification::TaskCoordination);
        assert_eq!(metadata.name, "Swarm Coordinator Agent");
        assert_eq!(metadata.cli_endpoint, "swarm");

        // Verify capabilities
        assert!(!metadata.capabilities.execute_workflows);
        assert!(metadata.capabilities.invoke_downstream_agents);
        assert!(metadata.capabilities.manage_task_dependencies);
        assert!(!metadata.capabilities.trigger_retries);
        assert!(metadata.capabilities.coordinate_parallel_execution);
        assert!(metadata.capabilities.transition_workflow_states);

        // Verify allowed invokers
        assert!(metadata.allowed_invokers.contains("workflow-orchestrator"));
        assert!(metadata.allowed_invokers.contains("llm-edge-agent"));
        assert!(metadata.allowed_invokers.contains("governance-systems"));

        // Verify non-responsibilities are defined
        assert!(!metadata.non_responsibilities.is_empty());
        assert!(metadata.non_responsibilities.iter().any(|r| r.contains("Execute workflow steps directly")));
        assert!(metadata.non_responsibilities.iter().any(|r| r.contains("Connect directly to Google SQL")));
    }

    #[test]
    fn test_swarm_coordinator_capabilities() {
        let capabilities = AgentCapabilities::swarm_coordinator();
        assert!(!capabilities.execute_workflows);
        assert!(capabilities.invoke_downstream_agents);
        assert!(capabilities.manage_task_dependencies);
        assert!(!capabilities.trigger_retries);
        assert!(capabilities.coordinate_parallel_execution);
        assert!(capabilities.transition_workflow_states);
    }
}
