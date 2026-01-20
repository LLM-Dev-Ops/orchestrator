// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Dependency resolution schemas for the Dependency Resolver Agent.
//!
//! This module defines the contract schemas for dependency resolution
//! within the Agent Infrastructure Constitution. The Dependency Resolver
//! Agent is classified under TASK_COORDINATION and is responsible for
//! analyzing task dependencies and determining execution order.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

use crate::agent::{AgentId, AgentVersion};

/// Dependency Resolver Agent ID constant.
pub const DEPENDENCY_RESOLVER_AGENT_ID: &str = "dependency-resolver";

/// Dependency Resolver Agent Version.
pub const DEPENDENCY_RESOLVER_AGENT_VERSION: &str = "0.1.0";

/// Dependency constraint for task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyConstraint {
    /// Task ID that this depends on.
    pub task_id: String,
    /// Required status of the dependency.
    pub required_status: DependencyStatus,
    /// Timeout for waiting on this dependency (milliseconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// Dependency resolution status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyStatus {
    /// Dependency must complete successfully.
    Success,
    /// Dependency must complete (success or failure).
    Complete,
    /// Dependency must be started.
    Started,
    /// Dependency must be skipped.
    Skipped,
    /// Any status is acceptable.
    Any,
}

impl Default for DependencyStatus {
    fn default() -> Self {
        Self::Success
    }
}

/// Dependency graph for workflow execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    /// Graph ID.
    pub id: Uuid,
    /// Workflow ID this graph belongs to.
    pub workflow_id: Uuid,
    /// Nodes in the graph (task IDs).
    pub nodes: Vec<String>,
    /// Edges in the graph (from -> to).
    pub edges: Vec<DependencyEdge>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// Edge in a dependency graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    /// Source task ID.
    pub from: String,
    /// Target task ID.
    pub to: String,
    /// Edge type.
    pub edge_type: EdgeType,
}

/// Type of dependency edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    /// Hard dependency - target cannot start until source completes.
    Hard,
    /// Soft dependency - target prefers source to complete but can proceed.
    Soft,
    /// Data dependency - target needs data from source.
    Data,
    /// Conditional dependency - depends on condition evaluation.
    Conditional,
}

impl Default for EdgeType {
    fn default() -> Self {
        Self::Hard
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_constraint() {
        let constraint = DependencyConstraint {
            task_id: "task-1".to_string(),
            required_status: DependencyStatus::Success,
            timeout_ms: Some(30000),
        };

        assert_eq!(constraint.task_id, "task-1");
        assert_eq!(constraint.required_status, DependencyStatus::Success);
    }

    #[test]
    fn test_dependency_graph() {
        let graph = DependencyGraph {
            id: Uuid::new_v4(),
            workflow_id: Uuid::new_v4(),
            nodes: vec!["task-1".to_string(), "task-2".to_string()],
            edges: vec![DependencyEdge {
                from: "task-1".to_string(),
                to: "task-2".to_string(),
                edge_type: EdgeType::Hard,
            }],
            created_at: Utc::now(),
        };

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
    }
}

// =============================================================================
// Dependency Resolution Agent Contract Types
// =============================================================================
// These types define the input/output contracts for the Dependency Resolver
// Agent according to the Agent Infrastructure Constitution.

/// Request for dependency resolution.
///
/// Input contract for the Dependency Resolver Agent's resolve operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyResolutionRequest {
    /// Unique request ID.
    pub request_id: Uuid,
    /// Workflow ID for which to resolve dependencies.
    pub workflow_id: Uuid,
    /// List of tasks with their dependency information.
    pub tasks: Vec<TaskDependencyNode>,
    /// Resolution configuration.
    #[serde(default)]
    pub config: DependencyResolutionConfig,
    /// Request timestamp.
    pub timestamp: DateTime<Utc>,
}

/// Result of dependency resolution.
///
/// Output contract for the Dependency Resolver Agent's resolve operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyResolutionResult {
    /// Request ID this result corresponds to.
    pub request_id: Uuid,
    /// Workflow ID.
    pub workflow_id: Uuid,
    /// Resolution status.
    pub status: ResolutionStatus,
    /// Resolved execution order (topologically sorted task IDs).
    pub execution_order: Vec<String>,
    /// Groups of tasks that can be executed in parallel.
    pub parallel_groups: Vec<ParallelExecutionGroup>,
    /// Summary of the dependency graph.
    pub graph_summary: DependencyGraphSummary,
    /// Critical path through the workflow.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub critical_path: Option<CriticalPath>,
    /// Issues detected during resolution.
    pub issues: Vec<DependencyIssue>,
    /// Resolution timestamp.
    pub resolved_at: DateTime<Utc>,
}

/// A task node with its dependency information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDependencyNode {
    /// Task ID.
    pub task_id: String,
    /// Task name for display.
    pub name: String,
    /// Task type or category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_type: Option<String>,
    /// Direct dependencies (task IDs this task depends on).
    pub dependencies: Vec<DependencyConstraint>,
    /// Estimated execution duration in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_duration_ms: Option<u64>,
    /// Task priority (higher = more important).
    #[serde(default)]
    pub priority: i32,
    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Configuration for dependency resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyResolutionConfig {
    /// Maximum depth for transitive dependency resolution.
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,
    /// Whether to detect and report cycles.
    #[serde(default = "default_true")]
    pub detect_cycles: bool,
    /// Whether to calculate critical path.
    #[serde(default = "default_true")]
    pub calculate_critical_path: bool,
    /// Whether to optimize for parallel execution.
    #[serde(default = "default_true")]
    pub optimize_parallelism: bool,
    /// Timeout for resolution in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

fn default_max_depth() -> u32 {
    100
}

fn default_true() -> bool {
    true
}

impl Default for DependencyResolutionConfig {
    fn default() -> Self {
        Self {
            max_depth: 100,
            detect_cycles: true,
            calculate_critical_path: true,
            optimize_parallelism: true,
            timeout_ms: None,
        }
    }
}

/// Group of tasks that can be executed in parallel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelExecutionGroup {
    /// Group index (execution order).
    pub group_index: u32,
    /// Task IDs in this group.
    pub task_ids: Vec<String>,
    /// All dependencies for this group must be from previous groups.
    pub depends_on_groups: Vec<u32>,
}

/// Summary of the dependency graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraphSummary {
    /// Total number of tasks.
    pub total_tasks: u32,
    /// Total number of dependencies.
    pub total_dependencies: u32,
    /// Maximum depth of the dependency tree.
    pub max_depth: u32,
    /// Number of root tasks (no dependencies).
    pub root_tasks: u32,
    /// Number of leaf tasks (no dependents).
    pub leaf_tasks: u32,
    /// Number of parallel execution groups.
    pub parallel_groups: u32,
    /// Whether the graph contains cycles.
    pub has_cycles: bool,
}

/// An issue detected during dependency resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyIssue {
    /// Issue severity.
    pub severity: IssueSeverity,
    /// Issue type.
    pub issue_type: IssueType,
    /// Affected task IDs.
    pub affected_tasks: Vec<String>,
    /// Human-readable description.
    pub description: String,
    /// Suggested resolution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// Severity of a dependency issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    /// Informational only.
    Info,
    /// Warning - resolution can proceed but may have issues.
    Warning,
    /// Error - resolution cannot proceed without fixing.
    Error,
}

/// Type of dependency issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueType {
    /// Circular dependency detected.
    CyclicDependency,
    /// Missing dependency (referenced task doesn't exist).
    MissingDependency,
    /// Duplicate dependency declaration.
    DuplicateDependency,
    /// Dependency on self.
    SelfDependency,
    /// Maximum depth exceeded.
    DepthExceeded,
    /// Orphaned task (no path from root).
    OrphanedTask,
    /// Conflicting constraints.
    ConflictingConstraints,
}

/// Critical path through the workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalPath {
    /// Task IDs in the critical path (in execution order).
    pub task_ids: Vec<String>,
    /// Total estimated duration in milliseconds.
    pub total_duration_ms: u64,
    /// Number of tasks in the critical path.
    pub task_count: u32,
}

/// Status of dependency resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionStatus {
    /// Resolution completed successfully.
    Success,
    /// Resolution completed with warnings.
    SuccessWithWarnings,
    /// Resolution failed due to errors.
    Failed,
    /// Resolution timed out.
    Timeout,
    /// Resolution was cancelled.
    Cancelled,
}

// =============================================================================
// Dependency Resolver Agent DecisionEvent Types
// =============================================================================

/// DecisionEvent specific to the Dependency Resolver Agent.
///
/// Emitted for every invocation of the Dependency Resolver Agent
/// according to the Agent Infrastructure Constitution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyResolutionDecisionEvent {
    /// Agent identifier.
    pub agent_id: AgentId,
    /// Agent version.
    pub agent_version: AgentVersion,
    /// Decision type.
    pub decision_type: DependencyDecisionType,
    /// SHA-256 hash of the input request.
    pub inputs_hash: String,
    /// Resolution outputs.
    pub outputs: DependencyResolutionOutputs,
    /// Confidence in the resolution (0.0 to 1.0).
    pub confidence: f64,
    /// Constraints that were applied during resolution.
    pub constraints_applied: DependencyConstraintsApplied,
    /// Execution reference for tracing.
    pub execution_ref: DependencyExecutionRef,
    /// Event timestamp.
    pub timestamp: DateTime<Utc>,
}

/// Decision type for dependency resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyDecisionType {
    /// Resolve dependencies for a workflow.
    Resolve,
    /// Validate a dependency graph.
    Validate,
    /// Analyze dependencies for optimization.
    Analyze,
    /// Reorder tasks based on dependencies.
    Reorder,
}

/// Outputs from dependency resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyResolutionOutputs {
    /// Resolution status.
    pub status: ResolutionStatus,
    /// Number of tasks resolved.
    pub tasks_resolved: u32,
    /// Number of parallel groups identified.
    pub parallel_groups: u32,
    /// Number of issues detected.
    pub issues_count: u32,
    /// Whether critical path was calculated.
    pub critical_path_calculated: bool,
}

/// Constraints applied during dependency resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyConstraintsApplied {
    /// Maximum depth constraint.
    pub max_depth: u32,
    /// Cycle detection enabled.
    pub cycle_detection: bool,
    /// Parallelism optimization enabled.
    pub parallelism_optimization: bool,
    /// Timeout constraint in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// Execution reference for dependency resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyExecutionRef {
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
