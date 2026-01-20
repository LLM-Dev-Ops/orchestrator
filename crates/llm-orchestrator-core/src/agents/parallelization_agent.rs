// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Parallelization Agent implementation.
//!
//! # Agent Contract
//!
//! **Classification**: EXECUTION PLANNING (maps to TASK_COORDINATION)
//!
//! **Purpose**: Identify tasks that can execute concurrently to improve
//! workflow throughput.
//!
//! ## What This Agent MAY Do
//!
//! - Analyze task independence and shared constraints
//! - Determine safe parallel execution groups
//! - Emit parallel execution plans
//! - Optimize execution efficiency without altering logic
//! - Manage task dependencies for parallel grouping
//! - Coordinate parallel execution planning
//!
//! ## What This Agent MUST NOT Do
//!
//! - Execute workflow steps directly (delegates to Workflow Orchestrator)
//! - Modify task inputs or outputs
//! - Change provider configurations
//! - Intercept runtime traffic directly (that is Edge)
//! - Enforce security policies (that is Shield)
//! - Emit anomaly detections (that is Sentinel)
//! - Perform optimization analysis (that is Auto-Optimizer)
//! - Connect directly to Google SQL
//! - Execute SQL statements
//! - Trigger retries or recovery operations
//!
//! ## CLI Endpoints
//!
//! - `parallel analyze` - Analyze workflow for parallelization opportunities
//! - `parallel inspect` - Inspect a previous analysis result
//! - `parallel replay` - Replay a previous analysis

use crate::adapters::observatory::ObservatoryAdapter;
use crate::adapters::ruvector_service::{DecisionEventRecord, RuVectorServiceClient};
use crate::dag::WorkflowDAG;
use crate::error::{OrchestratorError, Result};
use crate::workflow::Workflow;
use agentics_contracts::dependency::{
    DependencyConstraint, DependencyGraphSummary, DependencyStatus, ParallelExecutionGroup,
    TaskDependencyNode,
};
use agentics_contracts::parallelization::{
    ExecutionPhase, ExecutionStrategy, OptimizationMetrics, ParallelExecutionPlan,
    ParallelTask, ParallelizationConfig, ParallelizationDecisionEvent,
    ParallelizationDecisionOutputs, ParallelizationConstraintsApplied, ParallelizationEligibility,
    ParallelizationExecutionRef, ParallelizationIssue, ParallelizationIssueSeverity,
    ParallelizationIssueType, ParallelizationRequest, ParallelizationResult,
    ParallelizationStatus, ParallelizationSummary, PhaseResources, ResourceConstraints,
    TaskResources, PARALLELIZATION_AGENT_ID, PARALLELIZATION_AGENT_VERSION,
};
use agentics_contracts::{AgentId, AgentVersion};
use chrono::{DateTime, Utc};
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Parallelization Agent.
///
/// This agent analyzes workflow tasks to identify opportunities for
/// parallel execution, optimizing throughput without altering task logic.
///
/// # Classification
///
/// **EXECUTION PLANNING** (maps to TASK_COORDINATION)
///
/// # Non-Responsibilities
///
/// This agent MUST NEVER:
/// - Execute workflow steps directly
/// - Modify task inputs or outputs
/// - Intercept runtime traffic (that is Edge)
/// - Enforce security policies (that is Shield)
/// - Emit anomaly detections (that is Sentinel)
/// - Connect directly to Google SQL
/// - Execute SQL
pub struct ParallelizationAgent {
    /// Agent configuration.
    config: ParallelizationAgentConfig,
    /// RuVector service client for persistence.
    ruvector_client: RuVectorServiceClient,
    /// Observatory adapter for telemetry.
    observatory: ObservatoryAdapter,
    /// Agent ID.
    agent_id: String,
}

/// Configuration for the Parallelization Agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelizationAgentConfig {
    /// Default maximum concurrent tasks.
    pub default_max_concurrency: u32,
    /// Whether to enable resource-aware planning by default.
    pub resource_aware: bool,
    /// Whether to enable makespan optimization by default.
    pub optimize_makespan: bool,
    /// Analysis timeout in milliseconds.
    pub timeout_ms: Option<u64>,
}

impl Default for ParallelizationAgentConfig {
    fn default() -> Self {
        Self {
            default_max_concurrency: 8,
            resource_aware: true,
            optimize_makespan: true,
            timeout_ms: Some(30000), // 30 seconds
        }
    }
}

impl ParallelizationAgent {
    /// Creates a new Parallelization Agent.
    pub fn new(config: ParallelizationAgentConfig) -> Self {
        let agent_id = format!(
            "{}-{}",
            PARALLELIZATION_AGENT_ID,
            &Uuid::new_v4().to_string()[..8]
        );

        Self {
            config,
            ruvector_client: RuVectorServiceClient::disabled(),
            observatory: ObservatoryAdapter::disabled(),
            agent_id,
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

    /// Returns the agent ID.
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    /// Returns the agent version.
    pub fn version(&self) -> &str {
        PARALLELIZATION_AGENT_VERSION
    }

    // =========================================================================
    // Core Operations
    // =========================================================================

    /// Analyzes a workflow for parallelization opportunities.
    ///
    /// This is the primary CLI endpoint for parallelization analysis.
    pub async fn analyze(&self, request: ParallelizationRequest) -> Result<ParallelizationResult> {
        let start_time = std::time::Instant::now();

        tracing::info!(
            workflow_id = %request.workflow_id,
            tasks_count = request.tasks.len(),
            max_concurrency = request.config.max_concurrency,
            "Starting parallelization analysis"
        );

        // Calculate inputs hash for reproducibility
        let inputs_hash = self.hash_inputs(&request);

        // Validate request
        self.validate_request(&request)?;

        // Build dependency graph
        let (graph, node_map) = self.build_dependency_graph(&request.tasks)?;

        // Detect issues (cycles, missing dependencies, etc.)
        let issues = self.detect_issues(&graph, &node_map, &request.tasks);

        // Check for blocking errors
        let has_blocking_errors = issues
            .iter()
            .any(|i| i.severity == ParallelizationIssueSeverity::Error);

        if has_blocking_errors {
            return self
                .build_failed_result(&request, issues, &inputs_hash, start_time)
                .await;
        }

        // Perform topological sort
        let topo_order = match toposort(&graph, None) {
            Ok(order) => order,
            Err(cycle) => {
                let cycle_task = node_map
                    .iter()
                    .find(|(_, &idx)| idx == cycle.node_id())
                    .map(|(id, _)| id.clone())
                    .unwrap_or_default();

                let mut issues = issues;
                issues.push(ParallelizationIssue {
                    severity: ParallelizationIssueSeverity::Error,
                    issue_type: ParallelizationIssueType::CircularDependency,
                    affected_tasks: vec![cycle_task],
                    description: "Circular dependency detected in task graph".to_string(),
                    impact: "Cannot determine execution order".to_string(),
                    suggestion: Some("Remove circular dependencies to enable parallelization".to_string()),
                });

                return self
                    .build_failed_result(&request, issues, &inputs_hash, start_time)
                    .await;
            }
        };

        // Create reverse node map
        let reverse_map: HashMap<NodeIndex, String> = node_map
            .iter()
            .map(|(k, v)| (*v, k.clone()))
            .collect();

        // Compute parallel execution phases
        let phases = self.compute_parallel_phases(
            &graph,
            &topo_order,
            &reverse_map,
            &request.tasks,
            &request.config,
            &request.resource_constraints,
        );

        // Build execution plan
        let execution_plan = self.build_execution_plan(&phases, &request.tasks);

        // Calculate optimization metrics
        let optimization_metrics = self.calculate_optimization_metrics(
            &request.tasks,
            &execution_plan,
        );

        // Build summary
        let summary = self.build_summary(&request.tasks, &execution_plan, &graph);

        // Determine status
        let status = if execution_plan.max_parallelism <= 1 {
            ParallelizationStatus::NoOpportunities
        } else if !issues.is_empty() {
            ParallelizationStatus::SuccessWithWarnings
        } else {
            ParallelizationStatus::Success
        };

        let result = ParallelizationResult {
            request_id: request.request_id,
            workflow_id: request.workflow_id,
            status: status.clone(),
            execution_plan,
            summary,
            optimization_metrics,
            issues,
            analyzed_at: Utc::now(),
        };

        // Calculate confidence
        let confidence = self.calculate_confidence(&request, &result);

        // Build and persist decision event
        let decision_event = self.build_decision_event(
            &request,
            &result,
            &inputs_hash,
            confidence,
        );

        self.persist_decision_event(&decision_event).await?;

        // Emit telemetry
        self.emit_telemetry(&request, &result, start_time.elapsed()).await;

        tracing::info!(
            workflow_id = %request.workflow_id,
            status = ?status,
            phases = result.execution_plan.total_phases,
            max_parallelism = result.execution_plan.max_parallelism,
            speedup = result.optimization_metrics.speedup_factor,
            confidence = confidence,
            "Parallelization analysis complete"
        );

        Ok(result)
    }

    /// Analyzes a workflow from a Workflow struct.
    pub async fn analyze_workflow(&self, workflow: &Workflow) -> Result<ParallelizationResult> {
        let tasks = self.workflow_to_tasks(workflow)?;
        let request = ParallelizationRequest::new(workflow.id, tasks)
            .with_config(ParallelizationConfig {
                max_concurrency: self.config.default_max_concurrency,
                optimize_makespan: self.config.optimize_makespan,
                ..Default::default()
            });

        self.analyze(request).await
    }

    /// Inspects a previous analysis result.
    pub async fn inspect(&self, request_id: Uuid) -> Result<InspectResponse> {
        tracing::debug!(request_id = %request_id, "Inspecting parallelization analysis");

        // Query decision events for this request from ruvector-service
        let events = self
            .ruvector_client
            .query_by_agent(&self.agent_id, Some(100))
            .await
            .map_err(|e| OrchestratorError::Internal(e.to_string()))?;

        // Find the relevant event
        let event = events
            .iter()
            .find(|e| e.metadata.get("request_id").and_then(|v: &serde_json::Value| v.as_str()) == Some(&request_id.to_string()))
            .cloned();

        let found = event.is_some();
        Ok(InspectResponse {
            request_id,
            decision_event: event,
            found,
        })
    }

    /// Replays a previous analysis with the same inputs.
    pub async fn replay(&self, request: ParallelizationRequest) -> Result<ParallelizationResult> {
        tracing::info!(
            request_id = %request.request_id,
            "Replaying parallelization analysis"
        );

        // Simply re-run the analysis (deterministic)
        self.analyze(request).await
    }

    // =========================================================================
    // Internal Implementation
    // =========================================================================

    /// Validates the parallelization request.
    fn validate_request(&self, request: &ParallelizationRequest) -> Result<()> {
        if request.tasks.is_empty() {
            return Err(OrchestratorError::validation("No tasks provided for analysis"));
        }

        // Check for duplicate task IDs
        let mut seen = HashSet::new();
        for task in &request.tasks {
            if !seen.insert(&task.task_id) {
                return Err(OrchestratorError::validation(format!(
                    "Duplicate task ID: {}",
                    task.task_id
                )));
            }
        }

        // Validate configuration
        if request.config.max_concurrency == 0 {
            return Err(OrchestratorError::validation("max_concurrency must be > 0"));
        }

        Ok(())
    }

    /// Builds a directed graph from task dependencies.
    fn build_dependency_graph(
        &self,
        tasks: &[TaskDependencyNode],
    ) -> Result<(DiGraph<String, ()>, HashMap<String, NodeIndex>)> {
        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();

        // Add all nodes
        for task in tasks {
            let idx = graph.add_node(task.task_id.clone());
            node_map.insert(task.task_id.clone(), idx);
        }

        // Add edges (dependency -> dependent)
        for task in tasks {
            let task_idx = node_map[&task.task_id];
            for dep in &task.dependencies {
                if let Some(&dep_idx) = node_map.get(&dep.task_id) {
                    graph.add_edge(dep_idx, task_idx, ());
                }
            }
        }

        Ok((graph, node_map))
    }

    /// Detects issues in the dependency graph.
    fn detect_issues(
        &self,
        graph: &DiGraph<String, ()>,
        node_map: &HashMap<String, NodeIndex>,
        tasks: &[TaskDependencyNode],
    ) -> Vec<ParallelizationIssue> {
        let mut issues = Vec::new();

        // Check for missing dependencies
        let task_ids: HashSet<_> = tasks.iter().map(|t| &t.task_id).collect();
        for task in tasks {
            for dep in &task.dependencies {
                if !task_ids.contains(&dep.task_id) {
                    issues.push(ParallelizationIssue {
                        severity: ParallelizationIssueSeverity::Error,
                        issue_type: ParallelizationIssueType::UnknownDependency,
                        affected_tasks: vec![task.task_id.clone()],
                        description: format!(
                            "Task '{}' depends on unknown task '{}'",
                            task.task_id, dep.task_id
                        ),
                        impact: "Cannot determine execution order".to_string(),
                        suggestion: Some(format!(
                            "Add task '{}' to the workflow or remove the dependency",
                            dep.task_id
                        )),
                    });
                }
            }
        }

        // Check for self-dependencies
        for task in tasks {
            for dep in &task.dependencies {
                if dep.task_id == task.task_id {
                    issues.push(ParallelizationIssue {
                        severity: ParallelizationIssueSeverity::Error,
                        issue_type: ParallelizationIssueType::CircularDependency,
                        affected_tasks: vec![task.task_id.clone()],
                        description: format!("Task '{}' depends on itself", task.task_id),
                        impact: "Self-dependency creates infinite loop".to_string(),
                        suggestion: Some("Remove self-dependency".to_string()),
                    });
                }
            }
        }

        issues
    }

    /// Computes parallel execution phases using a level-based approach.
    fn compute_parallel_phases(
        &self,
        graph: &DiGraph<String, ()>,
        topo_order: &[NodeIndex],
        reverse_map: &HashMap<NodeIndex, String>,
        tasks: &[TaskDependencyNode],
        config: &ParallelizationConfig,
        resource_constraints: &ResourceConstraints,
    ) -> Vec<Vec<String>> {
        let task_map: HashMap<_, _> = tasks.iter().map(|t| (&t.task_id, t)).collect();

        // Calculate levels for each node
        let mut levels: HashMap<NodeIndex, u32> = HashMap::new();
        for &node in topo_order {
            let max_parent_level = graph
                .neighbors_directed(node, petgraph::Direction::Incoming)
                .filter_map(|parent| levels.get(&parent))
                .max()
                .copied()
                .unwrap_or(0);

            let level = if graph.neighbors_directed(node, petgraph::Direction::Incoming).count() == 0 {
                0
            } else {
                max_parent_level + 1
            };

            levels.insert(node, level);
        }

        // Group by level
        let max_level = levels.values().max().copied().unwrap_or(0);
        let mut phases: Vec<Vec<String>> = vec![Vec::new(); (max_level + 1) as usize];

        for (node, &level) in &levels {
            if let Some(task_id) = reverse_map.get(node) {
                phases[level as usize].push(task_id.clone());
            }
        }

        // Apply concurrency limits
        let mut final_phases = Vec::new();
        for phase in phases {
            if phase.is_empty() {
                continue;
            }

            // Split phases that exceed max_concurrency
            for chunk in phase.chunks(config.max_concurrency as usize) {
                final_phases.push(chunk.to_vec());
            }
        }

        // Apply resource constraints if enabled
        if config.consider_resource_constraints {
            final_phases = self.apply_resource_constraints(
                final_phases,
                &task_map,
                resource_constraints,
            );
        }

        final_phases
    }

    /// Applies resource constraints to phases.
    fn apply_resource_constraints(
        &self,
        mut phases: Vec<Vec<String>>,
        task_map: &HashMap<&String, &TaskDependencyNode>,
        constraints: &ResourceConstraints,
    ) -> Vec<Vec<String>> {
        // If no constraints, return as-is
        if constraints.max_total_memory_bytes.is_none()
            && constraints.max_total_cpu.is_none()
            && constraints.max_total_gpu.is_none()
            && constraints.max_total_tokens.is_none()
        {
            return phases;
        }

        // For now, we don't have resource info per task, so return as-is
        // In production, this would split phases based on resource limits
        phases
    }

    /// Builds the execution plan from phases.
    fn build_execution_plan(
        &self,
        phases: &[Vec<String>],
        tasks: &[TaskDependencyNode],
    ) -> ParallelExecutionPlan {
        let task_map: HashMap<_, _> = tasks.iter().map(|t| (&t.task_id, t)).collect();

        let mut execution_phases = Vec::new();
        let mut task_phase_map = HashMap::new();
        let mut max_parallelism = 1usize;
        let mut critical_path_tasks = Vec::new();

        for (phase_index, phase_tasks) in phases.iter().enumerate() {
            let depends_on_phases = if phase_index > 0 {
                vec![(phase_index - 1) as u32]
            } else {
                Vec::new()
            };

            let parallel_tasks: Vec<ParallelTask> = phase_tasks
                .iter()
                .map(|task_id| {
                    let task = task_map.get(task_id);
                    let duration = task
                        .and_then(|t| t.estimated_duration_ms)
                        .unwrap_or(1000);

                    task_phase_map.insert(task_id.clone(), phase_index as u32);

                    ParallelTask {
                        task_id: task_id.clone(),
                        name: task.map(|t| t.name.clone()).unwrap_or_default(),
                        eligibility: if phase_tasks.len() > 1 {
                            ParallelizationEligibility::Eligible
                        } else {
                            ParallelizationEligibility::NotEligible
                        },
                        safe_concurrent_with: phase_tasks
                            .iter()
                            .filter(|&t| t != task_id)
                            .cloned()
                            .collect(),
                        conflicts_with: Vec::new(),
                        estimated_duration_ms: duration,
                        resources: TaskResources::default(),
                    }
                })
                .collect();

            max_parallelism = max_parallelism.max(parallel_tasks.len());

            // Track critical path (phases with single task)
            if parallel_tasks.len() == 1 {
                critical_path_tasks.push(parallel_tasks[0].task_id.clone());
            }

            let phase_duration = parallel_tasks
                .iter()
                .map(|t| t.estimated_duration_ms)
                .max()
                .unwrap_or(0);

            execution_phases.push(ExecutionPhase {
                phase_index: phase_index as u32,
                parallel_tasks,
                depends_on_phases,
                estimated_duration_ms: phase_duration,
                total_resources: PhaseResources::default(),
                is_critical: phase_tasks.len() == 1,
            });
        }

        let strategy = if max_parallelism <= 1 {
            ExecutionStrategy::Sequential
        } else if max_parallelism <= 4 {
            ExecutionStrategy::StaticParallel
        } else {
            ExecutionStrategy::DynamicParallel
        };

        ParallelExecutionPlan {
            plan_id: Uuid::new_v4(),
            phases: execution_phases,
            total_phases: phases.len() as u32,
            max_parallelism: max_parallelism as u32,
            strategy,
            critical_path_tasks,
            task_phase_map,
        }
    }

    /// Calculates optimization metrics.
    fn calculate_optimization_metrics(
        &self,
        tasks: &[TaskDependencyNode],
        plan: &ParallelExecutionPlan,
    ) -> OptimizationMetrics {
        // Sequential duration = sum of all task durations
        let sequential_duration: u64 = tasks
            .iter()
            .map(|t| t.estimated_duration_ms.unwrap_or(1000))
            .sum();

        // Parallel duration = sum of phase durations (max of parallel tasks)
        let parallel_duration: u64 = plan
            .phases
            .iter()
            .map(|p| p.estimated_duration_ms)
            .sum();

        // Critical path duration (phases with single task)
        let critical_path_duration: u64 = plan
            .phases
            .iter()
            .filter(|p| p.is_critical)
            .map(|p| p.estimated_duration_ms)
            .sum();

        let speedup = if parallel_duration > 0 {
            sequential_duration as f64 / parallel_duration as f64
        } else {
            1.0
        };

        // Efficiency = speedup / max_parallelism
        let efficiency = if plan.max_parallelism > 0 {
            speedup / plan.max_parallelism as f64
        } else {
            1.0
        };

        // Resource utilization (approximation)
        let total_work: u64 = tasks.iter().map(|t| t.estimated_duration_ms.unwrap_or(1000)).sum();
        let total_capacity = parallel_duration * plan.max_parallelism as u64;
        let utilization = if total_capacity > 0 {
            total_work as f64 / total_capacity as f64
        } else {
            1.0
        };

        // Overhead estimate (parallel coordination)
        let overhead = (plan.total_phases.saturating_sub(1) * 10) as u64; // 10ms per phase transition

        OptimizationMetrics {
            sequential_duration_ms: sequential_duration,
            parallel_duration_ms: parallel_duration,
            speedup_factor: speedup,
            efficiency_score: efficiency.min(1.0),
            resource_utilization: utilization.min(1.0),
            critical_path_duration_ms: critical_path_duration,
            overhead_estimate_ms: overhead,
        }
    }

    /// Builds the analysis summary.
    fn build_summary(
        &self,
        tasks: &[TaskDependencyNode],
        plan: &ParallelExecutionPlan,
        graph: &DiGraph<String, ()>,
    ) -> ParallelizationSummary {
        let total_tasks = tasks.len() as u32;
        let parallelizable = plan
            .phases
            .iter()
            .filter(|p| p.parallel_tasks.len() > 1)
            .flat_map(|p| &p.parallel_tasks)
            .count() as u32;

        let sequential = total_tasks.saturating_sub(parallelizable);

        let total_parallelism: usize = plan.phases.iter().map(|p| p.parallel_tasks.len()).sum();
        let avg_parallelism = if !plan.phases.is_empty() {
            total_parallelism as f64 / plan.phases.len() as f64
        } else {
            1.0
        };

        // Build graph summary
        let total_deps = graph.edge_count() as u32;
        let max_depth = self.calculate_max_depth(graph);
        let root_tasks = graph
            .node_indices()
            .filter(|&n| graph.neighbors_directed(n, petgraph::Direction::Incoming).count() == 0)
            .count() as u32;
        let leaf_tasks = graph
            .node_indices()
            .filter(|&n| graph.neighbors_directed(n, petgraph::Direction::Outgoing).count() == 0)
            .count() as u32;

        ParallelizationSummary {
            total_tasks,
            parallelizable_tasks: parallelizable,
            sequential_tasks: sequential,
            execution_phases: plan.total_phases,
            max_parallelism: plan.max_parallelism,
            avg_parallelism,
            graph_summary: DependencyGraphSummary {
                total_tasks,
                total_dependencies: total_deps,
                max_depth,
                root_tasks,
                leaf_tasks,
                parallel_groups: plan.total_phases,
                has_cycles: false, // Already validated
            },
        }
    }

    /// Calculates maximum depth of the dependency graph.
    fn calculate_max_depth(&self, graph: &DiGraph<String, ()>) -> u32 {
        if graph.node_count() == 0 {
            return 0;
        }

        // Use topological sort to calculate depths
        if let Ok(topo) = toposort(graph, None) {
            let mut depths: HashMap<NodeIndex, u32> = HashMap::new();
            for node in topo {
                let max_parent_depth = graph
                    .neighbors_directed(node, petgraph::Direction::Incoming)
                    .filter_map(|p| depths.get(&p))
                    .max()
                    .copied()
                    .unwrap_or(0);

                let depth = if graph.neighbors_directed(node, petgraph::Direction::Incoming).count() == 0 {
                    0
                } else {
                    max_parent_depth + 1
                };

                depths.insert(node, depth);
            }

            depths.values().max().copied().unwrap_or(0)
        } else {
            0 // Has cycles
        }
    }

    /// Calculates confidence score for the analysis.
    fn calculate_confidence(
        &self,
        request: &ParallelizationRequest,
        result: &ParallelizationResult,
    ) -> f64 {
        let mut confidence: f64 = 0.0;

        // Dependency completeness (0.3 weight)
        // Higher if all dependencies are explicit
        let dep_completeness = 1.0; // Assume complete for now
        confidence += dep_completeness * 0.3;

        // Duration estimation availability (0.2 weight)
        let duration_coverage = request
            .tasks
            .iter()
            .filter(|t| t.estimated_duration_ms.is_some())
            .count() as f64
            / request.tasks.len().max(1) as f64;
        confidence += duration_coverage * 0.2;

        // Analysis success (0.3 weight)
        let success_score = match result.status {
            ParallelizationStatus::Success => 1.0,
            ParallelizationStatus::SuccessWithWarnings => 0.8,
            ParallelizationStatus::NoOpportunities => 0.9,
            ParallelizationStatus::Failed | ParallelizationStatus::Timeout => 0.0,
        };
        confidence += success_score * 0.3;

        // No issues detected (0.2 weight)
        let issue_score = if result.issues.is_empty() {
            1.0
        } else if result.issues.iter().any(|i| i.severity == ParallelizationIssueSeverity::Error) {
            0.0
        } else {
            0.7
        };
        confidence += issue_score * 0.2;

        confidence.clamp(0.0, 1.0)
    }

    /// Builds the decision event.
    fn build_decision_event(
        &self,
        request: &ParallelizationRequest,
        result: &ParallelizationResult,
        inputs_hash: &str,
        confidence: f64,
    ) -> ParallelizationDecisionEvent {
        ParallelizationDecisionEvent {
            id: Uuid::new_v4(),
            agent_id: AgentId::from_string(&self.agent_id),
            agent_version: AgentVersion::parse(PARALLELIZATION_AGENT_VERSION).unwrap(),
            decision_type: "parallel_execution_plan".to_string(),
            inputs_hash: inputs_hash.to_string(),
            outputs: ParallelizationDecisionOutputs {
                status: result.status.clone(),
                tasks_analyzed: request.tasks.len() as u32,
                parallelizable_tasks: result.summary.parallelizable_tasks,
                execution_phases: result.execution_plan.total_phases,
                max_parallelism: result.execution_plan.max_parallelism,
                speedup_factor: result.optimization_metrics.speedup_factor,
                issues_count: result.issues.len() as u32,
                strategy: result.execution_plan.strategy.clone(),
            },
            confidence,
            constraints_applied: ParallelizationConstraintsApplied {
                max_concurrency: request.config.max_concurrency,
                resource_constrained: request.config.consider_resource_constraints,
                soft_dependencies_respected: request.config.respect_soft_dependencies,
                data_dependencies_analyzed: request.config.analyze_data_dependencies,
                makespan_optimized: request.config.optimize_makespan,
                timeout_ms: request.config.timeout_ms,
            },
            execution_ref: ParallelizationExecutionRef::new(request.workflow_id, request.request_id),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Persists the decision event to ruvector-service.
    async fn persist_decision_event(&self, event: &ParallelizationDecisionEvent) -> Result<()> {
        // Convert to generic DecisionEventRecord for ruvector
        let record = DecisionEventRecord {
            id: event.id,
            agent_id: self.agent_id.clone(),
            agent_version: PARALLELIZATION_AGENT_VERSION.to_string(),
            decision_type: event.decision_type.clone(),
            inputs_hash: event.inputs_hash.clone(),
            outputs: serde_json::to_value(&event.outputs).unwrap_or_default(),
            confidence: event.confidence,
            constraints_applied: serde_json::to_value(&event.constraints_applied).unwrap_or_default(),
            workflow_execution_id: event.execution_ref.workflow_id,
            step_execution_id: None,
            trace_id: event.execution_ref.trace_id.clone(),
            span_id: event.execution_ref.span_id.clone(),
            timestamp: event.timestamp,
            metadata: event.metadata.clone(),
        };

        self.ruvector_client
            .persist_decision_event(&record)
            .await
            .map_err(|e| OrchestratorError::Internal(e.to_string()))?;

        Ok(())
    }

    /// Builds a failed result.
    async fn build_failed_result(
        &self,
        request: &ParallelizationRequest,
        issues: Vec<ParallelizationIssue>,
        inputs_hash: &str,
        start_time: std::time::Instant,
    ) -> Result<ParallelizationResult> {
        let result = ParallelizationResult {
            request_id: request.request_id,
            workflow_id: request.workflow_id,
            status: ParallelizationStatus::Failed,
            execution_plan: ParallelExecutionPlan::sequential(&request.tasks),
            summary: ParallelizationSummary {
                total_tasks: request.tasks.len() as u32,
                parallelizable_tasks: 0,
                sequential_tasks: request.tasks.len() as u32,
                execution_phases: request.tasks.len() as u32,
                max_parallelism: 1,
                avg_parallelism: 1.0,
                graph_summary: DependencyGraphSummary {
                    total_tasks: request.tasks.len() as u32,
                    total_dependencies: 0,
                    max_depth: 0,
                    root_tasks: 0,
                    leaf_tasks: 0,
                    parallel_groups: 0,
                    has_cycles: true,
                },
            },
            optimization_metrics: OptimizationMetrics {
                sequential_duration_ms: 0,
                parallel_duration_ms: 0,
                speedup_factor: 1.0,
                efficiency_score: 0.0,
                resource_utilization: 0.0,
                critical_path_duration_ms: 0,
                overhead_estimate_ms: 0,
            },
            issues,
            analyzed_at: Utc::now(),
        };

        // Persist failed decision event
        let decision_event = self.build_decision_event(request, &result, inputs_hash, 0.0);
        self.persist_decision_event(&decision_event).await?;

        // Emit telemetry
        self.emit_telemetry(request, &result, start_time.elapsed()).await;

        Ok(result)
    }

    /// Emits telemetry for the analysis.
    async fn emit_telemetry(
        &self,
        request: &ParallelizationRequest,
        result: &ParallelizationResult,
        duration: std::time::Duration,
    ) {
        tracing::info!(
            workflow_id = %request.workflow_id,
            status = ?result.status,
            tasks = request.tasks.len(),
            phases = result.execution_plan.total_phases,
            max_parallelism = result.execution_plan.max_parallelism,
            speedup = result.optimization_metrics.speedup_factor,
            duration_ms = duration.as_millis() as u64,
            "parallelization.analysis.complete"
        );
    }

    /// Hashes the request inputs.
    fn hash_inputs(&self, request: &ParallelizationRequest) -> String {
        let json = serde_json::to_string(request).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Converts a Workflow to TaskDependencyNodes.
    fn workflow_to_tasks(&self, workflow: &Workflow) -> Result<Vec<TaskDependencyNode>> {
        let tasks = workflow
            .steps
            .iter()
            .map(|step| TaskDependencyNode {
                task_id: step.id.clone(),
                name: step.id.clone(),
                task_type: Some(format!("{:?}", step.step_type)),
                dependencies: step
                    .depends_on
                    .iter()
                    .map(|dep_id| DependencyConstraint {
                        task_id: dep_id.clone(),
                        required_status: DependencyStatus::Success,
                        timeout_ms: None,
                    })
                    .collect(),
                estimated_duration_ms: step.timeout_seconds.map(|s| s * 1000),
                priority: 0,
                metadata: HashMap::new(),
            })
            .collect();

        Ok(tasks)
    }
}

// =============================================================================
// Response Types
// =============================================================================

/// Response for inspect operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectResponse {
    /// Request ID inspected.
    pub request_id: Uuid,
    /// Decision event if found.
    pub decision_event: Option<DecisionEventRecord>,
    /// Whether the analysis was found.
    pub found: bool,
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
                task_id: "task-a".to_string(),
                name: "Task A".to_string(),
                task_type: Some("llm".to_string()),
                dependencies: Vec::new(),
                estimated_duration_ms: Some(1000),
                priority: 0,
                metadata: HashMap::new(),
            },
            TaskDependencyNode {
                task_id: "task-b".to_string(),
                name: "Task B".to_string(),
                task_type: Some("transform".to_string()),
                dependencies: vec![DependencyConstraint {
                    task_id: "task-a".to_string(),
                    required_status: DependencyStatus::Success,
                    timeout_ms: None,
                }],
                estimated_duration_ms: Some(500),
                priority: 0,
                metadata: HashMap::new(),
            },
            TaskDependencyNode {
                task_id: "task-c".to_string(),
                name: "Task C".to_string(),
                task_type: Some("transform".to_string()),
                dependencies: vec![DependencyConstraint {
                    task_id: "task-a".to_string(),
                    required_status: DependencyStatus::Success,
                    timeout_ms: None,
                }],
                estimated_duration_ms: Some(750),
                priority: 0,
                metadata: HashMap::new(),
            },
            TaskDependencyNode {
                task_id: "task-d".to_string(),
                name: "Task D".to_string(),
                task_type: Some("action".to_string()),
                dependencies: vec![
                    DependencyConstraint {
                        task_id: "task-b".to_string(),
                        required_status: DependencyStatus::Success,
                        timeout_ms: None,
                    },
                    DependencyConstraint {
                        task_id: "task-c".to_string(),
                        required_status: DependencyStatus::Success,
                        timeout_ms: None,
                    },
                ],
                estimated_duration_ms: Some(250),
                priority: 0,
                metadata: HashMap::new(),
            },
        ]
    }

    #[test]
    fn test_agent_creation() {
        let agent = ParallelizationAgent::new(ParallelizationAgentConfig::default());
        assert!(agent.agent_id().starts_with(PARALLELIZATION_AGENT_ID));
        assert_eq!(agent.version(), PARALLELIZATION_AGENT_VERSION);
    }

    #[tokio::test]
    async fn test_analyze_parallel_tasks() {
        let agent = ParallelizationAgent::new(ParallelizationAgentConfig::default());
        let tasks = sample_tasks();
        let workflow_id = Uuid::new_v4();

        let request = ParallelizationRequest::new(workflow_id, tasks);
        let result = agent.analyze(request).await.unwrap();

        assert_eq!(result.status, ParallelizationStatus::Success);
        // Task A runs alone, then B and C can run in parallel, then D
        assert!(result.execution_plan.max_parallelism >= 2);
        assert!(result.optimization_metrics.speedup_factor >= 1.0);
    }

    #[tokio::test]
    async fn test_analyze_sequential_tasks() {
        let agent = ParallelizationAgent::new(ParallelizationAgentConfig::default());

        // Chain of dependent tasks (no parallelism possible)
        let tasks = vec![
            TaskDependencyNode {
                task_id: "t1".to_string(),
                name: "T1".to_string(),
                task_type: None,
                dependencies: Vec::new(),
                estimated_duration_ms: Some(100),
                priority: 0,
                metadata: HashMap::new(),
            },
            TaskDependencyNode {
                task_id: "t2".to_string(),
                name: "T2".to_string(),
                task_type: None,
                dependencies: vec![DependencyConstraint {
                    task_id: "t1".to_string(),
                    required_status: DependencyStatus::Success,
                    timeout_ms: None,
                }],
                estimated_duration_ms: Some(100),
                priority: 0,
                metadata: HashMap::new(),
            },
            TaskDependencyNode {
                task_id: "t3".to_string(),
                name: "T3".to_string(),
                task_type: None,
                dependencies: vec![DependencyConstraint {
                    task_id: "t2".to_string(),
                    required_status: DependencyStatus::Success,
                    timeout_ms: None,
                }],
                estimated_duration_ms: Some(100),
                priority: 0,
                metadata: HashMap::new(),
            },
        ];

        let request = ParallelizationRequest::new(Uuid::new_v4(), tasks);
        let result = agent.analyze(request).await.unwrap();

        assert_eq!(result.status, ParallelizationStatus::NoOpportunities);
        assert_eq!(result.execution_plan.max_parallelism, 1);
    }

    #[tokio::test]
    async fn test_analyze_missing_dependency() {
        let agent = ParallelizationAgent::new(ParallelizationAgentConfig::default());

        let tasks = vec![TaskDependencyNode {
            task_id: "t1".to_string(),
            name: "T1".to_string(),
            task_type: None,
            dependencies: vec![DependencyConstraint {
                task_id: "nonexistent".to_string(),
                required_status: DependencyStatus::Success,
                timeout_ms: None,
            }],
            estimated_duration_ms: Some(100),
            priority: 0,
            metadata: HashMap::new(),
        }];

        let request = ParallelizationRequest::new(Uuid::new_v4(), tasks);
        let result = agent.analyze(request).await.unwrap();

        assert_eq!(result.status, ParallelizationStatus::Failed);
        assert!(!result.issues.is_empty());
        assert!(result.issues.iter().any(|i| i.issue_type == ParallelizationIssueType::UnknownDependency));
    }

    #[test]
    fn test_config_defaults() {
        let config = ParallelizationAgentConfig::default();
        assert_eq!(config.default_max_concurrency, 8);
        assert!(config.resource_aware);
        assert!(config.optimize_makespan);
    }

    #[test]
    fn test_hash_inputs_deterministic() {
        let agent = ParallelizationAgent::new(ParallelizationAgentConfig::default());
        let request = ParallelizationRequest::new(Uuid::new_v4(), sample_tasks());

        let hash1 = agent.hash_inputs(&request);
        let hash2 = agent.hash_inputs(&request);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 = 64 hex chars
    }
}
