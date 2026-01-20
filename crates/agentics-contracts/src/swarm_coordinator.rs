// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Swarm Coordinator Agent schemas for multi-agent coordination.
//!
//! This module defines the contract schemas for the Swarm Coordinator Agent
//! within the Agent Infrastructure Constitution. The Swarm Coordinator Agent
//! is classified under MULTI-AGENT COORDINATION and is responsible for
//! coordinating fan-out / fan-in execution across multiple agents working
//! toward a shared objective.
//!
//! # Agent Contract
//!
//! **Agent Name**: Swarm Coordinator Agent
//!
//! **Classification**: MULTI-AGENT COORDINATION (maps to TASK_COORDINATION)
//!
//! **Purpose**: Coordinate fan-out / fan-in execution across multiple agents
//! working toward a shared objective.
//!
//! **Scope**:
//! - Spawn and manage parallel agent executions
//! - Aggregate results from multiple agents
//! - Resolve conflicts or convergence conditions
//! - Emit unified swarm outcomes
//!
//! **decision_type**: `"swarm_coordination"`
//!
//! # Non-Responsibilities (MUST NEVER)
//!
//! - Intercept runtime traffic directly (that is Edge)
//! - Enforce security policies (that is Shield)
//! - Emit anomaly detections (that is Sentinel)
//! - Perform optimization analysis (that is Auto-Optimizer)
//! - Execute workflow steps directly (delegates to spawned agents)
//! - Connect directly to Google SQL
//! - Execute SQL

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::agent::{AgentId, AgentVersion};

/// Swarm Coordinator Agent ID constant.
pub const SWARM_COORDINATOR_AGENT_ID: &str = "swarm-coordinator-agent";

/// Swarm Coordinator Agent Version.
pub const SWARM_COORDINATOR_AGENT_VERSION: &str = "0.1.0";

// =============================================================================
// Input Schema: SwarmCoordinationRequest
// =============================================================================

/// Request for swarm coordination.
///
/// Input contract for the Swarm Coordinator Agent's coordinate operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmCoordinationRequest {
    /// Unique request ID.
    pub request_id: Uuid,

    /// Workflow ID associated with this coordination.
    pub workflow_id: Uuid,

    /// Shared objective for the swarm.
    pub objective: SwarmObjective,

    /// Worker specifications to spawn.
    pub workers: Vec<WorkerSpec>,

    /// Coordination configuration.
    #[serde(default)]
    pub config: SwarmCoordinatorConfig,

    /// Aggregation strategy for results.
    #[serde(default)]
    pub aggregation: AggregationStrategy,

    /// Request timestamp.
    pub timestamp: DateTime<Utc>,
}

/// Shared objective for the swarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmObjective {
    /// Objective identifier.
    pub id: String,

    /// Human-readable description.
    pub description: String,

    /// Objective type.
    pub objective_type: ObjectiveType,

    /// Input data for the objective.
    #[serde(default)]
    pub inputs: HashMap<String, serde_json::Value>,

    /// Success criteria.
    #[serde(default)]
    pub success_criteria: Vec<SuccessCriterion>,

    /// Priority level (0 = lowest, 10 = highest).
    #[serde(default)]
    pub priority: u8,
}

/// Type of swarm objective.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveType {
    /// Research and gather information.
    Research,
    /// Analyze data or code.
    Analysis,
    /// Generate or transform content.
    Generation,
    /// Execute a set of tasks.
    Execution,
    /// Review and validate work.
    Review,
    /// Custom objective type.
    Custom(String),
}

/// Success criterion for an objective.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessCriterion {
    /// Criterion identifier.
    pub id: String,

    /// Description of the criterion.
    pub description: String,

    /// Whether this criterion is required.
    #[serde(default = "default_true")]
    pub required: bool,

    /// Validation type.
    pub validation: ValidationRule,
}

/// Validation rule for success criteria.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationRule {
    /// All workers must complete successfully.
    AllSuccess,
    /// At least N workers must succeed.
    MinSuccess { count: u32 },
    /// Majority of workers must succeed.
    Majority,
    /// Any one worker success is sufficient.
    AnySuccess,
    /// Custom validation function.
    Custom(String),
}

/// Specification for a worker to spawn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerSpec {
    /// Unique worker ID within the swarm.
    pub worker_id: String,

    /// Agent type to spawn.
    pub agent_type: WorkerAgentType,

    /// Task for this worker.
    pub task: WorkerTask,

    /// Dependencies on other workers (must complete before this starts).
    #[serde(default)]
    pub depends_on: Vec<String>,

    /// Worker-specific configuration.
    #[serde(default)]
    pub config: WorkerConfig,
}

/// Type of agent to spawn as a worker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerAgentType {
    /// Workflow orchestrator agent.
    WorkflowOrchestrator,
    /// Dependency resolver agent.
    DependencyResolver,
    /// Parallelization agent.
    Parallelization,
    /// Custom agent type.
    Custom(String),
}

/// Task for a worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerTask {
    /// Task identifier.
    pub task_id: String,

    /// Task description.
    pub description: String,

    /// Input data for the task.
    #[serde(default)]
    pub inputs: HashMap<String, serde_json::Value>,

    /// Expected output keys.
    #[serde(default)]
    pub expected_outputs: Vec<String>,

    /// Timeout for this task (milliseconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// Worker-specific configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkerConfig {
    /// Maximum retries for this worker.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Retry delay (milliseconds).
    #[serde(default = "default_retry_delay")]
    pub retry_delay_ms: u64,

    /// Whether this worker is critical (failure fails the swarm).
    #[serde(default)]
    pub critical: bool,

    /// Resource limits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_limits: Option<WorkerResourceLimits>,
}

/// Resource limits for a worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResourceLimits {
    /// Memory limit in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_bytes: Option<u64>,

    /// CPU limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<f64>,

    /// Token limit for LLM calls.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<u64>,
}

fn default_true() -> bool {
    true
}

fn default_max_retries() -> u32 {
    3
}

fn default_retry_delay() -> u64 {
    1000
}

/// Configuration for the Swarm Coordinator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmCoordinatorConfig {
    /// Maximum number of concurrent workers.
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_workers: u32,

    /// Overall coordination timeout (milliseconds).
    #[serde(default = "default_coordination_timeout")]
    pub coordination_timeout_ms: u64,

    /// Consensus strategy for conflict resolution.
    #[serde(default)]
    pub consensus_strategy: ConsensusStrategy,

    /// Whether to continue on partial failure.
    #[serde(default)]
    pub continue_on_partial_failure: bool,

    /// Convergence threshold (0.0 to 1.0) for consensus.
    #[serde(default = "default_convergence_threshold")]
    pub convergence_threshold: f64,

    /// Whether to enable result caching.
    #[serde(default = "default_true")]
    pub enable_caching: bool,
}

fn default_max_concurrent() -> u32 {
    8
}

fn default_coordination_timeout() -> u64 {
    300000 // 5 minutes
}

fn default_convergence_threshold() -> f64 {
    0.8
}

impl Default for SwarmCoordinatorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_workers: 8,
            coordination_timeout_ms: 300000,
            consensus_strategy: ConsensusStrategy::default(),
            continue_on_partial_failure: false,
            convergence_threshold: 0.8,
            enable_caching: true,
        }
    }
}

/// Consensus strategy for conflict resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsensusStrategy {
    /// First worker result wins.
    FirstWins,
    /// Last worker result wins.
    LastWins,
    /// Majority consensus (requires quorum).
    Majority,
    /// Weighted majority based on confidence scores.
    WeightedMajority,
    /// Merge all results (no conflict resolution).
    MergeAll,
    /// Use a designated leader worker.
    Leader { leader_id: String },
    /// Custom consensus function.
    Custom(String),
}

impl Default for ConsensusStrategy {
    fn default() -> Self {
        Self::WeightedMajority
    }
}

/// Aggregation strategy for worker results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationStrategy {
    /// Aggregation mode.
    #[serde(default)]
    pub mode: AggregationMode,

    /// Fields to aggregate.
    #[serde(default)]
    pub aggregate_fields: Vec<String>,

    /// Fields to merge (combine all values).
    #[serde(default)]
    pub merge_fields: Vec<String>,

    /// Fields to reduce (apply reduction function).
    #[serde(default)]
    pub reduce_fields: Vec<FieldReduction>,
}

impl Default for AggregationStrategy {
    fn default() -> Self {
        Self {
            mode: AggregationMode::Combine,
            aggregate_fields: Vec::new(),
            merge_fields: Vec::new(),
            reduce_fields: Vec::new(),
        }
    }
}

/// Aggregation mode for results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregationMode {
    /// Combine all results into a single output.
    Combine,
    /// Select best result based on scoring.
    SelectBest,
    /// Create a summary from all results.
    Summarize,
    /// Keep all results as a collection.
    Collect,
    /// Custom aggregation logic.
    Custom(String),
}

impl Default for AggregationMode {
    fn default() -> Self {
        Self::Combine
    }
}

/// Field reduction specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldReduction {
    /// Field name.
    pub field: String,

    /// Reduction operation.
    pub operation: ReductionOperation,
}

/// Reduction operation for a field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReductionOperation {
    /// Sum numeric values.
    Sum,
    /// Average numeric values.
    Average,
    /// Take minimum value.
    Min,
    /// Take maximum value.
    Max,
    /// Concatenate strings.
    Concat,
    /// Union of arrays.
    Union,
    /// Intersection of arrays.
    Intersect,
}

// =============================================================================
// Output Schema: SwarmCoordinationResult
// =============================================================================

/// Result of swarm coordination.
///
/// Output contract for the Swarm Coordinator Agent's coordinate operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmCoordinationResult {
    /// Request ID this result corresponds to.
    pub request_id: Uuid,

    /// Workflow ID.
    pub workflow_id: Uuid,

    /// Coordination status.
    pub status: SwarmCoordinationStatus,

    /// Aggregated output from all workers.
    pub aggregated_output: AggregatedOutput,

    /// Individual worker results.
    pub worker_results: Vec<WorkerResult>,

    /// Coordination summary.
    pub summary: CoordinationSummary,

    /// Issues encountered during coordination.
    pub issues: Vec<CoordinationIssue>,

    /// Coordination completion timestamp.
    pub completed_at: DateTime<Utc>,
}

/// Status of swarm coordination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwarmCoordinationStatus {
    /// All workers completed successfully.
    Success,
    /// Partial success (some workers failed but objective met).
    PartialSuccess,
    /// Objective achieved with consensus.
    ConsensusReached,
    /// Coordination failed.
    Failed,
    /// Coordination timed out.
    Timeout,
    /// Coordination cancelled.
    Cancelled,
}

/// Aggregated output from all workers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AggregatedOutput {
    /// Primary aggregated result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<serde_json::Value>,

    /// Aggregated fields by name.
    #[serde(default)]
    pub fields: HashMap<String, serde_json::Value>,

    /// Consensus data if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consensus: Option<ConsensusData>,

    /// Confidence score for the aggregated result (0.0 to 1.0).
    pub confidence: f64,
}

/// Consensus data from worker results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusData {
    /// Consensus strategy used.
    pub strategy: ConsensusStrategy,

    /// Whether consensus was reached.
    pub reached: bool,

    /// Convergence score (0.0 to 1.0).
    pub convergence_score: f64,

    /// Number of agreeing workers.
    pub agreeing_workers: u32,

    /// Total workers participating.
    pub total_workers: u32,

    /// Dissenting worker IDs.
    #[serde(default)]
    pub dissenting_workers: Vec<String>,
}

/// Result from an individual worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResult {
    /// Worker ID.
    pub worker_id: String,

    /// Agent type used.
    pub agent_type: WorkerAgentType,

    /// Worker status.
    pub status: WorkerStatus,

    /// Output data from the worker.
    pub output: Option<serde_json::Value>,

    /// Confidence score for this result.
    pub confidence: f64,

    /// Error details if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<WorkerError>,

    /// Execution metrics.
    pub metrics: WorkerMetrics,

    /// Completion timestamp.
    pub completed_at: DateTime<Utc>,
}

/// Status of an individual worker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerStatus {
    /// Worker not yet started.
    Pending,
    /// Worker is running.
    Running,
    /// Worker completed successfully.
    Success,
    /// Worker failed.
    Failed,
    /// Worker timed out.
    Timeout,
    /// Worker was skipped.
    Skipped,
    /// Worker was cancelled.
    Cancelled,
}

/// Error details for a worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerError {
    /// Error code.
    pub code: String,

    /// Error message.
    pub message: String,

    /// Whether the error is retryable.
    pub retryable: bool,

    /// Retry attempt number (if retried).
    #[serde(default)]
    pub attempt: u32,
}

/// Execution metrics for a worker.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkerMetrics {
    /// Start timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,

    /// Duration in milliseconds.
    pub duration_ms: u64,

    /// Number of retries.
    pub retries: u32,

    /// Token usage (if LLM-based).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<u64>,
}

/// Coordination summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationSummary {
    /// Total workers spawned.
    pub total_workers: u32,

    /// Successfully completed workers.
    pub successful_workers: u32,

    /// Failed workers.
    pub failed_workers: u32,

    /// Skipped workers.
    pub skipped_workers: u32,

    /// Maximum concurrency achieved.
    pub max_concurrency_achieved: u32,

    /// Total coordination duration (milliseconds).
    pub total_duration_ms: u64,

    /// Whether the objective was met.
    pub objective_met: bool,

    /// Success criteria met.
    pub criteria_met: Vec<String>,

    /// Success criteria not met.
    pub criteria_not_met: Vec<String>,
}

/// Issue encountered during coordination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationIssue {
    /// Issue severity.
    pub severity: CoordinationIssueSeverity,

    /// Issue type.
    pub issue_type: CoordinationIssueType,

    /// Affected workers.
    pub affected_workers: Vec<String>,

    /// Description.
    pub description: String,

    /// Impact on coordination.
    pub impact: String,

    /// Resolution taken.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
}

/// Severity of a coordination issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinationIssueSeverity {
    /// Informational.
    Info,
    /// Warning (non-blocking).
    Warning,
    /// Error (may affect outcome).
    Error,
    /// Critical (coordination failed).
    Critical,
}

/// Type of coordination issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinationIssueType {
    /// Worker failed.
    WorkerFailure,
    /// Worker timed out.
    WorkerTimeout,
    /// Dependency not met.
    DependencyFailure,
    /// Consensus not reached.
    ConsensusFailure,
    /// Resource limit exceeded.
    ResourceExceeded,
    /// Configuration error.
    ConfigurationError,
    /// Communication error.
    CommunicationError,
}

// =============================================================================
// DecisionEvent Schema for Swarm Coordinator Agent
// =============================================================================

/// DecisionEvent specific to the Swarm Coordinator Agent.
///
/// Emitted for every invocation of the Swarm Coordinator Agent
/// according to the Agent Infrastructure Constitution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmCoordinationDecisionEvent {
    /// Unique event ID.
    pub id: Uuid,

    /// Agent identifier.
    pub agent_id: AgentId,

    /// Agent version.
    pub agent_version: AgentVersion,

    /// Decision type: "swarm_coordination".
    pub decision_type: String,

    /// SHA-256 hash of the input request.
    pub inputs_hash: String,

    /// Coordination outputs.
    pub outputs: SwarmCoordinationDecisionOutputs,

    /// Confidence in the coordination result (0.0 to 1.0).
    ///
    /// Calculated based on:
    /// - Worker success rate
    /// - Consensus convergence
    /// - Objective completion
    pub confidence: f64,

    /// Constraints applied during coordination.
    pub constraints_applied: SwarmCoordinationConstraintsApplied,

    /// Execution reference for tracing.
    pub execution_ref: SwarmCoordinationExecutionRef,

    /// Event timestamp.
    pub timestamp: DateTime<Utc>,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Outputs for the swarm coordination decision event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmCoordinationDecisionOutputs {
    /// Coordination status.
    pub status: SwarmCoordinationStatus,

    /// Total workers.
    pub total_workers: u32,

    /// Successful workers.
    pub successful_workers: u32,

    /// Failed workers.
    pub failed_workers: u32,

    /// Consensus reached.
    pub consensus_reached: bool,

    /// Convergence score.
    pub convergence_score: f64,

    /// Objective met.
    pub objective_met: bool,

    /// Total duration (ms).
    pub total_duration_ms: u64,
}

/// Constraints applied during swarm coordination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmCoordinationConstraintsApplied {
    /// Maximum concurrent workers.
    pub max_concurrent_workers: u32,

    /// Coordination timeout.
    pub coordination_timeout_ms: u64,

    /// Consensus strategy.
    pub consensus_strategy: ConsensusStrategy,

    /// Continue on partial failure.
    pub continue_on_partial_failure: bool,

    /// Convergence threshold.
    pub convergence_threshold: f64,
}

/// Execution reference for swarm coordination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmCoordinationExecutionRef {
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

impl SwarmCoordinationExecutionRef {
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

impl SwarmCoordinationRequest {
    /// Creates a new swarm coordination request.
    pub fn new(workflow_id: Uuid, objective: SwarmObjective, workers: Vec<WorkerSpec>) -> Self {
        Self {
            request_id: Uuid::new_v4(),
            workflow_id,
            objective,
            workers,
            config: SwarmCoordinatorConfig::default(),
            aggregation: AggregationStrategy::default(),
            timestamp: Utc::now(),
        }
    }

    /// Sets the configuration.
    pub fn with_config(mut self, config: SwarmCoordinatorConfig) -> Self {
        self.config = config;
        self
    }

    /// Sets the aggregation strategy.
    pub fn with_aggregation(mut self, aggregation: AggregationStrategy) -> Self {
        self.aggregation = aggregation;
        self
    }
}

impl SwarmObjective {
    /// Creates a new swarm objective.
    pub fn new(
        id: impl Into<String>,
        description: impl Into<String>,
        objective_type: ObjectiveType,
    ) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            objective_type,
            inputs: HashMap::new(),
            success_criteria: Vec::new(),
            priority: 5,
        }
    }

    /// Adds input data.
    pub fn with_input(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.inputs.insert(key.into(), value);
        self
    }

    /// Adds a success criterion.
    pub fn with_criterion(mut self, criterion: SuccessCriterion) -> Self {
        self.success_criteria.push(criterion);
        self
    }
}

impl WorkerSpec {
    /// Creates a new worker specification.
    pub fn new(
        worker_id: impl Into<String>,
        agent_type: WorkerAgentType,
        task: WorkerTask,
    ) -> Self {
        Self {
            worker_id: worker_id.into(),
            agent_type,
            task,
            depends_on: Vec::new(),
            config: WorkerConfig::default(),
        }
    }

    /// Adds a dependency on another worker.
    pub fn depends_on(mut self, worker_id: impl Into<String>) -> Self {
        self.depends_on.push(worker_id.into());
        self
    }

    /// Sets the worker as critical.
    pub fn critical(mut self) -> Self {
        self.config.critical = true;
        self
    }
}

impl WorkerTask {
    /// Creates a new worker task.
    pub fn new(task_id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            description: description.into(),
            inputs: HashMap::new(),
            expected_outputs: Vec::new(),
            timeout_ms: None,
        }
    }

    /// Adds input data.
    pub fn with_input(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.inputs.insert(key.into(), value);
        self
    }

    /// Sets the timeout.
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_objective() -> SwarmObjective {
        SwarmObjective::new(
            "research-objective",
            "Research and analyze the codebase",
            ObjectiveType::Research,
        )
        .with_input("target", serde_json::json!("src/"))
        .with_criterion(SuccessCriterion {
            id: "coverage".to_string(),
            description: "Cover all modules".to_string(),
            required: true,
            validation: ValidationRule::AllSuccess,
        })
    }

    fn sample_workers() -> Vec<WorkerSpec> {
        vec![
            WorkerSpec::new(
                "analyzer-1",
                WorkerAgentType::Custom("analyzer".to_string()),
                WorkerTask::new("analyze-core", "Analyze core module"),
            ),
            WorkerSpec::new(
                "analyzer-2",
                WorkerAgentType::Custom("analyzer".to_string()),
                WorkerTask::new("analyze-utils", "Analyze utils module"),
            ),
            WorkerSpec::new(
                "aggregator",
                WorkerAgentType::Custom("aggregator".to_string()),
                WorkerTask::new("aggregate-results", "Combine analysis results"),
            )
            .depends_on("analyzer-1")
            .depends_on("analyzer-2")
            .critical(),
        ]
    }

    #[test]
    fn test_swarm_coordination_request_creation() {
        let workflow_id = Uuid::new_v4();
        let objective = sample_objective();
        let workers = sample_workers();

        let request = SwarmCoordinationRequest::new(workflow_id, objective, workers);

        assert_eq!(request.workflow_id, workflow_id);
        assert_eq!(request.workers.len(), 3);
        assert_eq!(request.config.max_concurrent_workers, 8);
    }

    #[test]
    fn test_worker_dependencies() {
        let workers = sample_workers();

        let aggregator = workers
            .iter()
            .find(|w| w.worker_id == "aggregator")
            .unwrap();
        assert!(aggregator.depends_on.contains(&"analyzer-1".to_string()));
        assert!(aggregator.depends_on.contains(&"analyzer-2".to_string()));
        assert!(aggregator.config.critical);
    }

    #[test]
    fn test_consensus_strategy_serialization() {
        let strategies = vec![
            ConsensusStrategy::FirstWins,
            ConsensusStrategy::LastWins,
            ConsensusStrategy::Majority,
            ConsensusStrategy::WeightedMajority,
            ConsensusStrategy::MergeAll,
            ConsensusStrategy::Leader {
                leader_id: "leader-1".to_string(),
            },
        ];

        for strategy in strategies {
            let json = serde_json::to_string(&strategy).unwrap();
            let parsed: ConsensusStrategy = serde_json::from_str(&json).unwrap();
            assert_eq!(strategy, parsed);
        }
    }

    #[test]
    fn test_swarm_coordination_status_serialization() {
        let statuses = vec![
            SwarmCoordinationStatus::Success,
            SwarmCoordinationStatus::PartialSuccess,
            SwarmCoordinationStatus::ConsensusReached,
            SwarmCoordinationStatus::Failed,
            SwarmCoordinationStatus::Timeout,
            SwarmCoordinationStatus::Cancelled,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let parsed: SwarmCoordinationStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, parsed);
        }
    }

    #[test]
    fn test_execution_ref_creation() {
        let workflow_id = Uuid::new_v4();
        let request_id = Uuid::new_v4();

        let exec_ref = SwarmCoordinationExecutionRef::new(workflow_id, request_id);

        assert_eq!(exec_ref.workflow_id, workflow_id);
        assert_eq!(exec_ref.request_id, request_id);
        assert!(exec_ref.trace_id.is_some());
        assert!(exec_ref.span_id.is_some());
    }

    #[test]
    fn test_config_defaults() {
        let config = SwarmCoordinatorConfig::default();
        assert_eq!(config.max_concurrent_workers, 8);
        assert_eq!(config.coordination_timeout_ms, 300000);
        assert_eq!(
            config.consensus_strategy,
            ConsensusStrategy::WeightedMajority
        );
        assert!(!config.continue_on_partial_failure);
        assert_eq!(config.convergence_threshold, 0.8);
    }

    #[test]
    fn test_objective_types() {
        let types = vec![
            ObjectiveType::Research,
            ObjectiveType::Analysis,
            ObjectiveType::Generation,
            ObjectiveType::Execution,
            ObjectiveType::Review,
            ObjectiveType::Custom("custom".to_string()),
        ];

        for obj_type in types {
            let json = serde_json::to_string(&obj_type).unwrap();
            let parsed: ObjectiveType = serde_json::from_str(&json).unwrap();
            assert_eq!(obj_type, parsed);
        }
    }
}
