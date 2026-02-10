// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Swarm Coordinator Agent implementation.
//!
//! # Agent Contract
//!
//! **Classification**: MULTI-AGENT COORDINATION (maps to TASK_COORDINATION)
//!
//! **Purpose**: Coordinate fan-out / fan-in execution across multiple agents
//! working toward a shared objective.
//!
//! ## What This Agent MAY Do
//!
//! - Spawn and manage parallel agent executions
//! - Aggregate results from multiple agents
//! - Resolve conflicts or convergence conditions
//! - Emit unified swarm outcomes
//! - Invoke downstream agents (spawn workers)
//! - Manage task dependencies (worker coordination)
//! - Coordinate parallel execution (fan-out)
//! - Transition workflow states (swarm lifecycle)
//!
//! ## What This Agent MUST NOT Do
//!
//! - Execute workflow steps directly (delegates to spawned agents)
//! - Modify task inputs or outputs produced by workers
//! - Change provider configurations
//! - Intercept runtime traffic directly (that is Edge)
//! - Enforce security policies (that is Shield)
//! - Emit anomaly detections (that is Sentinel)
//! - Perform optimization analysis (that is Auto-Optimizer)
//! - Connect directly to Google SQL
//! - Execute SQL statements
//! - Generate or evaluate model quality (that is Analytics/Observatory)
//!
//! ## CLI Endpoints
//!
//! - `swarm coordinate` - Coordinate a swarm of agents toward an objective
//! - `swarm inspect` - Inspect a previous coordination result
//! - `swarm replay` - Replay a previous coordination

use crate::adapters::observatory::ObservatoryAdapter;
use crate::adapters::ruvector_service::{DecisionEventRecord, RuVectorServiceClient};
use crate::error::{OrchestratorError, Result};
use agentics_contracts::swarm_coordinator::{
    AggregatedOutput, AggregationMode, AggregationStrategy, ConsensusData, ConsensusStrategy,
    CoordinationIssue, CoordinationIssueSeverity, CoordinationIssueType, CoordinationSummary,
    FieldReduction, ReductionOperation, SwarmCoordinationConstraintsApplied,
    SwarmCoordinationDecisionEvent, SwarmCoordinationDecisionOutputs,
    SwarmCoordinationExecutionRef, SwarmCoordinationRequest, SwarmCoordinationResult,
    SwarmCoordinationStatus, SwarmCoordinatorConfig, ValidationRule, WorkerError, WorkerMetrics,
    WorkerResult, WorkerSpec, WorkerStatus, SWARM_COORDINATOR_AGENT_ID,
    SWARM_COORDINATOR_AGENT_VERSION,
};
use agentics_contracts::{AgentId, AgentVersion};
use chrono::{DateTime, Utc};
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};
use uuid::Uuid;

use crate::feu_collector::FeuSpanCollector;
use agentics_contracts::feu::SpanStatus;

/// Swarm Coordinator Agent.
///
/// This agent coordinates fan-out / fan-in execution across multiple agents
/// working toward a shared objective.
///
/// # Classification
///
/// **MULTI-AGENT COORDINATION** (maps to TASK_COORDINATION)
///
/// # Non-Responsibilities
///
/// This agent MUST NEVER:
/// - Execute workflow steps directly (delegates to spawned agents)
/// - Modify task inputs or outputs produced by workers
/// - Intercept runtime traffic (that is Edge)
/// - Enforce security policies (that is Shield)
/// - Emit anomaly detections (that is Sentinel)
/// - Connect directly to Google SQL
/// - Execute SQL
pub struct SwarmCoordinatorAgent {
    /// Agent configuration.
    config: SwarmCoordinatorAgentConfig,
    /// RuVector service client for persistence.
    ruvector_client: RuVectorServiceClient,
    /// Observatory adapter for telemetry.
    observatory: ObservatoryAdapter,
    /// Agent ID.
    agent_id: String,
    /// FEU span collector (optional, for Agentics execution mode).
    feu_collector: Option<FeuSpanCollector>,
}

/// Configuration for the Swarm Coordinator Agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmCoordinatorAgentConfig {
    /// Default maximum concurrent workers.
    pub default_max_concurrent: u32,
    /// Default coordination timeout (ms).
    pub default_timeout_ms: u64,
    /// Default consensus strategy.
    pub default_consensus: ConsensusStrategy,
    /// Whether to enable worker result caching.
    pub enable_caching: bool,
}

impl Default for SwarmCoordinatorAgentConfig {
    fn default() -> Self {
        Self {
            default_max_concurrent: 8,
            default_timeout_ms: 300000, // 5 minutes
            default_consensus: ConsensusStrategy::WeightedMajority,
            enable_caching: true,
        }
    }
}

impl SwarmCoordinatorAgent {
    /// Creates a new Swarm Coordinator Agent.
    pub fn new(config: SwarmCoordinatorAgentConfig) -> Self {
        let agent_id = format!(
            "{}-{}",
            SWARM_COORDINATOR_AGENT_ID,
            &Uuid::new_v4().to_string()[..8]
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
        SWARM_COORDINATOR_AGENT_VERSION
    }

    // =========================================================================
    // Core Operations
    // =========================================================================

    /// Coordinates a swarm of agents toward a shared objective.
    ///
    /// This is the primary CLI endpoint for swarm coordination.
    /// Implements fan-out / fan-in pattern:
    /// 1. Fan-out: Spawn workers according to dependency order
    /// 2. Execute: Run workers in parallel where possible
    /// 3. Fan-in: Aggregate results using the configured strategy
    /// 4. Consensus: Resolve conflicts if needed
    pub async fn coordinate(
        &self,
        request: SwarmCoordinationRequest,
    ) -> Result<SwarmCoordinationResult> {
        let start_time = std::time::Instant::now();
        let coordination_start = Utc::now();

        // Start FEU agent span if in Agentics execution mode
        let feu_span_id = self.feu_collector.as_ref()
            .map(|c| c.start_agent_span("SwarmCoordinatorAgent"));

        tracing::info!(
            workflow_id = %request.workflow_id,
            objective_id = %request.objective.id,
            workers_count = request.workers.len(),
            "Starting swarm coordination"
        );

        // Calculate inputs hash for reproducibility
        let inputs_hash = self.hash_inputs(&request);

        // Validate request
        self.validate_request(&request)?;

        // Build worker dependency graph
        let (graph, node_map) = self.build_worker_graph(&request.workers)?;

        // Check for cycles
        let topo_order = match toposort(&graph, None) {
            Ok(order) => order,
            Err(cycle) => {
                let cycle_worker = node_map
                    .iter()
                    .find(|(_, &idx)| idx == cycle.node_id())
                    .map(|(id, _)| id.clone())
                    .unwrap_or_default();

                return self
                    .build_failed_result(
                        &request,
                        vec![CoordinationIssue {
                            severity: CoordinationIssueSeverity::Critical,
                            issue_type: CoordinationIssueType::ConfigurationError,
                            affected_workers: vec![cycle_worker],
                            description: "Circular dependency detected in worker graph".to_string(),
                            impact: "Cannot determine worker execution order".to_string(),
                            resolution: None,
                        }],
                        &inputs_hash,
                        start_time,
                    )
                    .await;
            }
        };

        // Create reverse node map
        let reverse_map: HashMap<NodeIndex, String> =
            node_map.iter().map(|(k, v)| (*v, k.clone())).collect();

        // Compute execution levels for parallel scheduling
        let execution_levels = self.compute_execution_levels(&graph, &topo_order, &reverse_map);

        // Execute workers (fan-out / fan-in)
        let (worker_results, issues) = self
            .execute_workers(
                &request,
                &execution_levels,
                &node_map,
                &reverse_map,
                coordination_start,
            )
            .await;

        // Aggregate results (fan-in)
        let aggregated_output =
            self.aggregate_results(&request, &worker_results, &request.aggregation);

        // Build coordination summary
        let summary = self.build_summary(&request, &worker_results);

        // Determine final status
        let status = self.determine_status(&request, &worker_results, &aggregated_output, &issues);

        let result = SwarmCoordinationResult {
            request_id: request.request_id,
            workflow_id: request.workflow_id,
            status: status.clone(),
            aggregated_output,
            worker_results,
            summary,
            issues,
            completed_at: Utc::now(),
        };

        // Calculate confidence
        let confidence = self.calculate_confidence(&request, &result);

        // Build and persist decision event
        let decision_event =
            self.build_decision_event(&request, &result, &inputs_hash, confidence, start_time);

        self.persist_decision_event(&decision_event).await?;

        // Emit telemetry
        self.emit_telemetry(&request, &result, start_time.elapsed())
            .await;

        tracing::info!(
            workflow_id = %request.workflow_id,
            status = ?status,
            workers = result.summary.total_workers,
            successful = result.summary.successful_workers,
            objective_met = result.summary.objective_met,
            confidence = confidence,
            "Swarm coordination complete"
        );

        // End FEU agent span if in Agentics execution mode
        if let (Some(collector), Some(span_id)) = (&self.feu_collector, &feu_span_id) {
            let feu_status = match &result.status {
                SwarmCoordinationStatus::Success | SwarmCoordinationStatus::ConsensusReached => SpanStatus::Ok,
                _ => SpanStatus::Failed,
            };
            collector.end_agent_span(&span_id, feu_status, Vec::new(), vec![
                serde_json::to_value(&decision_event).unwrap_or_default()
            ]);
        }

        Ok(result)
    }

    /// Inspects a previous coordination result.
    pub async fn inspect(&self, request_id: Uuid) -> Result<SwarmInspectResponse> {
        tracing::debug!(request_id = %request_id, "Inspecting swarm coordination");

        // Query decision events for this request from ruvector-service
        let events = self
            .ruvector_client
            .query_by_agent(&self.agent_id, Some(100))
            .await
            .map_err(|e| OrchestratorError::Internal(e.to_string()))?;

        // Find the relevant event
        let event = events
            .iter()
            .find(|e| {
                e.metadata.get("request_id").and_then(|v| v.as_str())
                    == Some(&request_id.to_string())
            })
            .cloned();

        let found = event.is_some();
        Ok(SwarmInspectResponse {
            request_id,
            decision_event: event,
            found,
        })
    }

    /// Replays a previous coordination with the same inputs.
    pub async fn replay(
        &self,
        request: SwarmCoordinationRequest,
    ) -> Result<SwarmCoordinationResult> {
        tracing::info!(
            request_id = %request.request_id,
            "Replaying swarm coordination"
        );

        // Simply re-run the coordination (deterministic)
        self.coordinate(request).await
    }

    // =========================================================================
    // Internal Implementation
    // =========================================================================

    /// Validates the coordination request.
    fn validate_request(&self, request: &SwarmCoordinationRequest) -> Result<()> {
        if request.workers.is_empty() {
            return Err(OrchestratorError::validation(
                "No workers provided for coordination",
            ));
        }

        // Check for duplicate worker IDs
        let mut seen = HashSet::new();
        for worker in &request.workers {
            if !seen.insert(&worker.worker_id) {
                return Err(OrchestratorError::validation(format!(
                    "Duplicate worker ID: {}",
                    worker.worker_id
                )));
            }
        }

        // Validate configuration
        if request.config.max_concurrent_workers == 0 {
            return Err(OrchestratorError::validation(
                "max_concurrent_workers must be > 0",
            ));
        }

        // Validate dependencies exist
        let worker_ids: HashSet<_> = request.workers.iter().map(|w| &w.worker_id).collect();
        for worker in &request.workers {
            for dep in &worker.depends_on {
                if !worker_ids.contains(dep) {
                    return Err(OrchestratorError::validation(format!(
                        "Worker '{}' depends on unknown worker '{}'",
                        worker.worker_id, dep
                    )));
                }
            }
        }

        Ok(())
    }

    /// Builds a directed graph from worker dependencies.
    fn build_worker_graph(
        &self,
        workers: &[WorkerSpec],
    ) -> Result<(DiGraph<String, ()>, HashMap<String, NodeIndex>)> {
        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();

        // Add all nodes
        for worker in workers {
            let idx = graph.add_node(worker.worker_id.clone());
            node_map.insert(worker.worker_id.clone(), idx);
        }

        // Add edges (dependency -> dependent)
        for worker in workers {
            let worker_idx = node_map[&worker.worker_id];
            for dep in &worker.depends_on {
                if let Some(&dep_idx) = node_map.get(dep) {
                    graph.add_edge(dep_idx, worker_idx, ());
                }
            }
        }

        Ok((graph, node_map))
    }

    /// Computes execution levels for parallel scheduling.
    fn compute_execution_levels(
        &self,
        graph: &DiGraph<String, ()>,
        topo_order: &[NodeIndex],
        reverse_map: &HashMap<NodeIndex, String>,
    ) -> Vec<Vec<String>> {
        // Calculate levels for each node
        let mut levels: HashMap<NodeIndex, u32> = HashMap::new();
        for &node in topo_order {
            let max_parent_level = graph
                .neighbors_directed(node, petgraph::Direction::Incoming)
                .filter_map(|parent| levels.get(&parent))
                .max()
                .copied()
                .unwrap_or(0);

            let level = if graph
                .neighbors_directed(node, petgraph::Direction::Incoming)
                .count()
                == 0
            {
                0
            } else {
                max_parent_level + 1
            };

            levels.insert(node, level);
        }

        // Group by level
        let max_level = levels.values().max().copied().unwrap_or(0);
        let mut execution_levels: Vec<Vec<String>> = vec![Vec::new(); (max_level + 1) as usize];

        for (node, &level) in &levels {
            if let Some(worker_id) = reverse_map.get(node) {
                execution_levels[level as usize].push(worker_id.clone());
            }
        }

        // Remove empty levels
        execution_levels.retain(|level| !level.is_empty());

        execution_levels
    }

    /// Executes workers according to execution levels (fan-out / fan-in).
    async fn execute_workers(
        &self,
        request: &SwarmCoordinationRequest,
        execution_levels: &[Vec<String>],
        node_map: &HashMap<String, NodeIndex>,
        _reverse_map: &HashMap<NodeIndex, String>,
        coordination_start: DateTime<Utc>,
    ) -> (Vec<WorkerResult>, Vec<CoordinationIssue>) {
        let worker_map: HashMap<String, WorkerSpec> = request
            .workers
            .iter()
            .map(|w| (w.worker_id.clone(), w.clone()))
            .collect();

        let results = Arc::new(Mutex::new(Vec::new()));
        let issues = Arc::new(Mutex::new(Vec::new()));

        // Execute each level (levels must be sequential, workers within a level can be parallel)
        for (level_idx, level_workers) in execution_levels.iter().enumerate() {
            tracing::debug!(
                level = level_idx,
                workers = ?level_workers,
                "Executing worker level"
            );

            // Apply concurrency limit
            let chunks: Vec<Vec<&String>> = level_workers
                .iter()
                .collect::<Vec<_>>()
                .chunks(request.config.max_concurrent_workers as usize)
                .map(|c| c.to_vec())
                .collect();

            for chunk in chunks {
                // Execute workers in this chunk in parallel
                let mut handles = Vec::new();

                for worker_id in chunk {
                    let worker_spec = worker_map.get(worker_id).cloned();
                    let results_clone = Arc::clone(&results);
                    let issues_clone = Arc::clone(&issues);
                    let timeout_ms = request.config.coordination_timeout_ms;
                    let continue_on_failure = request.config.continue_on_partial_failure;
                    let worker_id = worker_id.clone();

                    handles.push(tokio::spawn(async move {
                        if let Some(spec) = worker_spec {
                            let result = execute_single_worker(&spec, timeout_ms).await;

                            match &result.status {
                                WorkerStatus::Failed | WorkerStatus::Timeout => {
                                    let mut issues_guard = issues_clone.lock().await;
                                    issues_guard.push(CoordinationIssue {
                                        severity: if spec.config.critical {
                                            CoordinationIssueSeverity::Critical
                                        } else {
                                            CoordinationIssueSeverity::Warning
                                        },
                                        issue_type: if result.status == WorkerStatus::Timeout {
                                            CoordinationIssueType::WorkerTimeout
                                        } else {
                                            CoordinationIssueType::WorkerFailure
                                        },
                                        affected_workers: vec![worker_id.clone()],
                                        description: result
                                            .error
                                            .as_ref()
                                            .map(|e| e.message.clone())
                                            .unwrap_or_else(|| "Worker failed".to_string()),
                                        impact: if spec.config.critical {
                                            "Critical worker failure - swarm may fail".to_string()
                                        } else {
                                            "Non-critical worker failure".to_string()
                                        },
                                        resolution: if continue_on_failure {
                                            Some("Continuing with partial results".to_string())
                                        } else {
                                            None
                                        },
                                    });
                                }
                                _ => {}
                            }

                            let mut results_guard = results_clone.lock().await;
                            results_guard.push(result);
                        }
                    }));
                }

                // Wait for all workers in this chunk
                for handle in handles {
                    let _ = handle.await;
                }
            }

            // Check if we should abort due to critical failures
            if !request.config.continue_on_partial_failure {
                let issues_guard = issues.lock().await;
                if issues_guard
                    .iter()
                    .any(|i| i.severity == CoordinationIssueSeverity::Critical)
                {
                    tracing::warn!(level = level_idx, "Aborting swarm due to critical failure");
                    break;
                }
            }
        }

        let final_results = Arc::try_unwrap(results)
            .map_or_else(
                |arc| (*arc.blocking_lock()).clone(),
                |mutex| mutex.into_inner(),
            );

        let final_issues = Arc::try_unwrap(issues)
            .map_or_else(
                |arc| (*arc.blocking_lock()).clone(),
                |mutex| mutex.into_inner(),
            );

        (final_results, final_issues)
    }

    /// Aggregates results from workers (fan-in).
    fn aggregate_results(
        &self,
        request: &SwarmCoordinationRequest,
        worker_results: &[WorkerResult],
        strategy: &AggregationStrategy,
    ) -> AggregatedOutput {
        let successful_results: Vec<_> = worker_results
            .iter()
            .filter(|r| r.status == WorkerStatus::Success)
            .collect();

        if successful_results.is_empty() {
            return AggregatedOutput {
                primary: None,
                fields: HashMap::new(),
                consensus: None,
                confidence: 0.0,
            };
        }

        // Aggregate based on mode
        let primary = match strategy.mode {
            AggregationMode::Combine => {
                // Combine all outputs into an array
                let combined: Vec<_> = successful_results
                    .iter()
                    .filter_map(|r| r.output.clone())
                    .collect();
                Some(serde_json::json!(combined))
            }
            AggregationMode::SelectBest => {
                // Select result with highest confidence
                successful_results
                    .iter()
                    .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
                    .and_then(|r| r.output.clone())
            }
            AggregationMode::Collect => {
                // Keep all as a map
                let collected: HashMap<String, serde_json::Value> = successful_results
                    .iter()
                    .filter_map(|r| r.output.clone().map(|o| (r.worker_id.clone(), o)))
                    .collect();
                Some(serde_json::json!(collected))
            }
            AggregationMode::Summarize => {
                // Create summary object
                let summary = serde_json::json!({
                    "total_results": successful_results.len(),
                    "results": successful_results.iter()
                        .filter_map(|r| r.output.clone())
                        .collect::<Vec<_>>(),
                });
                Some(summary)
            }
            AggregationMode::Custom(_) => {
                // Custom aggregation - return combined for now
                let combined: Vec<_> = successful_results
                    .iter()
                    .filter_map(|r| r.output.clone())
                    .collect();
                Some(serde_json::json!(combined))
            }
        };

        // Apply field reductions
        let mut fields = HashMap::new();
        for reduction in &strategy.reduce_fields {
            let value = self.apply_reduction(&reduction, &successful_results);
            if let Some(v) = value {
                fields.insert(reduction.field.clone(), v);
            }
        }

        // Build consensus data
        let consensus = self.compute_consensus(request, worker_results);

        // Calculate aggregation confidence
        let confidence = if !successful_results.is_empty() {
            let avg_confidence: f64 = successful_results.iter().map(|r| r.confidence).sum::<f64>()
                / successful_results.len() as f64;

            let success_rate = successful_results.len() as f64 / worker_results.len() as f64;

            (avg_confidence * 0.6 + success_rate * 0.4).min(1.0)
        } else {
            0.0
        };

        AggregatedOutput {
            primary,
            fields,
            consensus,
            confidence,
        }
    }

    /// Applies a field reduction across worker results.
    fn apply_reduction(
        &self,
        reduction: &FieldReduction,
        results: &[&WorkerResult],
    ) -> Option<serde_json::Value> {
        let values: Vec<&serde_json::Value> = results
            .iter()
            .filter_map(|r| r.output.as_ref())
            .filter_map(|o| o.get(&reduction.field))
            .collect();

        if values.is_empty() {
            return None;
        }

        match &reduction.operation {
            ReductionOperation::Sum => {
                let sum: f64 = values.iter().filter_map(|v| v.as_f64()).sum();
                Some(serde_json::json!(sum))
            }
            ReductionOperation::Average => {
                let nums: Vec<f64> = values.iter().filter_map(|v| v.as_f64()).collect();
                if nums.is_empty() {
                    None
                } else {
                    let avg = nums.iter().sum::<f64>() / nums.len() as f64;
                    Some(serde_json::json!(avg))
                }
            }
            ReductionOperation::Min => values
                .iter()
                .filter_map(|v| v.as_f64())
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .map(|v| serde_json::json!(v)),
            ReductionOperation::Max => values
                .iter()
                .filter_map(|v| v.as_f64())
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .map(|v| serde_json::json!(v)),
            ReductionOperation::Concat => {
                let strings: Vec<&str> = values.iter().filter_map(|v| v.as_str()).collect();
                Some(serde_json::json!(strings.join("")))
            }
            ReductionOperation::Union => {
                let mut union: HashSet<String> = HashSet::new();
                for v in &values {
                    if let Some(arr) = v.as_array() {
                        for item in arr {
                            if let Some(s) = item.as_str() {
                                union.insert(s.to_string());
                            }
                        }
                    }
                }
                Some(serde_json::json!(union.into_iter().collect::<Vec<_>>()))
            }
            ReductionOperation::Intersect => {
                let mut sets: Vec<HashSet<String>> = Vec::new();
                for v in &values {
                    if let Some(arr) = v.as_array() {
                        let set: HashSet<String> = arr
                            .iter()
                            .filter_map(|item: &serde_json::Value| item.as_str().map(String::from))
                            .collect();
                        sets.push(set);
                    }
                }
                if sets.is_empty() {
                    None
                } else {
                    let mut intersection = sets[0].clone();
                    for set in sets.iter().skip(1) {
                        intersection = intersection.intersection(set).cloned().collect();
                    }
                    Some(serde_json::json!(intersection
                        .into_iter()
                        .collect::<Vec<_>>()))
                }
            }
        }
    }

    /// Computes consensus from worker results.
    fn compute_consensus(
        &self,
        request: &SwarmCoordinationRequest,
        worker_results: &[WorkerResult],
    ) -> Option<ConsensusData> {
        let successful_results: Vec<_> = worker_results
            .iter()
            .filter(|r| r.status == WorkerStatus::Success)
            .collect();

        if successful_results.len() < 2 {
            return None;
        }

        let total = worker_results.len() as u32;
        let agreeing = successful_results.len() as u32;

        let convergence_score = agreeing as f64 / total as f64;
        let reached = convergence_score >= request.config.convergence_threshold;

        let dissenting: Vec<String> = worker_results
            .iter()
            .filter(|r| r.status != WorkerStatus::Success)
            .map(|r| r.worker_id.clone())
            .collect();

        Some(ConsensusData {
            strategy: request.config.consensus_strategy.clone(),
            reached,
            convergence_score,
            agreeing_workers: agreeing,
            total_workers: total,
            dissenting_workers: dissenting,
        })
    }

    /// Builds the coordination summary.
    fn build_summary(
        &self,
        request: &SwarmCoordinationRequest,
        worker_results: &[WorkerResult],
    ) -> CoordinationSummary {
        let total = worker_results.len() as u32;
        let successful = worker_results
            .iter()
            .filter(|r| r.status == WorkerStatus::Success)
            .count() as u32;
        let failed = worker_results
            .iter()
            .filter(|r| r.status == WorkerStatus::Failed || r.status == WorkerStatus::Timeout)
            .count() as u32;
        let skipped = worker_results
            .iter()
            .filter(|r| r.status == WorkerStatus::Skipped)
            .count() as u32;

        let total_duration: u64 = worker_results.iter().map(|r| r.metrics.duration_ms).sum();

        // Check success criteria
        let mut criteria_met = Vec::new();
        let mut criteria_not_met = Vec::new();

        for criterion in &request.objective.success_criteria {
            let met = match &criterion.validation {
                ValidationRule::AllSuccess => successful == total,
                ValidationRule::MinSuccess { count } => successful >= *count,
                ValidationRule::Majority => successful > total / 2,
                ValidationRule::AnySuccess => successful >= 1,
                ValidationRule::Custom(_) => true, // Assume met for custom
            };

            if met {
                criteria_met.push(criterion.id.clone());
            } else {
                criteria_not_met.push(criterion.id.clone());
            }
        }

        let objective_met = criteria_not_met.is_empty()
            || criteria_not_met.iter().all(|c_id| {
                request
                    .objective
                    .success_criteria
                    .iter()
                    .find(|c| &c.id == c_id)
                    .map(|c| !c.required)
                    .unwrap_or(true)
            });

        CoordinationSummary {
            total_workers: total,
            successful_workers: successful,
            failed_workers: failed,
            skipped_workers: skipped,
            max_concurrency_achieved: request.config.max_concurrent_workers.min(total),
            total_duration_ms: total_duration,
            objective_met,
            criteria_met,
            criteria_not_met,
        }
    }

    /// Determines the final coordination status.
    fn determine_status(
        &self,
        request: &SwarmCoordinationRequest,
        worker_results: &[WorkerResult],
        aggregated_output: &AggregatedOutput,
        issues: &[CoordinationIssue],
    ) -> SwarmCoordinationStatus {
        // Check for critical issues
        if issues
            .iter()
            .any(|i| i.severity == CoordinationIssueSeverity::Critical)
        {
            return SwarmCoordinationStatus::Failed;
        }

        // Check for timeout
        if worker_results
            .iter()
            .any(|r| r.status == WorkerStatus::Timeout)
            && !request.config.continue_on_partial_failure
        {
            return SwarmCoordinationStatus::Timeout;
        }

        // Count successes
        let successful = worker_results
            .iter()
            .filter(|r| r.status == WorkerStatus::Success)
            .count();

        let total = worker_results.len();

        if successful == total {
            // Check consensus
            if let Some(ref consensus) = aggregated_output.consensus {
                if consensus.reached {
                    return SwarmCoordinationStatus::ConsensusReached;
                }
            }
            SwarmCoordinationStatus::Success
        } else if successful > 0 {
            SwarmCoordinationStatus::PartialSuccess
        } else {
            SwarmCoordinationStatus::Failed
        }
    }

    /// Calculates confidence score for the coordination.
    fn calculate_confidence(
        &self,
        request: &SwarmCoordinationRequest,
        result: &SwarmCoordinationResult,
    ) -> f64 {
        let mut confidence: f64 = 0.0;

        // Success rate (0.3 weight)
        let success_rate =
            result.summary.successful_workers as f64 / result.summary.total_workers.max(1) as f64;
        confidence += success_rate * 0.3;

        // Objective met (0.3 weight)
        let objective_score = if result.summary.objective_met {
            1.0
        } else {
            0.3
        };
        confidence += objective_score * 0.3;

        // Consensus convergence (0.2 weight)
        let consensus_score = result
            .aggregated_output
            .consensus
            .as_ref()
            .map(|c| c.convergence_score)
            .unwrap_or(success_rate);
        confidence += consensus_score * 0.2;

        // No issues (0.2 weight)
        let issue_score = if result.issues.is_empty() {
            1.0
        } else if result
            .issues
            .iter()
            .any(|i| i.severity == CoordinationIssueSeverity::Critical)
        {
            0.0
        } else {
            0.5
        };
        confidence += issue_score * 0.2;

        confidence.clamp(0.0, 1.0)
    }

    /// Builds the decision event.
    fn build_decision_event(
        &self,
        request: &SwarmCoordinationRequest,
        result: &SwarmCoordinationResult,
        inputs_hash: &str,
        confidence: f64,
        start_time: std::time::Instant,
    ) -> SwarmCoordinationDecisionEvent {
        SwarmCoordinationDecisionEvent {
            id: Uuid::new_v4(),
            agent_id: AgentId::from_string(&self.agent_id),
            agent_version: AgentVersion::parse(SWARM_COORDINATOR_AGENT_VERSION).unwrap(),
            decision_type: "swarm_coordination".to_string(),
            inputs_hash: inputs_hash.to_string(),
            outputs: SwarmCoordinationDecisionOutputs {
                status: result.status.clone(),
                total_workers: result.summary.total_workers,
                successful_workers: result.summary.successful_workers,
                failed_workers: result.summary.failed_workers,
                consensus_reached: result
                    .aggregated_output
                    .consensus
                    .as_ref()
                    .map(|c| c.reached)
                    .unwrap_or(false),
                convergence_score: result
                    .aggregated_output
                    .consensus
                    .as_ref()
                    .map(|c| c.convergence_score)
                    .unwrap_or(0.0),
                objective_met: result.summary.objective_met,
                total_duration_ms: start_time.elapsed().as_millis() as u64,
            },
            confidence,
            constraints_applied: SwarmCoordinationConstraintsApplied {
                max_concurrent_workers: request.config.max_concurrent_workers,
                coordination_timeout_ms: request.config.coordination_timeout_ms,
                consensus_strategy: request.config.consensus_strategy.clone(),
                continue_on_partial_failure: request.config.continue_on_partial_failure,
                convergence_threshold: request.config.convergence_threshold,
            },
            execution_ref: SwarmCoordinationExecutionRef::new(
                request.workflow_id,
                request.request_id,
            ),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Persists the decision event to ruvector-service.
    async fn persist_decision_event(&self, event: &SwarmCoordinationDecisionEvent) -> Result<()> {
        // Convert to generic DecisionEventRecord for ruvector
        let record = DecisionEventRecord {
            id: event.id,
            agent_id: self.agent_id.clone(),
            agent_version: SWARM_COORDINATOR_AGENT_VERSION.to_string(),
            decision_type: event.decision_type.clone(),
            inputs_hash: event.inputs_hash.clone(),
            outputs: serde_json::to_value(&event.outputs).unwrap_or_default(),
            confidence: event.confidence,
            constraints_applied: serde_json::to_value(&event.constraints_applied)
                .unwrap_or_default(),
            workflow_execution_id: event.execution_ref.workflow_id,
            step_execution_id: None,
            trace_id: event.execution_ref.trace_id.clone(),
            span_id: event.execution_ref.span_id.clone(),
            timestamp: event.timestamp,
            metadata: {
                let mut m = event.metadata.clone();
                m.insert(
                    "request_id".to_string(),
                    serde_json::json!(event.execution_ref.request_id.to_string()),
                );
                m
            },
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
        request: &SwarmCoordinationRequest,
        issues: Vec<CoordinationIssue>,
        inputs_hash: &str,
        start_time: std::time::Instant,
    ) -> Result<SwarmCoordinationResult> {
        let result = SwarmCoordinationResult {
            request_id: request.request_id,
            workflow_id: request.workflow_id,
            status: SwarmCoordinationStatus::Failed,
            aggregated_output: AggregatedOutput::default(),
            worker_results: Vec::new(),
            summary: CoordinationSummary {
                total_workers: request.workers.len() as u32,
                successful_workers: 0,
                failed_workers: request.workers.len() as u32,
                skipped_workers: 0,
                max_concurrency_achieved: 0,
                total_duration_ms: 0,
                objective_met: false,
                criteria_met: Vec::new(),
                criteria_not_met: request
                    .objective
                    .success_criteria
                    .iter()
                    .map(|c| c.id.clone())
                    .collect(),
            },
            issues,
            completed_at: Utc::now(),
        };

        // Persist failed decision event
        let decision_event =
            self.build_decision_event(request, &result, inputs_hash, 0.0, start_time);
        self.persist_decision_event(&decision_event).await?;

        Ok(result)
    }

    /// Emits telemetry for the coordination.
    async fn emit_telemetry(
        &self,
        request: &SwarmCoordinationRequest,
        result: &SwarmCoordinationResult,
        duration: std::time::Duration,
    ) {
        tracing::info!(
            workflow_id = %request.workflow_id,
            objective_id = %request.objective.id,
            status = ?result.status,
            total_workers = result.summary.total_workers,
            successful_workers = result.summary.successful_workers,
            objective_met = result.summary.objective_met,
            duration_ms = duration.as_millis() as u64,
            "swarm.coordination.complete"
        );
    }

    /// Hashes the request inputs.
    fn hash_inputs(&self, request: &SwarmCoordinationRequest) -> String {
        let json = serde_json::to_string(request).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Executes a single worker (simulated for now).
///
/// In production, this would spawn an actual agent process.
async fn execute_single_worker(spec: &WorkerSpec, timeout_ms: u64) -> WorkerResult {
    let start = Utc::now();
    let start_time = std::time::Instant::now();

    // Simulate worker execution with timeout
    let worker_timeout = spec.task.timeout_ms.unwrap_or(timeout_ms).min(timeout_ms);

    let result = timeout(
        Duration::from_millis(worker_timeout),
        simulate_worker_execution(spec),
    )
    .await;

    let duration = start_time.elapsed().as_millis() as u64;

    match result {
        Ok(output) => WorkerResult {
            worker_id: spec.worker_id.clone(),
            agent_type: spec.agent_type.clone(),
            status: WorkerStatus::Success,
            output: Some(output),
            confidence: 0.9, // Simulated confidence
            error: None,
            metrics: WorkerMetrics {
                started_at: Some(start),
                duration_ms: duration,
                retries: 0,
                tokens_used: None,
            },
            completed_at: Utc::now(),
        },
        Err(_) => WorkerResult {
            worker_id: spec.worker_id.clone(),
            agent_type: spec.agent_type.clone(),
            status: WorkerStatus::Timeout,
            output: None,
            confidence: 0.0,
            error: Some(WorkerError {
                code: "TIMEOUT".to_string(),
                message: format!("Worker timed out after {}ms", worker_timeout),
                retryable: true,
                attempt: 0,
            }),
            metrics: WorkerMetrics {
                started_at: Some(start),
                duration_ms: duration,
                retries: 0,
                tokens_used: None,
            },
            completed_at: Utc::now(),
        },
    }
}

/// Simulates worker execution (placeholder for actual agent invocation).
async fn simulate_worker_execution(spec: &WorkerSpec) -> serde_json::Value {
    // Simulate some processing time
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Return simulated output based on task
    serde_json::json!({
        "worker_id": spec.worker_id,
        "task_id": spec.task.task_id,
        "completed": true,
        "result": "Simulated worker result"
    })
}

// =============================================================================
// Response Types
// =============================================================================

/// Response for inspect operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmInspectResponse {
    /// Request ID inspected.
    pub request_id: Uuid,
    /// Decision event if found.
    pub decision_event: Option<DecisionEventRecord>,
    /// Whether the coordination was found.
    pub found: bool,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use agentics_contracts::swarm_coordinator::{
        ObjectiveType, SwarmObjective, WorkerAgentType, WorkerTask,
    };

    fn sample_objective() -> SwarmObjective {
        SwarmObjective::new(
            "test-objective",
            "Test research objective",
            ObjectiveType::Research,
        )
    }

    fn sample_workers() -> Vec<WorkerSpec> {
        vec![
            WorkerSpec::new(
                "worker-a",
                WorkerAgentType::Custom("analyzer".to_string()),
                WorkerTask::new("task-a", "Analyze module A"),
            ),
            WorkerSpec::new(
                "worker-b",
                WorkerAgentType::Custom("analyzer".to_string()),
                WorkerTask::new("task-b", "Analyze module B"),
            ),
            WorkerSpec::new(
                "worker-c",
                WorkerAgentType::Custom("aggregator".to_string()),
                WorkerTask::new("task-c", "Aggregate results"),
            )
            .depends_on("worker-a")
            .depends_on("worker-b"),
        ]
    }

    #[test]
    fn test_agent_creation() {
        let agent = SwarmCoordinatorAgent::new(SwarmCoordinatorAgentConfig::default());
        assert!(agent.agent_id().starts_with(SWARM_COORDINATOR_AGENT_ID));
        assert_eq!(agent.version(), SWARM_COORDINATOR_AGENT_VERSION);
    }

    #[tokio::test]
    async fn test_coordinate_simple_swarm() {
        let agent = SwarmCoordinatorAgent::new(SwarmCoordinatorAgentConfig::default());
        let objective = sample_objective();
        let workers = sample_workers();

        let request = SwarmCoordinationRequest::new(Uuid::new_v4(), objective, workers);
        let result = agent.coordinate(request).await.unwrap();

        assert!(matches!(
            result.status,
            SwarmCoordinationStatus::Success
                | SwarmCoordinationStatus::ConsensusReached
                | SwarmCoordinationStatus::PartialSuccess
        ));
        assert_eq!(result.summary.total_workers, 3);
    }

    #[tokio::test]
    async fn test_coordinate_parallel_workers() {
        let agent = SwarmCoordinatorAgent::new(SwarmCoordinatorAgentConfig::default());
        let objective = sample_objective();

        // Workers with no dependencies (all can run in parallel)
        let workers = vec![
            WorkerSpec::new(
                "worker-1",
                WorkerAgentType::Custom("analyzer".to_string()),
                WorkerTask::new("task-1", "Task 1"),
            ),
            WorkerSpec::new(
                "worker-2",
                WorkerAgentType::Custom("analyzer".to_string()),
                WorkerTask::new("task-2", "Task 2"),
            ),
            WorkerSpec::new(
                "worker-3",
                WorkerAgentType::Custom("analyzer".to_string()),
                WorkerTask::new("task-3", "Task 3"),
            ),
        ];

        let request = SwarmCoordinationRequest::new(Uuid::new_v4(), objective, workers);
        let result = agent.coordinate(request).await.unwrap();

        assert_eq!(result.summary.total_workers, 3);
        assert!(result.summary.successful_workers >= 3);
    }

    #[test]
    fn test_validate_request_duplicate_workers() {
        let agent = SwarmCoordinatorAgent::new(SwarmCoordinatorAgentConfig::default());
        let objective = sample_objective();

        let workers = vec![
            WorkerSpec::new(
                "worker-a",
                WorkerAgentType::Custom("analyzer".to_string()),
                WorkerTask::new("task-a", "Task A"),
            ),
            WorkerSpec::new(
                "worker-a", // Duplicate ID
                WorkerAgentType::Custom("analyzer".to_string()),
                WorkerTask::new("task-b", "Task B"),
            ),
        ];

        let request = SwarmCoordinationRequest::new(Uuid::new_v4(), objective, workers);
        let result = agent.validate_request(&request);

        assert!(result.is_err());
    }

    #[test]
    fn test_validate_request_missing_dependency() {
        let agent = SwarmCoordinatorAgent::new(SwarmCoordinatorAgentConfig::default());
        let objective = sample_objective();

        let workers = vec![WorkerSpec::new(
            "worker-a",
            WorkerAgentType::Custom("analyzer".to_string()),
            WorkerTask::new("task-a", "Task A"),
        )
        .depends_on("nonexistent")];

        let request = SwarmCoordinationRequest::new(Uuid::new_v4(), objective, workers);
        let result = agent.validate_request(&request);

        assert!(result.is_err());
    }

    #[test]
    fn test_config_defaults() {
        let config = SwarmCoordinatorAgentConfig::default();
        assert_eq!(config.default_max_concurrent, 8);
        assert_eq!(config.default_timeout_ms, 300000);
        assert!(config.enable_caching);
    }

    #[test]
    fn test_hash_inputs_deterministic() {
        let agent = SwarmCoordinatorAgent::new(SwarmCoordinatorAgentConfig::default());
        let objective = sample_objective();
        let workers = sample_workers();

        let request = SwarmCoordinationRequest::new(Uuid::new_v4(), objective, workers);

        let hash1 = agent.hash_inputs(&request);
        let hash2 = agent.hash_inputs(&request);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 = 64 hex chars
    }

    #[test]
    fn test_build_worker_graph() {
        let agent = SwarmCoordinatorAgent::new(SwarmCoordinatorAgentConfig::default());
        let workers = sample_workers();

        let (graph, node_map) = agent.build_worker_graph(&workers).unwrap();

        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 2); // worker-c depends on worker-a and worker-b
        assert!(node_map.contains_key("worker-a"));
        assert!(node_map.contains_key("worker-b"));
        assert!(node_map.contains_key("worker-c"));
    }

    #[test]
    fn test_compute_execution_levels() {
        let agent = SwarmCoordinatorAgent::new(SwarmCoordinatorAgentConfig::default());
        let workers = sample_workers();

        let (graph, node_map) = agent.build_worker_graph(&workers).unwrap();
        let topo_order = toposort(&graph, None).unwrap();
        let reverse_map: HashMap<NodeIndex, String> =
            node_map.iter().map(|(k, v)| (*v, k.clone())).collect();

        let levels = agent.compute_execution_levels(&graph, &topo_order, &reverse_map);

        assert_eq!(levels.len(), 2); // Level 0: worker-a, worker-b; Level 1: worker-c
        assert!(levels[0].contains(&"worker-a".to_string()));
        assert!(levels[0].contains(&"worker-b".to_string()));
        assert!(levels[1].contains(&"worker-c".to_string()));
    }
}
