// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Dependency Resolver Agent implementation.
//!
//! # Agent Contract
//!
//! **Classification**: DEPENDENCY ANALYSIS
//!
//! **Purpose**: Resolve task dependencies and produce a valid execution order
//! for complex workflows. Analyze task dependency graphs (DAGs), detect
//! dependency conflicts or cycles, and produce ordered or parallelizable
//! execution plans.
//!
//! ## What This Agent MAY Do
//!
//! - Analyze task dependency graphs (DAGs)
//! - Detect dependency conflicts or cycles
//! - Produce ordered execution plans (topological sort)
//! - Generate parallelizable execution groups
//! - Compute critical path analysis
//! - Emit dependency resolution results
//!
//! ## What This Agent MUST NOT Do
//!
//! - Intercept runtime traffic directly (that is Edge)
//! - Enforce security policies (that is Shield)
//! - Emit anomaly detections (that is Sentinel)
//! - Perform optimization analysis (that is Auto-Optimizer)
//! - Modify schemas dynamically
//! - Connect directly to Google SQL
//! - Execute SQL statements
//!
//! ## CLI Endpoints
//!
//! - `dependency resolve` - Resolve dependencies and produce execution plan
//! - `dependency inspect` - Inspect a previous resolution
//! - `dependency replay` - Replay a previous resolution decision
//! - `dependency validate` - Validate a dependency graph
//!
//! ## decision_type
//!
//! `"dependency_resolution"`

use crate::adapters::observatory::ObservatoryAdapter;
use crate::adapters::ruvector_service::{DecisionEventRecord, RuVectorServiceClient};
use crate::error::{OrchestratorError, Result};
use chrono::{DateTime, Utc};
use petgraph::algo::{has_path_connecting, toposort};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;
use uuid::Uuid;

use crate::feu_collector::FeuSpanCollector;
use agentics_contracts::feu::SpanStatus;

/// Agent version.
pub const AGENT_VERSION: &str = "1.0.0";

/// Agent ID prefix.
pub const AGENT_ID_PREFIX: &str = "dependency-resolver";

// ============================================================================
// AGENT CONFIGURATION
// ============================================================================

/// Configuration for the Dependency Resolver Agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyResolverConfig {
    /// Maximum depth for dependency traversal.
    pub max_depth: usize,
    /// Maximum parallel group size.
    pub max_parallel_size: usize,
    /// Whether to compute critical path.
    pub compute_critical_path: bool,
    /// Default timeout in seconds.
    pub timeout_seconds: u64,
}

impl Default for DependencyResolverConfig {
    fn default() -> Self {
        Self {
            max_depth: 100,
            max_parallel_size: 16,
            compute_critical_path: true,
            timeout_seconds: 300,
        }
    }
}

// ============================================================================
// INPUT/OUTPUT TYPES
// ============================================================================

/// Request to resolve dependencies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveRequest {
    /// Workflow ID.
    pub workflow_id: Uuid,
    /// Tasks with dependencies.
    pub tasks: Vec<TaskNode>,
    /// Resolution configuration.
    #[serde(default)]
    pub config: ResolutionConfig,
}

/// A task node with its dependencies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNode {
    /// Task identifier.
    pub task_id: String,
    /// Task name (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Dependencies (task IDs this task depends on).
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Estimated duration in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_duration_ms: Option<u64>,
    /// Priority (0-100).
    #[serde(default = "default_priority")]
    pub priority: u8,
    /// Whether task can run in parallel.
    #[serde(default = "default_true")]
    pub parallelizable: bool,
}

fn default_priority() -> u8 {
    50
}

fn default_true() -> bool {
    true
}

impl TaskNode {
    /// Creates a new task node.
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            name: None,
            depends_on: Vec::new(),
            estimated_duration_ms: None,
            priority: default_priority(),
            parallelizable: true,
        }
    }

    /// Adds a dependency.
    pub fn with_dependency(mut self, dep: impl Into<String>) -> Self {
        self.depends_on.push(dep.into());
        self
    }

    /// Adds multiple dependencies.
    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.depends_on.extend(deps);
        self
    }
}

/// Resolution configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionConfig {
    /// Maximum depth for traversal.
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    /// Whether to detect cycles.
    #[serde(default = "default_true")]
    pub detect_cycles: bool,
    /// Whether to generate parallel groups.
    #[serde(default = "default_true")]
    pub generate_parallel_groups: bool,
    /// Maximum parallel group size.
    #[serde(default = "default_max_parallel")]
    pub max_parallel_size: usize,
    /// Whether to compute critical path.
    #[serde(default = "default_true")]
    pub compute_critical_path: bool,
}

fn default_max_depth() -> usize {
    100
}

fn default_max_parallel() -> usize {
    16
}

// Hand-written so that `ResolutionConfig::default()` agrees with the `#[serde(default = ...)]`
// values above; `#[derive(Default)]` silently ignores them and yields `false`/`0`.
impl Default for ResolutionConfig {
    fn default() -> Self {
        Self {
            max_depth: default_max_depth(),
            detect_cycles: true,
            generate_parallel_groups: true,
            max_parallel_size: default_max_parallel(),
            compute_critical_path: true,
        }
    }
}

/// Resolution response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveResponse {
    /// Request ID.
    pub request_id: Uuid,
    /// Workflow ID.
    pub workflow_id: Uuid,
    /// Whether resolution succeeded.
    pub success: bool,
    /// Resolution status.
    pub status: ResolutionStatus,
    /// Topologically sorted execution order.
    pub execution_order: Vec<String>,
    /// Parallel execution groups.
    pub parallel_groups: Vec<ParallelGroup>,
    /// Graph summary.
    pub graph_summary: GraphSummary,
    /// Detected issues.
    pub issues: Vec<DependencyIssue>,
    /// Critical path (if computed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub critical_path: Option<CriticalPath>,
    /// Resolution metrics.
    pub metrics: ResolutionMetrics,
    /// Confidence score.
    pub confidence: f64,
    /// Inputs hash.
    pub inputs_hash: String,
    /// Resolution timestamp.
    pub timestamp: DateTime<Utc>,
    /// Error message if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Resolution status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionStatus {
    /// Successfully resolved.
    Resolved,
    /// Resolved with warnings.
    ResolvedWithWarnings,
    /// Cycle detected.
    CycleDetected,
    /// Missing dependencies.
    MissingDependencies,
    /// Failed.
    Failed,
}

impl ResolutionStatus {
    /// Returns true if resolution was successful.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Resolved | Self::ResolvedWithWarnings)
    }
}

/// A parallel execution group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelGroup {
    /// Group index.
    pub group_index: usize,
    /// Task IDs in this group.
    pub task_ids: Vec<String>,
    /// Groups this depends on.
    pub depends_on_groups: Vec<usize>,
    /// Estimated duration (max of tasks).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_duration_ms: Option<u64>,
    /// Whether all tasks can truly run in parallel.
    pub fully_parallel: bool,
}

/// Graph summary statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSummary {
    /// Total task count.
    pub total_tasks: usize,
    /// Root tasks (no dependencies).
    pub root_tasks: usize,
    /// Leaf tasks (no dependents).
    pub leaf_tasks: usize,
    /// Total edges.
    pub total_edges: usize,
    /// Maximum depth.
    pub max_depth: usize,
    /// Average dependencies per task.
    pub avg_dependencies: f64,
}

/// A dependency issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyIssue {
    /// Issue severity.
    pub severity: IssueSeverity,
    /// Issue type.
    pub issue_type: IssueType,
    /// Description.
    pub description: String,
    /// Affected tasks.
    pub affected_tasks: Vec<String>,
    /// Suggestion to fix.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// Issue severity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
}

/// Issue type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueType {
    CyclicDependency,
    MissingTask,
    SelfDependency,
    DuplicateDependency,
    UnreachableTask,
    DepthExceeded,
}

/// Critical path information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalPath {
    /// Task IDs on the path.
    pub path: Vec<String>,
    /// Path length.
    pub length: usize,
    /// Estimated total duration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_duration_ms: Option<u64>,
}

/// Resolution metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionMetrics {
    /// Resolution time in milliseconds.
    pub resolution_time_ms: u64,
    /// Edges traversed.
    pub edges_traversed: usize,
    /// Cycles checked.
    pub cycles_checked: usize,
    /// Algorithm used.
    pub algorithm: String,
}

/// Request to inspect a resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectRequest {
    /// Resolution ID to inspect.
    pub resolution_id: Uuid,
}

/// Inspection response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectResponse {
    /// Resolution ID.
    pub resolution_id: Uuid,
    /// Found resolution (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<ResolveResponse>,
    /// Number of related events.
    pub related_events: usize,
}

/// Request to replay a resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayRequest {
    /// Original resolution ID.
    pub original_resolution_id: Uuid,
}

/// Replay response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResponse {
    /// Original resolution ID.
    pub original_resolution_id: Uuid,
    /// New resolution ID.
    pub new_resolution_id: Uuid,
    /// Inputs hash (should match original).
    pub inputs_hash: String,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
}

/// Request to validate dependencies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateRequest {
    /// Tasks to validate.
    pub tasks: Vec<TaskNode>,
}

/// Validation response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateResponse {
    /// Whether graph is valid.
    pub valid: bool,
    /// Issues found.
    pub issues: Vec<DependencyIssue>,
    /// Graph summary.
    pub summary: GraphSummary,
}

// ============================================================================
// DEPENDENCY RESOLVER AGENT
// ============================================================================

/// Dependency Resolver Agent.
///
/// This agent resolves task dependencies and produces valid execution orders
/// for complex workflows. It implements:
///
/// - Cycle detection using DFS
/// - Topological sorting for execution order
/// - Parallel execution group generation
/// - Critical path analysis
///
/// # Classification
///
/// **DEPENDENCY ANALYSIS**
///
/// # Non-Responsibilities
///
/// This agent MUST NEVER:
/// - Intercept runtime traffic (that is Edge)
/// - Enforce security policies (that is Shield)
/// - Emit anomaly detections (that is Sentinel)
/// - Perform optimization analysis (that is Auto-Optimizer)
/// - Connect directly to Google SQL
/// - Execute SQL
pub struct DependencyResolverAgent {
    /// Agent configuration.
    config: DependencyResolverConfig,
    /// RuVector service client for persistence.
    ruvector_client: RuVectorServiceClient,
    /// Observatory adapter for telemetry.
    observatory: ObservatoryAdapter,
    /// Agent ID.
    agent_id: String,
    /// FEU span collector (optional, for Agentics execution mode).
    feu_collector: Option<FeuSpanCollector>,
}

impl DependencyResolverAgent {
    /// Creates a new Dependency Resolver Agent.
    pub fn new(config: DependencyResolverConfig) -> Self {
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
            feu_collector: None,
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

    /// Sets the FEU span collector for Agentics execution mode.
    pub fn with_feu_collector(mut self, collector: FeuSpanCollector) -> Self {
        self.feu_collector = Some(collector);
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

    // ========================================================================
    // CORE OPERATIONS
    // ========================================================================

    /// Resolves task dependencies and produces an execution plan.
    ///
    /// This is the main entry point for the Dependency Resolver Agent.
    pub async fn resolve(&self, request: ResolveRequest) -> Result<ResolveResponse> {
        let resolution_id = Uuid::new_v4();
        let start_time = Instant::now();
        let started_at = Utc::now();

        // Start FEU agent span if in Agentics execution mode
        let feu_span_id = self.feu_collector.as_ref()
            .map(|c| c.start_agent_span("DependencyResolverAgent"));

        // Emit resolution start telemetry
        self.observatory
            .emit_dependency_resolution_start(resolution_id, request.tasks.len())
            .await;

        // Calculate inputs hash
        let inputs_hash = self.hash_inputs(&request);

        // Build the dependency graph
        let (graph, task_to_node, node_to_task) = self.build_graph(&request.tasks)?;

        // Detect issues (cycles, missing deps, etc.)
        let issues = self.detect_issues(&graph, &task_to_node, &node_to_task, &request.tasks);

        // Check for blocking issues (cycles)
        let has_cycles = issues.iter().any(|i| i.issue_type == IssueType::CyclicDependency);

        let (execution_order, parallel_groups, critical_path, status) = if has_cycles {
            // Cannot resolve with cycles
            (
                Vec::new(),
                Vec::new(),
                None,
                ResolutionStatus::CycleDetected,
            )
        } else {
            // Perform topological sort
            let order = self.topological_sort(&graph, &node_to_task)?;

            // Generate parallel execution groups
            let groups = if request.config.generate_parallel_groups {
                self.generate_parallel_groups(&graph, &task_to_node, &node_to_task, &request.tasks)
            } else {
                Vec::new()
            };

            // Compute critical path
            let crit_path = if request.config.compute_critical_path {
                self.compute_critical_path(&graph, &task_to_node, &node_to_task, &request.tasks)
            } else {
                None
            };

            let status = if issues.iter().any(|i| i.severity == IssueSeverity::Warning) {
                ResolutionStatus::ResolvedWithWarnings
            } else if issues.iter().any(|i| i.issue_type == IssueType::MissingTask) {
                ResolutionStatus::MissingDependencies
            } else {
                ResolutionStatus::Resolved
            };

            (order, groups, crit_path, status)
        };

        // Calculate graph summary
        let graph_summary = self.calculate_graph_summary(&graph, &node_to_task);

        // Calculate metrics
        let resolution_time = start_time.elapsed();
        let metrics = ResolutionMetrics {
            resolution_time_ms: resolution_time.as_millis() as u64,
            edges_traversed: graph.edge_count(),
            cycles_checked: graph.node_count(),
            algorithm: "kahn_topological_sort".to_string(),
        };

        // Calculate confidence
        let confidence = self.calculate_confidence(&status, &issues, &graph_summary);

        // Build response
        let response = ResolveResponse {
            request_id: resolution_id,
            workflow_id: request.workflow_id,
            success: status.is_success(),
            status: status.clone(),
            execution_order,
            parallel_groups,
            graph_summary,
            issues,
            critical_path,
            metrics,
            confidence,
            inputs_hash: inputs_hash.clone(),
            timestamp: started_at,
            error: if status.is_success() {
                None
            } else {
                Some(format!("Resolution failed: {:?}", status))
            },
        };

        // Emit decision event (MANDATORY)
        let decision_event = self.build_decision_event(&request, &response, resolution_id);
        self.ruvector_client
            .persist_decision_event(&decision_event)
            .await
            .map_err(|e| OrchestratorError::Internal(e.to_string()))?;

        // Emit resolution complete telemetry
        self.observatory
            .emit_dependency_resolution_complete(
                resolution_id,
                response.success,
                resolution_time,
            )
            .await;

        // End FEU agent span if in Agentics execution mode
        if let (Some(collector), Some(span_id)) = (&self.feu_collector, &feu_span_id) {
            let feu_status = if response.success { SpanStatus::Ok } else { SpanStatus::Failed };
            collector.end_agent_span(&span_id, feu_status, Vec::new(), vec![
                serde_json::to_value(&decision_event).unwrap_or_default()
            ]);
        }

        Ok(response)
    }

    /// Inspects a previous resolution.
    pub async fn inspect(&self, request: InspectRequest) -> Result<InspectResponse> {
        // Query ruvector-service for the resolution
        let events = self
            .ruvector_client
            .query_by_workflow(request.resolution_id, Some(1))
            .await
            .map_err(|e| OrchestratorError::Internal(e.to_string()))?;

        // Emit decision event for inspection
        let decision_event = DecisionEventRecord {
            id: Uuid::new_v4(),
            agent_id: self.agent_id.clone(),
            agent_version: AGENT_VERSION.to_string(),
            decision_type: "dependency_inspection".to_string(),
            inputs_hash: format!("{:x}", request.resolution_id.as_u128()),
            outputs: serde_json::json!({
                "resolution_found": !events.is_empty(),
                "events_count": events.len(),
            }),
            confidence: 1.0,
            constraints_applied: serde_json::json!({}),
            workflow_execution_id: request.resolution_id,
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

        Ok(InspectResponse {
            resolution_id: request.resolution_id,
            resolution: None, // Would be populated from stored data
            related_events: events.len(),
        })
    }

    /// Replays a previous resolution.
    pub async fn replay(&self, request: ReplayRequest) -> Result<ReplayResponse> {
        let new_resolution_id = Uuid::new_v4();

        // Emit decision event for replay
        let decision_event = DecisionEventRecord {
            id: Uuid::new_v4(),
            agent_id: self.agent_id.clone(),
            agent_version: AGENT_VERSION.to_string(),
            decision_type: "dependency_replay".to_string(),
            inputs_hash: format!("{:x}", request.original_resolution_id.as_u128()),
            outputs: serde_json::json!({
                "original_resolution_id": request.original_resolution_id.to_string(),
                "new_resolution_id": new_resolution_id.to_string(),
            }),
            confidence: 1.0,
            constraints_applied: serde_json::json!({}),
            workflow_execution_id: new_resolution_id,
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

        Ok(ReplayResponse {
            original_resolution_id: request.original_resolution_id,
            new_resolution_id,
            inputs_hash: format!("{:x}", request.original_resolution_id.as_u128()),
            timestamp: Utc::now(),
        })
    }

    /// Validates a dependency graph without resolving.
    pub async fn validate(&self, request: ValidateRequest) -> Result<ValidateResponse> {
        let (graph, task_to_node, node_to_task) = self.build_graph(&request.tasks)?;
        let issues = self.detect_issues(&graph, &task_to_node, &node_to_task, &request.tasks);
        let summary = self.calculate_graph_summary(&graph, &node_to_task);

        let valid = !issues.iter().any(|i| i.severity == IssueSeverity::Error);

        // Emit decision event
        let decision_event = DecisionEventRecord {
            id: Uuid::new_v4(),
            agent_id: self.agent_id.clone(),
            agent_version: AGENT_VERSION.to_string(),
            decision_type: "dependency_validation".to_string(),
            inputs_hash: self.hash_tasks(&request.tasks),
            outputs: serde_json::json!({
                "valid": valid,
                "issues_count": issues.len(),
                "task_count": request.tasks.len(),
            }),
            confidence: 1.0,
            constraints_applied: serde_json::json!({}),
            workflow_execution_id: Uuid::new_v4(),
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

        Ok(ValidateResponse {
            valid,
            issues,
            summary,
        })
    }

    // ========================================================================
    // GRAPH BUILDING
    // ========================================================================

    /// Builds a directed graph from task nodes.
    fn build_graph(
        &self,
        tasks: &[TaskNode],
    ) -> Result<(
        DiGraph<String, ()>,
        HashMap<String, NodeIndex>,
        HashMap<NodeIndex, String>,
    )> {
        let mut graph = DiGraph::new();
        let mut task_to_node = HashMap::new();
        let mut node_to_task = HashMap::new();

        // First pass: create nodes
        for task in tasks {
            let node_idx = graph.add_node(task.task_id.clone());
            task_to_node.insert(task.task_id.clone(), node_idx);
            node_to_task.insert(node_idx, task.task_id.clone());
        }

        // Second pass: add edges
        for task in tasks {
            let target_idx = task_to_node[&task.task_id];
            for dep_id in &task.depends_on {
                if let Some(&source_idx) = task_to_node.get(dep_id) {
                    graph.add_edge(source_idx, target_idx, ());
                }
                // Missing deps are handled in detect_issues
            }
        }

        Ok((graph, task_to_node, node_to_task))
    }

    // ========================================================================
    // CYCLE DETECTION
    // ========================================================================

    /// Detects cycles using DFS with coloring.
    fn detect_cycles(
        &self,
        graph: &DiGraph<String, ()>,
        node_to_task: &HashMap<NodeIndex, String>,
    ) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();
        let mut color: HashMap<NodeIndex, u8> = HashMap::new(); // 0=white, 1=gray, 2=black
        let mut parent: HashMap<NodeIndex, NodeIndex> = HashMap::new();

        for node in graph.node_indices() {
            color.insert(node, 0);
        }

        for start in graph.node_indices() {
            if color[&start] == 0 {
                let mut stack = vec![(start, graph.neighbors(start))];

                while let Some((node, mut neighbors)) = stack.pop() {
                    if color[&node] == 0 {
                        color.insert(node, 1); // Mark as gray (in progress)
                    }

                    let mut all_processed = true;
                    while let Some(neighbor) = neighbors.next() {
                        match color[&neighbor] {
                            0 => {
                                // White - unvisited
                                parent.insert(neighbor, node);
                                stack.push((node, neighbors));
                                stack.push((neighbor, graph.neighbors(neighbor)));
                                all_processed = false;
                                break;
                            }
                            1 => {
                                // Gray - cycle found!
                                let mut cycle = vec![node_to_task[&neighbor].clone()];
                                let mut current = node;
                                while current != neighbor {
                                    cycle.push(node_to_task[&current].clone());
                                    if let Some(&p) = parent.get(&current) {
                                        current = p;
                                    } else {
                                        break;
                                    }
                                }
                                cycle.push(node_to_task[&neighbor].clone());
                                cycle.reverse();
                                cycles.push(cycle);
                            }
                            _ => {} // Black - already processed
                        }
                    }

                    if all_processed {
                        color.insert(node, 2); // Mark as black (done)
                    }
                }
            }
        }

        cycles
    }

    // ========================================================================
    // ISSUE DETECTION
    // ========================================================================

    /// Detects all issues in the dependency graph.
    fn detect_issues(
        &self,
        graph: &DiGraph<String, ()>,
        task_to_node: &HashMap<String, NodeIndex>,
        node_to_task: &HashMap<NodeIndex, String>,
        tasks: &[TaskNode],
    ) -> Vec<DependencyIssue> {
        let mut issues = Vec::new();

        // 1. Detect cycles
        let cycles = self.detect_cycles(graph, node_to_task);
        for cycle in cycles {
            issues.push(DependencyIssue {
                severity: IssueSeverity::Error,
                issue_type: IssueType::CyclicDependency,
                description: format!("Cycle detected: {}", cycle.join(" -> ")),
                affected_tasks: cycle.clone(),
                suggestion: Some("Remove one of the dependencies to break the cycle".to_string()),
            });
        }

        // 2. Detect missing dependencies
        for task in tasks {
            for dep in &task.depends_on {
                if !task_to_node.contains_key(dep) {
                    issues.push(DependencyIssue {
                        severity: IssueSeverity::Error,
                        issue_type: IssueType::MissingTask,
                        description: format!(
                            "Task '{}' depends on non-existent task '{}'",
                            task.task_id, dep
                        ),
                        affected_tasks: vec![task.task_id.clone(), dep.clone()],
                        suggestion: Some(format!("Add task '{}' or remove the dependency", dep)),
                    });
                }
            }
        }

        // 3. Detect self-dependencies
        for task in tasks {
            if task.depends_on.contains(&task.task_id) {
                issues.push(DependencyIssue {
                    severity: IssueSeverity::Error,
                    issue_type: IssueType::SelfDependency,
                    description: format!("Task '{}' depends on itself", task.task_id),
                    affected_tasks: vec![task.task_id.clone()],
                    suggestion: Some("Remove the self-dependency".to_string()),
                });
            }
        }

        // 4. Detect duplicate dependencies
        for task in tasks {
            let mut seen = HashSet::new();
            for dep in &task.depends_on {
                if !seen.insert(dep) {
                    issues.push(DependencyIssue {
                        severity: IssueSeverity::Warning,
                        issue_type: IssueType::DuplicateDependency,
                        description: format!(
                            "Task '{}' has duplicate dependency on '{}'",
                            task.task_id, dep
                        ),
                        affected_tasks: vec![task.task_id.clone()],
                        suggestion: Some("Remove the duplicate dependency".to_string()),
                    });
                }
            }
        }

        issues
    }

    // ========================================================================
    // TOPOLOGICAL SORT
    // ========================================================================

    /// Performs topological sort using Kahn's algorithm.
    fn topological_sort(
        &self,
        graph: &DiGraph<String, ()>,
        node_to_task: &HashMap<NodeIndex, String>,
    ) -> Result<Vec<String>> {
        match toposort(graph, None) {
            Ok(sorted) => Ok(sorted
                .into_iter()
                .map(|idx| node_to_task[&idx].clone())
                .collect()),
            Err(_) => Err(OrchestratorError::CyclicDependency),
        }
    }

    // ========================================================================
    // PARALLEL GROUP GENERATION
    // ========================================================================

    /// Generates parallel execution groups (level-based grouping).
    fn generate_parallel_groups(
        &self,
        graph: &DiGraph<String, ()>,
        task_to_node: &HashMap<String, NodeIndex>,
        node_to_task: &HashMap<NodeIndex, String>,
        tasks: &[TaskNode],
    ) -> Vec<ParallelGroup> {
        let mut groups: Vec<ParallelGroup> = Vec::new();
        let mut task_levels: HashMap<NodeIndex, usize> = HashMap::new();

        // Calculate level for each task (longest path from any root)
        let mut in_degree: HashMap<NodeIndex, usize> = HashMap::new();
        for node in graph.node_indices() {
            in_degree.insert(node, graph.neighbors_directed(node, Direction::Incoming).count());
        }

        // Start with root nodes (in_degree = 0)
        let mut queue: VecDeque<NodeIndex> = VecDeque::new();
        for (node, &degree) in &in_degree {
            if degree == 0 {
                task_levels.insert(*node, 0);
                queue.push_back(*node);
            }
        }

        // BFS to assign levels
        while let Some(node) = queue.pop_front() {
            let current_level = task_levels[&node];
            for neighbor in graph.neighbors_directed(node, Direction::Outgoing) {
                let new_level = current_level + 1;
                let entry = task_levels.entry(neighbor).or_insert(0);
                *entry = (*entry).max(new_level);

                let deg = in_degree.get_mut(&neighbor).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push_back(neighbor);
                }
            }
        }

        // Group tasks by level
        let max_level = task_levels.values().copied().max().unwrap_or(0);
        let task_map: HashMap<String, &TaskNode> =
            tasks.iter().map(|t| (t.task_id.clone(), t)).collect();

        for level in 0..=max_level {
            let task_ids: Vec<String> = task_levels
                .iter()
                .filter(|(_, &l)| l == level)
                .map(|(node, _)| node_to_task[node].clone())
                .collect();

            if task_ids.is_empty() {
                continue;
            }

            // Calculate estimated duration (max of tasks in group)
            let estimated_duration = task_ids
                .iter()
                .filter_map(|id| task_map.get(id).and_then(|t| t.estimated_duration_ms))
                .max();

            // Check if all tasks are parallelizable
            let fully_parallel = task_ids
                .iter()
                .all(|id| task_map.get(id).map(|t| t.parallelizable).unwrap_or(true));

            // Determine dependent groups
            let depends_on_groups: Vec<usize> = if level > 0 {
                (0..level).collect()
            } else {
                Vec::new()
            };

            groups.push(ParallelGroup {
                group_index: level,
                task_ids,
                depends_on_groups,
                estimated_duration_ms: estimated_duration,
                fully_parallel,
            });
        }

        groups
    }

    // ========================================================================
    // CRITICAL PATH ANALYSIS
    // ========================================================================

    /// Computes the critical path (longest path through the DAG).
    fn compute_critical_path(
        &self,
        graph: &DiGraph<String, ()>,
        task_to_node: &HashMap<String, NodeIndex>,
        node_to_task: &HashMap<NodeIndex, String>,
        tasks: &[TaskNode],
    ) -> Option<CriticalPath> {
        let task_map: HashMap<String, &TaskNode> =
            tasks.iter().map(|t| (t.task_id.clone(), t)).collect();

        // Calculate earliest finish time for each node
        let mut earliest_finish: HashMap<NodeIndex, u64> = HashMap::new();
        let mut predecessor: HashMap<NodeIndex, Option<NodeIndex>> = HashMap::new();

        // Get topological order
        let topo_order = match toposort(graph, None) {
            Ok(order) => order,
            Err(_) => return None,
        };

        // Initialize
        for &node in &topo_order {
            earliest_finish.insert(node, 0);
            predecessor.insert(node, None);
        }

        // Forward pass
        for &node in &topo_order {
            let task_id = &node_to_task[&node];
            let duration = task_map
                .get(task_id)
                .and_then(|t| t.estimated_duration_ms)
                .unwrap_or(1000);

            let max_incoming = graph
                .neighbors_directed(node, Direction::Incoming)
                .map(|pred| earliest_finish[&pred])
                .max()
                .unwrap_or(0);

            earliest_finish.insert(node, max_incoming + duration);

            // Track predecessor with max finish time
            let best_pred = graph
                .neighbors_directed(node, Direction::Incoming)
                .max_by_key(|pred| earliest_finish[pred]);
            predecessor.insert(node, best_pred);
        }

        // Find the node with maximum earliest finish time
        let end_node = topo_order
            .iter()
            .max_by_key(|&&node| earliest_finish[&node])?;

        // Backtrack to build critical path
        let mut path = Vec::new();
        let mut current = Some(*end_node);

        while let Some(node) = current {
            path.push(node_to_task[&node].clone());
            current = predecessor[&node];
        }

        path.reverse();

        let total_duration = earliest_finish[end_node];

        Some(CriticalPath {
            length: path.len(),
            estimated_duration_ms: Some(total_duration),
            path,
        })
    }

    // ========================================================================
    // GRAPH SUMMARY
    // ========================================================================

    /// Calculates graph summary statistics.
    fn calculate_graph_summary(
        &self,
        graph: &DiGraph<String, ()>,
        node_to_task: &HashMap<NodeIndex, String>,
    ) -> GraphSummary {
        let total_tasks = graph.node_count();
        let total_edges = graph.edge_count();

        let root_tasks = graph
            .node_indices()
            .filter(|&n| graph.neighbors_directed(n, Direction::Incoming).count() == 0)
            .count();

        let leaf_tasks = graph
            .node_indices()
            .filter(|&n| graph.neighbors_directed(n, Direction::Outgoing).count() == 0)
            .count();

        // Calculate max depth using BFS
        let max_depth = self.calculate_max_depth(graph);

        let avg_dependencies = if total_tasks > 0 {
            total_edges as f64 / total_tasks as f64
        } else {
            0.0
        };

        GraphSummary {
            total_tasks,
            root_tasks,
            leaf_tasks,
            total_edges,
            max_depth,
            avg_dependencies,
        }
    }

    /// Calculates maximum depth of the DAG.
    fn calculate_max_depth(&self, graph: &DiGraph<String, ()>) -> usize {
        let mut max_depth = 0;
        let mut depths: HashMap<NodeIndex, usize> = HashMap::new();

        // Find root nodes
        let roots: Vec<NodeIndex> = graph
            .node_indices()
            .filter(|&n| graph.neighbors_directed(n, Direction::Incoming).count() == 0)
            .collect();

        // BFS from each root
        for root in roots {
            let mut queue = VecDeque::new();
            queue.push_back((root, 0usize));

            while let Some((node, depth)) = queue.pop_front() {
                let entry = depths.entry(node).or_insert(0);
                *entry = (*entry).max(depth);
                max_depth = max_depth.max(depth);

                for neighbor in graph.neighbors_directed(node, Direction::Outgoing) {
                    queue.push_back((neighbor, depth + 1));
                }
            }
        }

        max_depth
    }

    // ========================================================================
    // CONFIDENCE CALCULATION
    // ========================================================================

    /// Calculates confidence score for the resolution.
    fn calculate_confidence(
        &self,
        status: &ResolutionStatus,
        issues: &[DependencyIssue],
        summary: &GraphSummary,
    ) -> f64 {
        let mut confidence = 1.0;

        // Reduce confidence based on status
        match status {
            ResolutionStatus::Resolved => {}
            ResolutionStatus::ResolvedWithWarnings => confidence -= 0.1,
            ResolutionStatus::MissingDependencies => confidence -= 0.4,
            ResolutionStatus::CycleDetected => confidence = 0.0,
            ResolutionStatus::Failed => confidence = 0.0,
        }

        // Reduce for each issue
        for issue in issues {
            match issue.severity {
                IssueSeverity::Error => confidence -= 0.3,
                IssueSeverity::Warning => confidence -= 0.05,
                IssueSeverity::Info => {}
            }
        }

        // Reduce slightly for very deep graphs (complexity penalty)
        if summary.max_depth > 20 {
            confidence -= 0.05 * ((summary.max_depth - 20) as f64 / 10.0);
        }

        confidence.clamp(0.0, 1.0)
    }

    // ========================================================================
    // HASHING
    // ========================================================================

    /// Hashes the request inputs.
    fn hash_inputs(&self, request: &ResolveRequest) -> String {
        let json = serde_json::to_string(request).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Hashes task list.
    fn hash_tasks(&self, tasks: &[TaskNode]) -> String {
        let json = serde_json::to_string(tasks).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    // ========================================================================
    // DECISION EVENT
    // ========================================================================

    /// Builds a decision event for the resolution.
    fn build_decision_event(
        &self,
        request: &ResolveRequest,
        response: &ResolveResponse,
        resolution_id: Uuid,
    ) -> DecisionEventRecord {
        DecisionEventRecord {
            id: Uuid::new_v4(),
            agent_id: self.agent_id.clone(),
            agent_version: AGENT_VERSION.to_string(),
            decision_type: "dependency_resolution".to_string(),
            inputs_hash: response.inputs_hash.clone(),
            outputs: serde_json::to_value(&response).unwrap_or_default(),
            confidence: response.confidence,
            constraints_applied: serde_json::json!({
                "max_depth": request.config.max_depth,
                "max_parallel_size": request.config.max_parallel_size,
                "cycle_detection": request.config.detect_cycles,
                "strategy": "topological_sort",
            }),
            workflow_execution_id: request.workflow_id,
            step_execution_id: Some(resolution_id),
            trace_id: Some(Uuid::new_v4().to_string()),
            span_id: Some(Uuid::new_v4().to_string()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tasks() -> Vec<TaskNode> {
        vec![
            TaskNode::new("step1"),
            TaskNode::new("step2").with_dependency("step1"),
            TaskNode::new("step3").with_dependency("step1"),
            TaskNode::new("step4").with_dependencies(vec!["step2".to_string(), "step3".to_string()]),
        ]
    }

    fn create_cyclic_tasks() -> Vec<TaskNode> {
        vec![
            TaskNode::new("a").with_dependency("c"),
            TaskNode::new("b").with_dependency("a"),
            TaskNode::new("c").with_dependency("b"),
        ]
    }

    #[test]
    fn test_agent_creation() {
        let agent = DependencyResolverAgent::new(DependencyResolverConfig::default());
        assert!(agent.agent_id().starts_with(AGENT_ID_PREFIX));
        assert_eq!(agent.version(), AGENT_VERSION);
    }

    #[test]
    fn test_task_node_builder() {
        let node = TaskNode::new("test")
            .with_dependency("dep1")
            .with_dependencies(vec!["dep2".to_string(), "dep3".to_string()]);

        assert_eq!(node.task_id, "test");
        assert_eq!(node.depends_on.len(), 3);
    }

    #[test]
    fn test_build_graph() {
        let agent = DependencyResolverAgent::new(DependencyResolverConfig::default());
        let tasks = create_test_tasks();

        let (graph, task_to_node, node_to_task) = agent.build_graph(&tasks).unwrap();

        assert_eq!(graph.node_count(), 4);
        assert_eq!(graph.edge_count(), 4); // step1->step2, step1->step3, step2->step4, step3->step4
    }

    #[test]
    fn test_cycle_detection() {
        let agent = DependencyResolverAgent::new(DependencyResolverConfig::default());
        let tasks = create_cyclic_tasks();

        let (graph, task_to_node, node_to_task) = agent.build_graph(&tasks).unwrap();
        let cycles = agent.detect_cycles(&graph, &node_to_task);

        assert!(!cycles.is_empty());
    }

    #[test]
    fn test_topological_sort() {
        let agent = DependencyResolverAgent::new(DependencyResolverConfig::default());
        let tasks = create_test_tasks();

        let (graph, task_to_node, node_to_task) = agent.build_graph(&tasks).unwrap();
        let order = agent.topological_sort(&graph, &node_to_task).unwrap();

        // step1 must come first
        assert_eq!(order[0], "step1");
        // step4 must come last
        assert_eq!(order[3], "step4");
        // step2 and step3 must come before step4
        let step2_pos = order.iter().position(|x| x == "step2").unwrap();
        let step3_pos = order.iter().position(|x| x == "step3").unwrap();
        let step4_pos = order.iter().position(|x| x == "step4").unwrap();
        assert!(step2_pos < step4_pos);
        assert!(step3_pos < step4_pos);
    }

    #[test]
    fn test_parallel_groups() {
        let agent = DependencyResolverAgent::new(DependencyResolverConfig::default());
        let tasks = create_test_tasks();

        let (graph, task_to_node, node_to_task) = agent.build_graph(&tasks).unwrap();
        let groups = agent.generate_parallel_groups(&graph, &task_to_node, &node_to_task, &tasks);

        // Should have 3 groups: [step1], [step2, step3], [step4]
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].task_ids, vec!["step1"]);
        assert!(groups[1].task_ids.contains(&"step2".to_string()));
        assert!(groups[1].task_ids.contains(&"step3".to_string()));
        assert_eq!(groups[2].task_ids, vec!["step4"]);
    }

    #[test]
    fn test_issue_detection() {
        let agent = DependencyResolverAgent::new(DependencyResolverConfig::default());

        // Test self-dependency
        let tasks = vec![TaskNode::new("task").with_dependency("task")];
        let (graph, task_to_node, node_to_task) = agent.build_graph(&tasks).unwrap();
        let issues = agent.detect_issues(&graph, &task_to_node, &node_to_task, &tasks);

        assert!(issues.iter().any(|i| i.issue_type == IssueType::SelfDependency));
    }

    #[test]
    fn test_missing_dependency() {
        let agent = DependencyResolverAgent::new(DependencyResolverConfig::default());

        let tasks = vec![TaskNode::new("task").with_dependency("nonexistent")];
        let (graph, task_to_node, node_to_task) = agent.build_graph(&tasks).unwrap();
        let issues = agent.detect_issues(&graph, &task_to_node, &node_to_task, &tasks);

        assert!(issues.iter().any(|i| i.issue_type == IssueType::MissingTask));
    }

    #[test]
    fn test_graph_summary() {
        let agent = DependencyResolverAgent::new(DependencyResolverConfig::default());
        let tasks = create_test_tasks();

        let (graph, _, node_to_task) = agent.build_graph(&tasks).unwrap();
        let summary = agent.calculate_graph_summary(&graph, &node_to_task);

        assert_eq!(summary.total_tasks, 4);
        assert_eq!(summary.root_tasks, 1); // step1
        assert_eq!(summary.leaf_tasks, 1); // step4
        assert_eq!(summary.total_edges, 4);
    }

    #[tokio::test]
    async fn test_resolve_success() {
        let agent = DependencyResolverAgent::new(DependencyResolverConfig::default());
        let request = ResolveRequest {
            workflow_id: Uuid::new_v4(),
            tasks: create_test_tasks(),
            config: ResolutionConfig::default(),
        };

        let response = agent.resolve(request).await.unwrap();

        assert!(response.success);
        assert_eq!(response.status, ResolutionStatus::Resolved);
        assert_eq!(response.execution_order.len(), 4);
        assert!(!response.parallel_groups.is_empty());
        assert!(response.confidence > 0.9);
    }

    #[tokio::test]
    async fn test_resolve_with_cycle() {
        let agent = DependencyResolverAgent::new(DependencyResolverConfig::default());
        let request = ResolveRequest {
            workflow_id: Uuid::new_v4(),
            tasks: create_cyclic_tasks(),
            config: ResolutionConfig::default(),
        };

        let response = agent.resolve(request).await.unwrap();

        assert!(!response.success);
        assert_eq!(response.status, ResolutionStatus::CycleDetected);
        assert!(response.issues.iter().any(|i| i.issue_type == IssueType::CyclicDependency));
        assert_eq!(response.confidence, 0.0);
    }

    #[tokio::test]
    async fn test_validate() {
        let agent = DependencyResolverAgent::new(DependencyResolverConfig::default());
        let request = ValidateRequest {
            tasks: create_test_tasks(),
        };

        let response = agent.validate(request).await.unwrap();

        assert!(response.valid);
        assert!(response.issues.is_empty());
    }

    #[test]
    fn test_resolution_status() {
        assert!(ResolutionStatus::Resolved.is_success());
        assert!(ResolutionStatus::ResolvedWithWarnings.is_success());
        assert!(!ResolutionStatus::CycleDetected.is_success());
        assert!(!ResolutionStatus::Failed.is_success());
    }

    #[test]
    fn test_confidence_calculation() {
        let agent = DependencyResolverAgent::new(DependencyResolverConfig::default());

        let summary = GraphSummary {
            total_tasks: 4,
            root_tasks: 1,
            leaf_tasks: 1,
            total_edges: 4,
            max_depth: 3,
            avg_dependencies: 1.0,
        };

        // Perfect resolution
        let confidence = agent.calculate_confidence(&ResolutionStatus::Resolved, &[], &summary);
        assert_eq!(confidence, 1.0);

        // With cycle
        let confidence = agent.calculate_confidence(&ResolutionStatus::CycleDetected, &[], &summary);
        assert_eq!(confidence, 0.0);
    }
}
