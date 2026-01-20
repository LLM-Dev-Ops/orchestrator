// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Parallelization Agent schemas for concurrent task execution planning.
//!
//! This module defines the contract schemas for the Parallelization Agent
//! within the Agent Infrastructure Constitution. The Parallelization Agent
//! is classified under EXECUTION PLANNING and is responsible for identifying
//! tasks that can execute concurrently to improve workflow throughput.
//!
//! # Agent Contract
//!
//! **Agent Name**: Parallelization Agent
//!
//! **Classification**: EXECUTION_PLANNING (maps to TASK_COORDINATION)
//!
//! **Purpose**: Identify tasks that can execute concurrently to improve
//! workflow throughput.
//!
//! **Scope**:
//! - Analyze task independence and shared constraints
//! - Determine safe parallel execution groups
//! - Emit parallel execution plans
//! - Optimize execution efficiency without altering logic
//!
//! **decision_type**: `"parallel_execution_plan"`
//!
//! # Non-Responsibilities (MUST NEVER)
//!
//! - Intercept runtime traffic directly (that is Edge)
//! - Enforce security policies (that is Shield)
//! - Emit anomaly detections (that is Sentinel)
//! - Perform optimization analysis (that is Auto-Optimizer)
//! - Modify task logic or inputs
//! - Connect directly to Google SQL
//! - Execute SQL

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::agent::{AgentId, AgentVersion};
use crate::dependency::{
    DependencyConstraint, DependencyGraphSummary, ParallelExecutionGroup, TaskDependencyNode,
};

/// Parallelization Agent ID constant.
pub const PARALLELIZATION_AGENT_ID: &str = "parallelization-agent";

/// Parallelization Agent Version.
pub const PARALLELIZATION_AGENT_VERSION: &str = "0.1.0";

// =============================================================================
// Input Schema: ParallelizationRequest
// =============================================================================

/// Request for parallelization analysis.
///
/// Input contract for the Parallelization Agent's analyze operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelizationRequest {
    /// Unique request ID.
    pub request_id: Uuid,

    /// Workflow ID to analyze.
    pub workflow_id: Uuid,

    /// List of tasks with their dependency information.
    pub tasks: Vec<TaskDependencyNode>,

    /// Analysis configuration.
    #[serde(default)]
    pub config: ParallelizationConfig,

    /// Resource constraints for parallelization.
    #[serde(default)]
    pub resource_constraints: ResourceConstraints,

    /// Request timestamp.
    pub timestamp: DateTime<Utc>,
}

/// Configuration for parallelization analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelizationConfig {
    /// Maximum concurrent tasks allowed.
    #[serde(default = "default_max_concurrency")]
    pub max_concurrency: u32,

    /// Whether to respect soft dependencies or only hard dependencies.
    #[serde(default = "default_true")]
    pub respect_soft_dependencies: bool,

    /// Whether to analyze data dependencies for parallelization opportunities.
    #[serde(default = "default_true")]
    pub analyze_data_dependencies: bool,

    /// Whether to consider resource constraints in planning.
    #[serde(default = "default_true")]
    pub consider_resource_constraints: bool,

    /// Whether to optimize for minimum makespan (total execution time).
    #[serde(default = "default_true")]
    pub optimize_makespan: bool,

    /// Whether to balance load across parallel groups.
    #[serde(default)]
    pub balance_load: bool,

    /// Analysis timeout in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

fn default_max_concurrency() -> u32 {
    8
}

fn default_true() -> bool {
    true
}

impl Default for ParallelizationConfig {
    fn default() -> Self {
        Self {
            max_concurrency: 8,
            respect_soft_dependencies: true,
            analyze_data_dependencies: true,
            consider_resource_constraints: true,
            optimize_makespan: true,
            balance_load: false,
            timeout_ms: None,
        }
    }
}

/// Resource constraints for parallelization planning.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceConstraints {
    /// Maximum total memory across all concurrent tasks (bytes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_total_memory_bytes: Option<u64>,

    /// Maximum total CPU cores across all concurrent tasks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_total_cpu: Option<f64>,

    /// Maximum total GPU units across all concurrent tasks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_total_gpu: Option<f64>,

    /// Maximum total token budget for LLM tasks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_total_tokens: Option<u64>,

    /// Per-task resource limits.
    #[serde(default)]
    pub per_task_limits: HashMap<String, TaskResourceLimit>,
}

/// Resource limit for a specific task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResourceLimit {
    /// Memory limit in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_bytes: Option<u64>,

    /// CPU limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<f64>,

    /// GPU limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu: Option<f64>,

    /// Token limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<u64>,
}

// =============================================================================
// Output Schema: ParallelizationResult
// =============================================================================

/// Result of parallelization analysis.
///
/// Output contract for the Parallelization Agent's analyze operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelizationResult {
    /// Request ID this result corresponds to.
    pub request_id: Uuid,

    /// Workflow ID.
    pub workflow_id: Uuid,

    /// Analysis status.
    pub status: ParallelizationStatus,

    /// Parallel execution plan.
    pub execution_plan: ParallelExecutionPlan,

    /// Summary of the analysis.
    pub summary: ParallelizationSummary,

    /// Optimization metrics comparing sequential vs parallel execution.
    pub optimization_metrics: OptimizationMetrics,

    /// Issues detected during analysis.
    pub issues: Vec<ParallelizationIssue>,

    /// Analysis timestamp.
    pub analyzed_at: DateTime<Utc>,
}

/// Status of parallelization analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParallelizationStatus {
    /// Analysis completed successfully.
    Success,

    /// Analysis completed but no parallelization opportunities found.
    NoOpportunities,

    /// Analysis completed with warnings (partial parallelization possible).
    SuccessWithWarnings,

    /// Analysis failed due to errors.
    Failed,

    /// Analysis timed out.
    Timeout,
}

/// Parallel execution plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelExecutionPlan {
    /// Plan ID.
    pub plan_id: Uuid,

    /// Execution phases (groups of tasks that can run in parallel).
    pub phases: Vec<ExecutionPhase>,

    /// Total number of phases.
    pub total_phases: u32,

    /// Maximum parallelism achieved in any phase.
    pub max_parallelism: u32,

    /// Execution strategy recommendation.
    pub strategy: ExecutionStrategy,

    /// Critical path tasks (cannot be parallelized).
    pub critical_path_tasks: Vec<String>,

    /// Task-to-phase mapping for quick lookup.
    pub task_phase_map: HashMap<String, u32>,
}

/// An execution phase containing tasks that can run in parallel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPhase {
    /// Phase index (0-based, in execution order).
    pub phase_index: u32,

    /// Tasks in this phase that can execute concurrently.
    pub parallel_tasks: Vec<ParallelTask>,

    /// Dependencies that must be satisfied before this phase starts.
    pub depends_on_phases: Vec<u32>,

    /// Estimated duration for this phase (max of all parallel tasks).
    pub estimated_duration_ms: u64,

    /// Total resource requirements for this phase.
    pub total_resources: PhaseResources,

    /// Whether this phase contains critical path tasks.
    pub is_critical: bool,
}

/// A task within a parallel execution phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelTask {
    /// Task ID.
    pub task_id: String,

    /// Task name.
    pub name: String,

    /// Parallelization eligibility status.
    pub eligibility: ParallelizationEligibility,

    /// Tasks that this can safely run alongside.
    pub safe_concurrent_with: Vec<String>,

    /// Tasks that this CANNOT run alongside (shared resources, data conflicts).
    pub conflicts_with: Vec<String>,

    /// Estimated duration in milliseconds.
    pub estimated_duration_ms: u64,

    /// Resource requirements.
    pub resources: TaskResources,
}

/// Eligibility status for parallelization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParallelizationEligibility {
    /// Fully eligible for parallel execution.
    Eligible,

    /// Eligible with constraints.
    EligibleWithConstraints,

    /// Not eligible (must run sequentially).
    NotEligible,

    /// Status unknown (insufficient information).
    Unknown,
}

/// Resource requirements for a task.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskResources {
    /// Estimated memory usage in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_bytes: Option<u64>,

    /// Estimated CPU usage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<f64>,

    /// Estimated GPU usage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu: Option<f64>,

    /// Estimated token usage for LLM tasks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<u64>,
}

/// Total resources for a phase.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhaseResources {
    /// Total memory across all parallel tasks.
    pub total_memory_bytes: u64,

    /// Total CPU across all parallel tasks.
    pub total_cpu: f64,

    /// Total GPU across all parallel tasks.
    pub total_gpu: f64,

    /// Total tokens across all parallel tasks.
    pub total_tokens: u64,

    /// Whether resource constraints are satisfied.
    pub constraints_satisfied: bool,
}

/// Recommended execution strategy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStrategy {
    /// Execute all tasks sequentially (no parallelization benefit).
    Sequential,

    /// Execute with static parallelism (fixed concurrent groups).
    StaticParallel,

    /// Execute with dynamic parallelism (adaptive based on runtime).
    DynamicParallel,

    /// Execute with pipeline parallelism (streaming between stages).
    Pipeline,

    /// Hybrid approach combining multiple strategies.
    Hybrid,
}

/// Summary of parallelization analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelizationSummary {
    /// Total number of tasks analyzed.
    pub total_tasks: u32,

    /// Number of tasks eligible for parallelization.
    pub parallelizable_tasks: u32,

    /// Number of tasks that must be sequential.
    pub sequential_tasks: u32,

    /// Number of parallel execution phases.
    pub execution_phases: u32,

    /// Maximum achievable parallelism.
    pub max_parallelism: u32,

    /// Average parallelism across phases.
    pub avg_parallelism: f64,

    /// Dependency graph summary.
    pub graph_summary: DependencyGraphSummary,
}

/// Metrics comparing sequential vs parallel execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationMetrics {
    /// Estimated sequential execution time (ms).
    pub sequential_duration_ms: u64,

    /// Estimated parallel execution time (ms).
    pub parallel_duration_ms: u64,

    /// Speedup factor (sequential / parallel).
    pub speedup_factor: f64,

    /// Efficiency score (0.0 to 1.0).
    pub efficiency_score: f64,

    /// Resource utilization improvement.
    pub resource_utilization: f64,

    /// Critical path length (ms).
    pub critical_path_duration_ms: u64,

    /// Parallelization overhead estimate (ms).
    pub overhead_estimate_ms: u64,
}

/// An issue detected during parallelization analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelizationIssue {
    /// Issue severity.
    pub severity: ParallelizationIssueSeverity,

    /// Issue type.
    pub issue_type: ParallelizationIssueType,

    /// Affected task IDs.
    pub affected_tasks: Vec<String>,

    /// Human-readable description.
    pub description: String,

    /// Impact on parallelization.
    pub impact: String,

    /// Suggested resolution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// Severity of a parallelization issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParallelizationIssueSeverity {
    /// Informational only.
    Info,
    /// Warning - parallelization possible but suboptimal.
    Warning,
    /// Error - prevents parallelization of affected tasks.
    Error,
}

/// Type of parallelization issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParallelizationIssueType {
    /// Resource conflict between tasks.
    ResourceConflict,
    /// Data dependency prevents parallelization.
    DataDependency,
    /// Shared state access conflict.
    SharedStateConflict,
    /// Ordering constraint prevents parallelization.
    OrderingConstraint,
    /// Resource limit exceeded.
    ResourceLimitExceeded,
    /// Maximum concurrency limit reached.
    ConcurrencyLimitReached,
    /// Unknown task dependency.
    UnknownDependency,
    /// Circular dependency detected.
    CircularDependency,
}

// =============================================================================
// DecisionEvent Schema for Parallelization Agent
// =============================================================================

/// DecisionEvent specific to the Parallelization Agent.
///
/// Emitted for every invocation of the Parallelization Agent
/// according to the Agent Infrastructure Constitution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelizationDecisionEvent {
    /// Unique event ID.
    pub id: Uuid,

    /// Agent identifier.
    pub agent_id: AgentId,

    /// Agent version.
    pub agent_version: AgentVersion,

    /// Decision type: "parallel_execution_plan".
    pub decision_type: String,

    /// SHA-256 hash of the input request.
    pub inputs_hash: String,

    /// Parallelization outputs.
    pub outputs: ParallelizationDecisionOutputs,

    /// Confidence in the analysis (0.0 to 1.0).
    ///
    /// Calculated based on:
    /// - Completeness of dependency information
    /// - Resource estimation accuracy
    /// - Historical accuracy of similar analyses
    pub confidence: f64,

    /// Constraints applied during analysis.
    pub constraints_applied: ParallelizationConstraintsApplied,

    /// Execution reference for tracing.
    pub execution_ref: ParallelizationExecutionRef,

    /// Event timestamp.
    pub timestamp: DateTime<Utc>,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Outputs for the parallelization decision event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelizationDecisionOutputs {
    /// Analysis status.
    pub status: ParallelizationStatus,

    /// Number of tasks analyzed.
    pub tasks_analyzed: u32,

    /// Number of parallelizable tasks identified.
    pub parallelizable_tasks: u32,

    /// Number of execution phases.
    pub execution_phases: u32,

    /// Maximum parallelism achieved.
    pub max_parallelism: u32,

    /// Speedup factor.
    pub speedup_factor: f64,

    /// Number of issues detected.
    pub issues_count: u32,

    /// Recommended strategy.
    pub strategy: ExecutionStrategy,
}

/// Constraints applied during parallelization analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelizationConstraintsApplied {
    /// Maximum concurrency constraint.
    pub max_concurrency: u32,

    /// Resource constraints applied.
    pub resource_constrained: bool,

    /// Soft dependencies respected.
    pub soft_dependencies_respected: bool,

    /// Data dependencies analyzed.
    pub data_dependencies_analyzed: bool,

    /// Makespan optimization enabled.
    pub makespan_optimized: bool,

    /// Analysis timeout (if applied).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// Execution reference for parallelization analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelizationExecutionRef {
    /// Unique execution ID.
    pub execution_id: Uuid,

    /// Workflow ID.
    pub workflow_id: Uuid,

    /// Request ID.
    pub request_id: Uuid,

    /// Trace ID for distributed tracing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,

    /// Span ID for distributed tracing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
}

impl ParallelizationExecutionRef {
    /// Creates a new execution reference.
    pub fn new(workflow_id: Uuid, request_id: Uuid) -> Self {
        Self {
            execution_id: Uuid::new_v4(),
            workflow_id,
            request_id,
            trace_id: Some(Uuid::new_v4().to_string()),
            span_id: Some(Uuid::new_v4().to_string()),
        }
    }
}

// =============================================================================
// Helper Implementations
// =============================================================================

impl ParallelizationRequest {
    /// Creates a new parallelization request.
    pub fn new(workflow_id: Uuid, tasks: Vec<TaskDependencyNode>) -> Self {
        Self {
            request_id: Uuid::new_v4(),
            workflow_id,
            tasks,
            config: ParallelizationConfig::default(),
            resource_constraints: ResourceConstraints::default(),
            timestamp: Utc::now(),
        }
    }

    /// Sets the configuration.
    pub fn with_config(mut self, config: ParallelizationConfig) -> Self {
        self.config = config;
        self
    }

    /// Sets the resource constraints.
    pub fn with_resource_constraints(mut self, constraints: ResourceConstraints) -> Self {
        self.resource_constraints = constraints;
        self
    }
}

impl ParallelExecutionPlan {
    /// Creates an empty plan (for sequential execution).
    pub fn sequential(tasks: &[TaskDependencyNode]) -> Self {
        let phases: Vec<ExecutionPhase> = tasks
            .iter()
            .enumerate()
            .map(|(i, task)| ExecutionPhase {
                phase_index: i as u32,
                parallel_tasks: vec![ParallelTask {
                    task_id: task.task_id.clone(),
                    name: task.name.clone(),
                    eligibility: ParallelizationEligibility::NotEligible,
                    safe_concurrent_with: Vec::new(),
                    conflicts_with: Vec::new(),
                    estimated_duration_ms: task.estimated_duration_ms.unwrap_or(1000),
                    resources: TaskResources::default(),
                }],
                depends_on_phases: if i > 0 { vec![(i - 1) as u32] } else { Vec::new() },
                estimated_duration_ms: task.estimated_duration_ms.unwrap_or(1000),
                total_resources: PhaseResources::default(),
                is_critical: true,
            })
            .collect();

        let task_phase_map: HashMap<String, u32> = tasks
            .iter()
            .enumerate()
            .map(|(i, t)| (t.task_id.clone(), i as u32))
            .collect();

        Self {
            plan_id: Uuid::new_v4(),
            total_phases: phases.len() as u32,
            max_parallelism: 1,
            strategy: ExecutionStrategy::Sequential,
            critical_path_tasks: tasks.iter().map(|t| t.task_id.clone()).collect(),
            phases,
            task_phase_map,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tasks() -> Vec<TaskDependencyNode> {
        vec![
            TaskDependencyNode {
                task_id: "task-1".to_string(),
                name: "First Task".to_string(),
                task_type: Some("llm".to_string()),
                dependencies: Vec::new(),
                estimated_duration_ms: Some(1000),
                priority: 0,
                metadata: HashMap::new(),
            },
            TaskDependencyNode {
                task_id: "task-2".to_string(),
                name: "Second Task".to_string(),
                task_type: Some("transform".to_string()),
                dependencies: vec![DependencyConstraint {
                    task_id: "task-1".to_string(),
                    required_status: crate::dependency::DependencyStatus::Success,
                    timeout_ms: None,
                }],
                estimated_duration_ms: Some(500),
                priority: 0,
                metadata: HashMap::new(),
            },
            TaskDependencyNode {
                task_id: "task-3".to_string(),
                name: "Third Task".to_string(),
                task_type: Some("transform".to_string()),
                dependencies: vec![DependencyConstraint {
                    task_id: "task-1".to_string(),
                    required_status: crate::dependency::DependencyStatus::Success,
                    timeout_ms: None,
                }],
                estimated_duration_ms: Some(750),
                priority: 0,
                metadata: HashMap::new(),
            },
        ]
    }

    #[test]
    fn test_parallelization_request_creation() {
        let workflow_id = Uuid::new_v4();
        let tasks = sample_tasks();

        let request = ParallelizationRequest::new(workflow_id, tasks.clone());

        assert_eq!(request.workflow_id, workflow_id);
        assert_eq!(request.tasks.len(), 3);
        assert_eq!(request.config.max_concurrency, 8);
    }

    #[test]
    fn test_parallelization_config_defaults() {
        let config = ParallelizationConfig::default();

        assert_eq!(config.max_concurrency, 8);
        assert!(config.respect_soft_dependencies);
        assert!(config.analyze_data_dependencies);
        assert!(config.consider_resource_constraints);
        assert!(config.optimize_makespan);
        assert!(!config.balance_load);
    }

    #[test]
    fn test_sequential_plan_creation() {
        let tasks = sample_tasks();
        let plan = ParallelExecutionPlan::sequential(&tasks);

        assert_eq!(plan.total_phases, 3);
        assert_eq!(plan.max_parallelism, 1);
        assert_eq!(plan.strategy, ExecutionStrategy::Sequential);
        assert_eq!(plan.phases.len(), 3);
    }

    #[test]
    fn test_execution_ref_creation() {
        let workflow_id = Uuid::new_v4();
        let request_id = Uuid::new_v4();

        let exec_ref = ParallelizationExecutionRef::new(workflow_id, request_id);

        assert_eq!(exec_ref.workflow_id, workflow_id);
        assert_eq!(exec_ref.request_id, request_id);
        assert!(exec_ref.trace_id.is_some());
        assert!(exec_ref.span_id.is_some());
    }

    #[test]
    fn test_parallelization_status_serialization() {
        let statuses = vec![
            ParallelizationStatus::Success,
            ParallelizationStatus::NoOpportunities,
            ParallelizationStatus::SuccessWithWarnings,
            ParallelizationStatus::Failed,
            ParallelizationStatus::Timeout,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let parsed: ParallelizationStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, parsed);
        }
    }

    #[test]
    fn test_execution_strategy_serialization() {
        let strategies = vec![
            ExecutionStrategy::Sequential,
            ExecutionStrategy::StaticParallel,
            ExecutionStrategy::DynamicParallel,
            ExecutionStrategy::Pipeline,
            ExecutionStrategy::Hybrid,
        ];

        for strategy in strategies {
            let json = serde_json::to_string(&strategy).unwrap();
            let parsed: ExecutionStrategy = serde_json::from_str(&json).unwrap();
            assert_eq!(strategy, parsed);
        }
    }

    #[test]
    fn test_eligibility_serialization() {
        let eligibilities = vec![
            ParallelizationEligibility::Eligible,
            ParallelizationEligibility::EligibleWithConstraints,
            ParallelizationEligibility::NotEligible,
            ParallelizationEligibility::Unknown,
        ];

        for eligibility in eligibilities {
            let json = serde_json::to_string(&eligibility).unwrap();
            let parsed: ParallelizationEligibility = serde_json::from_str(&json).unwrap();
            assert_eq!(eligibility, parsed);
        }
    }
}
