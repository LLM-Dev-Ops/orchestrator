// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Retry & Recovery Agent adapter for failure handling and recovery orchestration.
//!
//! This adapter implements the Retry & Recovery Agent as defined by the
//! LLM-Orchestrator Agent Infrastructure Constitution (Prompt 0).
//!
//! # Classification
//!
//! **EXECUTION RECOVERY** (maps to TASK_COORDINATION) - Determines safe retry,
//! backoff, or recovery strategies following task or workflow failure.
//!
//! # Responsibilities
//!
//! - Analyze failure conditions and retry eligibility
//! - Apply retry policies and backoff strategies
//! - Recommend recovery paths or workflow termination
//! - Emit retry or recovery decisions
//! - Emit DecisionEvents to ruvector-service
//!
//! # Non-Responsibilities (MUST NEVER)
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
//! - Modify workflow schemas dynamically
//! - Invoke other agents directly (state-driven only)
//!
//! # decision_type
//!
//! `"retry_decision"`

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;
use crate::feu_collector::FeuSpanCollector;
use agentics_contracts::feu::SpanStatus;

// ============================================================================
// CONTRACT SCHEMAS (agentics-contracts compatible)
// ============================================================================

/// Agent metadata for version tracking and identification.
pub const AGENT_ID: &str = "retry-recovery-agent";
pub const AGENT_VERSION: &str = "0.1.0";

/// Recovery action type enumeration defining what action to take after failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryActionType {
    /// Retry the failed task with the same inputs.
    Retry,
    /// Retry with modified inputs or configuration.
    RetryWithModification,
    /// Skip the failed task and continue workflow.
    Skip,
    /// Fail the task permanently (no more retries).
    Fail,
    /// Fail the entire workflow.
    FailWorkflow,
    /// Pause and wait for manual intervention.
    PauseForIntervention,
    /// Rollback to a previous checkpoint.
    Rollback,
    /// Execute an alternate recovery path.
    AlternatePath,
    /// Defer retry to a later time.
    DeferRetry,
}

impl Default for RecoveryActionType {
    fn default() -> Self {
        Self::Retry
    }
}

/// Backoff strategy type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackoffStrategy {
    /// Fixed delay between retries.
    Fixed,
    /// Exponential backoff (delay doubles each retry).
    Exponential,
    /// Linear backoff (delay increases linearly).
    Linear,
    /// Fibonacci backoff (delay follows fibonacci sequence).
    Fibonacci,
    /// Custom backoff with explicit delays.
    Custom,
    /// No delay between retries.
    Immediate,
}

impl Default for BackoffStrategy {
    fn default() -> Self {
        Self::Exponential
    }
}

/// Error category for classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// Transient errors that are likely to succeed on retry.
    Transient,
    /// Permanent errors that will not succeed on retry.
    Permanent,
    /// Timeout errors.
    Timeout,
    /// Resource exhaustion (quotas, limits).
    ResourceExhaustion,
    /// Configuration or validation errors.
    Configuration,
    /// Dependency failures (downstream service).
    Dependency,
    /// Rate limiting errors.
    RateLimit,
    /// Authentication/authorization errors.
    Authentication,
    /// Unknown error category.
    Unknown,
}

impl Default for ErrorCategory {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Retry eligibility status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryEligibility {
    /// Task is eligible for retry.
    Eligible,
    /// Task is not eligible (max attempts exhausted).
    ExhaustedAttempts,
    /// Task is not eligible (non-retryable error).
    NonRetryableError,
    /// Task is not eligible (deadline passed).
    DeadlineExceeded,
    /// Task is not eligible (dependency failure).
    DependencyBlocked,
    /// Task is not eligible (manual intervention required).
    RequiresIntervention,
    /// Eligibility cannot be determined.
    Indeterminate,
}

impl Default for RetryEligibility {
    fn default() -> Self {
        Self::Indeterminate
    }
}

// ============================================================================
// INPUT SCHEMA: RecoveryRequest
// ============================================================================

/// Input schema for recovery requests.
///
/// This schema is compatible with agentics-contracts task envelope format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryRequest {
    /// Unique request identifier.
    #[serde(default = "Uuid::new_v4")]
    pub request_id: Uuid,

    /// Task ID that failed.
    pub task_id: String,

    /// Workflow ID this task belongs to.
    pub workflow_id: Uuid,

    /// Step ID within the workflow (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_id: Option<String>,

    /// Current attempt number (0-indexed, pre-increment).
    pub current_attempt: u32,

    /// Maximum allowed attempts.
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,

    /// Error details from the failed execution.
    pub error: FailureDetails,

    /// Retry configuration.
    #[serde(default)]
    pub retry_config: RetryConfig,

    /// Workflow deadline (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_deadline: Option<chrono::DateTime<chrono::Utc>>,

    /// Previous retry history for this task.
    #[serde(default)]
    pub retry_history: Vec<RetryHistoryEntry>,

    /// Dependencies that must be re-evaluated.
    #[serde(default)]
    pub dependencies: Vec<DependencyInfo>,

    /// Task inputs (for modification on retry).
    #[serde(default)]
    pub task_inputs: HashMap<String, serde_json::Value>,

    /// Task configuration.
    #[serde(default)]
    pub task_config: HashMap<String, serde_json::Value>,

    /// Additional context from invoker.
    #[serde(default)]
    pub context: HashMap<String, serde_json::Value>,

    /// Invoker information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoker: Option<InvokerInfo>,

    /// Request timestamp.
    #[serde(default = "chrono::Utc::now")]
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

fn default_max_attempts() -> u32 {
    3
}

/// Failure details from the failed execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureDetails {
    /// Error code.
    pub error_code: String,

    /// Error message.
    pub error_message: String,

    /// Error category.
    #[serde(default)]
    pub category: ErrorCategory,

    /// Whether the error is explicitly marked as retryable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retryable: Option<bool>,

    /// Timestamp of the failure.
    pub failed_at: chrono::DateTime<chrono::Utc>,

    /// Duration of the failed execution (milliseconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,

    /// Provider that produced the error (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,

    /// Stack trace or error details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<String>,

    /// Structured error metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Retry configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Backoff strategy.
    #[serde(default)]
    pub backoff_strategy: BackoffStrategy,

    /// Initial delay in milliseconds.
    #[serde(default = "default_initial_delay_ms")]
    pub initial_delay_ms: u64,

    /// Maximum delay in milliseconds.
    #[serde(default = "default_max_delay_ms")]
    pub max_delay_ms: u64,

    /// Backoff multiplier (for exponential).
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,

    /// Enable jitter.
    #[serde(default = "default_jitter_enabled")]
    pub jitter_enabled: bool,

    /// Jitter factor (0.0 to 1.0).
    #[serde(default = "default_jitter_factor")]
    pub jitter_factor: f64,

    /// Custom delays for Custom backoff strategy.
    #[serde(default)]
    pub custom_delays_ms: Vec<u64>,

    /// Retryable error codes (if empty, uses category-based logic).
    #[serde(default)]
    pub retryable_error_codes: Vec<String>,

    /// Non-retryable error codes.
    #[serde(default)]
    pub non_retryable_error_codes: Vec<String>,
}

fn default_initial_delay_ms() -> u64 {
    1000
}

fn default_max_delay_ms() -> u64 {
    60000
}

fn default_backoff_multiplier() -> f64 {
    2.0
}

fn default_jitter_enabled() -> bool {
    true
}

fn default_jitter_factor() -> f64 {
    0.25
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            backoff_strategy: BackoffStrategy::default(),
            initial_delay_ms: default_initial_delay_ms(),
            max_delay_ms: default_max_delay_ms(),
            backoff_multiplier: default_backoff_multiplier(),
            jitter_enabled: default_jitter_enabled(),
            jitter_factor: default_jitter_factor(),
            custom_delays_ms: Vec::new(),
            retryable_error_codes: Vec::new(),
            non_retryable_error_codes: Vec::new(),
        }
    }
}

/// Entry in retry history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryHistoryEntry {
    /// Attempt number.
    pub attempt: u32,

    /// Timestamp of the attempt.
    pub attempted_at: chrono::DateTime<chrono::Utc>,

    /// Delay before this attempt (milliseconds).
    pub delay_ms: u64,

    /// Error code from this attempt.
    pub error_code: String,

    /// Error category.
    pub error_category: ErrorCategory,

    /// Duration of execution (milliseconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// Dependency information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    /// Dependent task ID.
    pub task_id: String,

    /// Dependency status.
    pub status: DependencyStatus,

    /// Whether dependency can be re-executed.
    pub can_retry: bool,
}

/// Dependency status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyStatus {
    /// Dependency completed successfully.
    Success,
    /// Dependency failed.
    Failed,
    /// Dependency is pending.
    Pending,
    /// Dependency is running.
    Running,
    /// Dependency was skipped.
    Skipped,
}

impl Default for DependencyStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// Invoker information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokerInfo {
    /// Invoker system name.
    pub system: String,

    /// Invoker version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Correlation ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
}

// ============================================================================
// OUTPUT SCHEMA: RecoveryDecision
// ============================================================================

/// Output schema for recovery decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryDecision {
    /// Original request ID.
    pub request_id: Uuid,

    /// Generated decision ID.
    pub decision_id: Uuid,

    /// Task ID.
    pub task_id: String,

    /// Workflow ID.
    pub workflow_id: Uuid,

    /// Recommended action.
    pub action: RecoveryActionType,

    /// Retry eligibility assessment.
    pub eligibility: RetryEligibility,

    /// Next attempt number (if retry).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_attempt: Option<u32>,

    /// Computed delay before next retry (milliseconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_ms: Option<u64>,

    /// Scheduled retry time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Backoff strategy used.
    pub backoff_strategy: BackoffStrategy,

    /// Error analysis.
    pub error_analysis: ErrorAnalysis,

    /// Constraints applied during decision.
    pub constraints_applied: Vec<String>,

    /// Recommended task state transition.
    pub state_transition: StateTransition,

    /// Additional recommendations.
    #[serde(default)]
    pub recommendations: Vec<Recommendation>,

    /// Decision reasoning.
    pub reasoning: DecisionReasoning,

    /// Error message if decision failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Decision timestamp.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Error analysis details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorAnalysis {
    /// Determined error category.
    pub category: ErrorCategory,

    /// Whether error is retryable.
    pub is_retryable: bool,

    /// Confidence in retryability determination (0.0 to 1.0).
    pub retryability_confidence: f64,

    /// Similar errors in history.
    pub similar_errors_count: u32,

    /// Success rate for similar retries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub historical_success_rate: Option<f64>,

    /// Root cause determination.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_cause: Option<String>,

    /// Analysis factors considered.
    pub factors: Vec<String>,
}

/// State transition recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    /// Current task state.
    pub from_state: TaskState,

    /// Recommended new state.
    pub to_state: TaskState,

    /// Transition reason.
    pub reason: String,
}

/// Task state (compatible with agentics-contracts).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    /// Task is pending.
    Pending,
    /// Task is queued.
    Queued,
    /// Task is running.
    Running,
    /// Task completed successfully.
    Completed,
    /// Task failed.
    Failed,
    /// Task was skipped.
    Skipped,
    /// Task was cancelled.
    Cancelled,
    /// Task is waiting for retry.
    RetryWaiting,
}

impl Default for TaskState {
    fn default() -> Self {
        Self::Failed
    }
}

/// Recommendation for recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    /// Recommendation type.
    pub recommendation_type: RecommendationType,

    /// Recommendation message.
    pub message: String,

    /// Priority (0 = lowest, 100 = highest).
    pub priority: u8,

    /// Associated action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<RecoveryActionType>,
}

/// Recommendation type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationType {
    /// Modify inputs before retry.
    ModifyInputs,
    /// Change provider.
    ChangeProvider,
    /// Increase timeout.
    IncreaseTimeout,
    /// Reduce request size.
    ReduceRequestSize,
    /// Wait for resource availability.
    WaitForResources,
    /// Escalate to manual handling.
    EscalateManual,
    /// Use alternate path.
    UseAlternatePath,
    /// No specific recommendation.
    None,
}

/// Decision reasoning details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionReasoning {
    /// Primary reason for the decision.
    pub primary_reason: String,

    /// Factors considered.
    pub factors_considered: Vec<String>,

    /// Alternative actions considered.
    pub alternatives_considered: Vec<AlternativeAction>,

    /// Why the chosen action was selected.
    pub selection_rationale: String,
}

/// Alternative action that was considered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativeAction {
    /// Action type.
    pub action: RecoveryActionType,

    /// Why it was not selected.
    pub rejection_reason: String,

    /// Confidence if this action were taken.
    pub confidence_if_taken: f64,
}

// ============================================================================
// DECISION EVENT (ruvector-service persistence)
// ============================================================================

/// DecisionEvent schema for ruvector-service persistence.
///
/// This event MUST be emitted exactly once per agent invocation.
/// All persistence occurs via ruvector-service client calls only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryDecisionEvent {
    /// Unique event identifier.
    pub id: Uuid,

    /// Agent identifier.
    pub agent_id: String,

    /// Agent version.
    pub agent_version: String,

    /// Decision type: "retry_decision".
    pub decision_type: String,

    /// Hash of inputs for deduplication/replay.
    pub inputs_hash: String,

    /// Decision outputs.
    pub outputs: RecoveryDecision,

    /// Confidence score (0.0 - 1.0) representing execution certainty.
    ///
    /// Calculated based on:
    /// - Error retryability determination confidence
    /// - Historical success rate for similar retries
    /// - Time constraints satisfaction
    /// - Dependency state clarity
    pub confidence: f64,

    /// Constraints applied during decision.
    ///
    /// Examples:
    /// - "retry:attempt_2_of_3"
    /// - "backoff:exponential"
    /// - "delay:4000ms"
    /// - "deadline:2025-01-21T00:00:00Z"
    pub constraints_applied: Vec<String>,

    /// Reference to workflow execution context.
    pub execution_ref: ExecutionRef,

    /// Timestamp (UTC).
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Execution reference for tracing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRef {
    /// Workflow execution ID.
    pub workflow_execution_id: Uuid,

    /// Step execution ID (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_execution_id: Option<Uuid>,

    /// Trace ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,

    /// Span ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
}

impl RetryDecisionEvent {
    /// Creates a new RetryDecisionEvent.
    pub fn new(
        request: &RecoveryRequest,
        decision: &RecoveryDecision,
        confidence: f64,
    ) -> Self {
        let inputs_hash = compute_inputs_hash(request);

        Self {
            id: Uuid::new_v4(),
            agent_id: AGENT_ID.to_string(),
            agent_version: AGENT_VERSION.to_string(),
            decision_type: "retry_decision".to_string(),
            inputs_hash,
            outputs: decision.clone(),
            confidence,
            constraints_applied: decision.constraints_applied.clone(),
            execution_ref: ExecutionRef {
                workflow_execution_id: request.workflow_id,
                step_execution_id: request.step_id.as_ref().map(|_| Uuid::new_v4()),
                trace_id: Some(Uuid::new_v4().to_string()),
                span_id: Some(Uuid::new_v4().to_string()),
            },
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }
}

/// Computes SHA-256 hash of request inputs for deduplication.
fn compute_inputs_hash(request: &RecoveryRequest) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    request.task_id.hash(&mut hasher);
    request.workflow_id.to_string().hash(&mut hasher);
    request.current_attempt.hash(&mut hasher);
    request.error.error_code.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

// ============================================================================
// RETRY & RECOVERY AGENT IMPLEMENTATION
// ============================================================================

/// Retry & Recovery Agent for failure handling and recovery orchestration.
///
/// This agent determines safe retry, backoff, or recovery strategies
/// following task or workflow failure.
///
/// # Classification
///
/// **EXECUTION RECOVERY** (maps to TASK_COORDINATION)
///
/// # Non-Responsibilities
///
/// This agent MUST NEVER:
/// - Execute workflow steps directly
/// - Modify task inputs or outputs
/// - Change provider configurations
/// - Intercept runtime traffic (that is Edge)
/// - Enforce security policies (that is Shield)
/// - Emit anomaly detections (that is Sentinel)
/// - Perform optimization analysis (that is Auto-Optimizer)
/// - Connect directly to Google SQL
/// - Execute SQL
#[derive(Debug, Clone)]
pub struct RetryRecoveryAgent {
    /// ruvector-service endpoint for persistence.
    ruvector_endpoint: String,
    /// Whether the agent is enabled.
    enabled: bool,
    /// Default timeout for operations.
    default_timeout: Duration,
    /// Enable telemetry emission.
    telemetry_enabled: bool,
    /// Historical data for decision making.
    historical_success_rates: HashMap<String, f64>,
    /// FEU span collector (optional, for Agentics execution mode).
    feu_collector: Option<FeuSpanCollector>,
}

impl Default for RetryRecoveryAgent {
    fn default() -> Self {
        Self {
            ruvector_endpoint: String::new(),
            enabled: false,
            default_timeout: Duration::from_secs(60),
            telemetry_enabled: true,
            historical_success_rates: HashMap::new(),
            feu_collector: None,
        }
    }
}

impl RetryRecoveryAgent {
    /// Creates a new RetryRecoveryAgent with the given ruvector-service endpoint.
    pub fn new(ruvector_endpoint: impl Into<String>) -> Self {
        Self {
            ruvector_endpoint: ruvector_endpoint.into(),
            enabled: true,
            default_timeout: Duration::from_secs(60),
            telemetry_enabled: true,
            historical_success_rates: HashMap::new(),
            feu_collector: None,
        }
    }

    /// Creates an agent with custom configuration.
    pub fn with_config(
        ruvector_endpoint: impl Into<String>,
        default_timeout: Duration,
        telemetry_enabled: bool,
    ) -> Self {
        Self {
            ruvector_endpoint: ruvector_endpoint.into(),
            enabled: true,
            default_timeout,
            telemetry_enabled,
            historical_success_rates: HashMap::new(),
            feu_collector: None,
        }
    }

    /// Creates a disabled agent (no-op mode for testing).
    pub fn disabled() -> Self {
        Self::default()
    }

    /// Returns whether the agent is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Returns the configured ruvector endpoint.
    pub fn ruvector_endpoint(&self) -> &str {
        &self.ruvector_endpoint
    }

    /// Sets the FEU span collector for Agentics execution mode.
    pub fn with_feu_collector(mut self, collector: FeuSpanCollector) -> Self {
        self.feu_collector = Some(collector);
        self
    }

    // ========================================================================
    // CORE RECOVERY OPERATIONS
    // ========================================================================

    /// Evaluates a failed task and determines recovery action.
    ///
    /// This is the main entry point for the Retry & Recovery Agent.
    /// It validates the request, analyzes the failure, determines the
    /// recovery action, and emits a DecisionEvent.
    pub async fn evaluate(&self, request: RecoveryRequest) -> Result<RecoveryDecision, RecoveryError> {
        if !self.enabled {
            return Err(RecoveryError::AgentDisabled);
        }

        tracing::info!(
            task_id = %request.task_id,
            workflow_id = %request.workflow_id,
            current_attempt = request.current_attempt,
            error_code = %request.error.error_code,
            "Processing recovery request"
        );

        // Start FEU agent span if in Agentics execution mode
        let feu_span_id = self.feu_collector.as_ref()
            .map(|c| c.start_agent_span("RetryRecoveryAgent"));

        // Validate request
        self.validate_request(&request)?;

        // Analyze error and determine eligibility
        let error_analysis = self.analyze_error(&request).await;
        let eligibility = self.determine_eligibility(&request, &error_analysis);

        // Determine recovery action
        let decision = self.determine_recovery_action(&request, &error_analysis, &eligibility).await?;

        // Calculate confidence
        let confidence = self.calculate_confidence(&request, &decision, &error_analysis);

        // Emit DecisionEvent to ruvector-service
        let decision_event = RetryDecisionEvent::new(&request, &decision, confidence);
        self.emit_decision_event(&decision_event).await?;

        // Emit telemetry
        if self.telemetry_enabled {
            self.emit_telemetry(&request, &decision).await;
        }

        tracing::info!(
            decision_id = %decision.decision_id,
            action = ?decision.action,
            eligibility = ?decision.eligibility,
            confidence = confidence,
            "Recovery decision made"
        );

        // End FEU agent span if in Agentics execution mode
        if let (Some(collector), Some(span_id)) = (&self.feu_collector, &feu_span_id) {
            collector.end_agent_span(&span_id, SpanStatus::Ok, Vec::new(), Vec::new());
        }

        Ok(decision)
    }

    /// Inspects the current retry state for a task.
    pub async fn inspect(&self, task_id: &str, workflow_id: Uuid) -> Result<RetryInspection, RecoveryError> {
        if !self.enabled {
            return Err(RecoveryError::AgentDisabled);
        }

        tracing::debug!(
            task_id = %task_id,
            workflow_id = %workflow_id,
            "Inspecting retry state"
        );

        // In production, query ruvector-service for retry state
        Ok(RetryInspection {
            task_id: task_id.to_string(),
            workflow_id,
            total_attempts: 0,
            successful_retries: 0,
            failed_retries: 0,
            last_decision: None,
            history: Vec::new(),
        })
    }

    /// Replays a previous recovery decision.
    pub async fn replay(&self, decision_event: &RetryDecisionEvent) -> Result<RecoveryDecision, RecoveryError> {
        if !self.enabled {
            return Err(RecoveryError::AgentDisabled);
        }

        tracing::info!(
            inputs_hash = %decision_event.inputs_hash,
            "Replaying recovery decision"
        );

        Ok(decision_event.outputs.clone())
    }

    // ========================================================================
    // INTERNAL RECOVERY LOGIC
    // ========================================================================

    /// Validates a recovery request.
    fn validate_request(&self, request: &RecoveryRequest) -> Result<(), RecoveryError> {
        if request.task_id.is_empty() {
            return Err(RecoveryError::ValidationError("task_id is required".into()));
        }

        if request.error.error_code.is_empty() {
            return Err(RecoveryError::ValidationError("error_code is required".into()));
        }

        if request.current_attempt > request.max_attempts {
            return Err(RecoveryError::ValidationError(
                "current_attempt cannot exceed max_attempts".into(),
            ));
        }

        // Validate retry config
        if request.retry_config.initial_delay_ms > request.retry_config.max_delay_ms {
            return Err(RecoveryError::ValidationError(
                "initial_delay_ms cannot exceed max_delay_ms".into(),
            ));
        }

        Ok(())
    }

    /// Analyzes the error to determine retryability and category.
    async fn analyze_error(&self, request: &RecoveryRequest) -> ErrorAnalysis {
        let error = &request.error;

        // Determine category if not provided
        let category = if error.category != ErrorCategory::Unknown {
            error.category.clone()
        } else {
            self.categorize_error(&error.error_code, &error.error_message)
        };

        // Determine retryability
        let (is_retryable, retryability_confidence) = self.determine_retryability(
            &category,
            error.retryable,
            &error.error_code,
            &request.retry_config,
        );

        // Check historical data
        let similar_errors_count = request.retry_history
            .iter()
            .filter(|h| h.error_code == error.error_code)
            .count() as u32;

        let historical_success_rate = self.historical_success_rates
            .get(&error.error_code)
            .copied();

        // Determine root cause
        let root_cause = self.determine_root_cause(&category, &error.error_message);

        ErrorAnalysis {
            category,
            is_retryable,
            retryability_confidence,
            similar_errors_count,
            historical_success_rate,
            root_cause,
            factors: vec![
                "error_category".to_string(),
                "error_code".to_string(),
                "retry_history".to_string(),
                "historical_success_rate".to_string(),
            ],
        }
    }

    /// Categorizes an error based on code and message.
    fn categorize_error(&self, error_code: &str, error_message: &str) -> ErrorCategory {
        let code_lower = error_code.to_lowercase();
        let msg_lower = error_message.to_lowercase();

        // Check for timeout errors
        if code_lower.contains("timeout") || msg_lower.contains("timeout") || msg_lower.contains("timed out") {
            return ErrorCategory::Timeout;
        }

        // Check for rate limit errors
        if code_lower.contains("rate") || code_lower.contains("429") || msg_lower.contains("rate limit") {
            return ErrorCategory::RateLimit;
        }

        // Check for authentication errors
        if code_lower.contains("auth") || code_lower.contains("401") || code_lower.contains("403") {
            return ErrorCategory::Authentication;
        }

        // Check for resource errors
        if code_lower.contains("quota") || code_lower.contains("resource") || msg_lower.contains("insufficient") {
            return ErrorCategory::ResourceExhaustion;
        }

        // Check for configuration/validation errors
        if code_lower.contains("validation") || code_lower.contains("invalid") || code_lower.contains("400") {
            return ErrorCategory::Configuration;
        }

        // Check for transient errors
        if code_lower.contains("500") || code_lower.contains("502") || code_lower.contains("503") ||
           code_lower.contains("504") || msg_lower.contains("temporary") || msg_lower.contains("transient") {
            return ErrorCategory::Transient;
        }

        // Check for dependency errors
        if code_lower.contains("dependency") || msg_lower.contains("downstream") || msg_lower.contains("service unavailable") {
            return ErrorCategory::Dependency;
        }

        ErrorCategory::Unknown
    }

    /// Determines if an error is retryable.
    fn determine_retryability(
        &self,
        category: &ErrorCategory,
        explicit_retryable: Option<bool>,
        error_code: &str,
        config: &RetryConfig,
    ) -> (bool, f64) {
        // Check explicit retryable flag
        if let Some(retryable) = explicit_retryable {
            return (retryable, 0.95);
        }

        // Check non-retryable codes
        if config.non_retryable_error_codes.contains(&error_code.to_string()) {
            return (false, 0.95);
        }

        // Check retryable codes
        if config.retryable_error_codes.contains(&error_code.to_string()) {
            return (true, 0.95);
        }

        // Category-based determination
        match category {
            ErrorCategory::Transient => (true, 0.9),
            ErrorCategory::Timeout => (true, 0.85),
            ErrorCategory::RateLimit => (true, 0.9),
            ErrorCategory::Dependency => (true, 0.8),
            ErrorCategory::ResourceExhaustion => (true, 0.7),
            ErrorCategory::Permanent => (false, 0.95),
            ErrorCategory::Configuration => (false, 0.9),
            ErrorCategory::Authentication => (false, 0.95),
            ErrorCategory::Unknown => (true, 0.5), // Conservative: retry but low confidence
        }
    }

    /// Determines a potential root cause.
    fn determine_root_cause(&self, category: &ErrorCategory, message: &str) -> Option<String> {
        match category {
            ErrorCategory::Timeout => Some("Execution exceeded time limit; consider increasing timeout or optimizing operation".to_string()),
            ErrorCategory::RateLimit => Some("API rate limit exceeded; implement backoff or reduce request frequency".to_string()),
            ErrorCategory::ResourceExhaustion => Some("Resource quota exceeded; check limits and usage".to_string()),
            ErrorCategory::Authentication => Some("Authentication failed; verify credentials and permissions".to_string()),
            ErrorCategory::Configuration => Some("Invalid input or configuration; review request parameters".to_string()),
            ErrorCategory::Dependency => Some("Downstream service failure; check service health".to_string()),
            ErrorCategory::Transient => Some("Temporary service disruption; retry should succeed".to_string()),
            ErrorCategory::Permanent => Some(format!("Permanent failure: {}", message)),
            ErrorCategory::Unknown => None,
        }
    }

    /// Determines retry eligibility.
    fn determine_eligibility(&self, request: &RecoveryRequest, analysis: &ErrorAnalysis) -> RetryEligibility {
        // Check if max attempts exhausted
        if request.current_attempt >= request.max_attempts {
            return RetryEligibility::ExhaustedAttempts;
        }

        // Check if error is non-retryable
        if !analysis.is_retryable {
            return RetryEligibility::NonRetryableError;
        }

        // Check deadline
        if let Some(deadline) = request.workflow_deadline {
            let now = chrono::Utc::now();
            // Check if we have time for at least one more retry
            let estimated_delay = self.calculate_delay(
                request.current_attempt + 1,
                &request.retry_config,
            );
            let estimated_retry_time = now + chrono::Duration::milliseconds(estimated_delay as i64);

            if estimated_retry_time > deadline {
                return RetryEligibility::DeadlineExceeded;
            }
        }

        // Check dependencies
        for dep in &request.dependencies {
            if dep.status == DependencyStatus::Failed && !dep.can_retry {
                return RetryEligibility::DependencyBlocked;
            }
        }

        // Check if manual intervention is required
        if matches!(analysis.category, ErrorCategory::Authentication | ErrorCategory::Configuration) {
            // Some configuration errors might require human intervention
            if analysis.similar_errors_count >= 2 {
                return RetryEligibility::RequiresIntervention;
            }
        }

        RetryEligibility::Eligible
    }

    /// Determines the recovery action to take.
    async fn determine_recovery_action(
        &self,
        request: &RecoveryRequest,
        analysis: &ErrorAnalysis,
        eligibility: &RetryEligibility,
    ) -> Result<RecoveryDecision, RecoveryError> {
        let decision_id = Uuid::new_v4();
        let now = chrono::Utc::now();

        // Determine action and state transition based on eligibility
        let (action, state_transition, next_attempt, delay_ms, retry_at) = match eligibility {
            RetryEligibility::Eligible => {
                let next = request.current_attempt + 1;
                let delay = self.calculate_delay(next, &request.retry_config);
                let retry_time = now + chrono::Duration::milliseconds(delay as i64);

                (
                    RecoveryActionType::Retry,
                    StateTransition {
                        from_state: TaskState::Failed,
                        to_state: TaskState::RetryWaiting,
                        reason: format!("Scheduling retry attempt {} of {}", next, request.max_attempts),
                    },
                    Some(next),
                    Some(delay),
                    Some(retry_time),
                )
            }
            RetryEligibility::ExhaustedAttempts => (
                RecoveryActionType::Fail,
                StateTransition {
                    from_state: TaskState::Failed,
                    to_state: TaskState::Failed,
                    reason: "Max retry attempts exhausted".to_string(),
                },
                None,
                None,
                None,
            ),
            RetryEligibility::NonRetryableError => (
                RecoveryActionType::Fail,
                StateTransition {
                    from_state: TaskState::Failed,
                    to_state: TaskState::Failed,
                    reason: "Non-retryable error encountered".to_string(),
                },
                None,
                None,
                None,
            ),
            RetryEligibility::DeadlineExceeded => (
                RecoveryActionType::Fail,
                StateTransition {
                    from_state: TaskState::Failed,
                    to_state: TaskState::Failed,
                    reason: "Workflow deadline would be exceeded".to_string(),
                },
                None,
                None,
                None,
            ),
            RetryEligibility::DependencyBlocked => (
                RecoveryActionType::FailWorkflow,
                StateTransition {
                    from_state: TaskState::Failed,
                    to_state: TaskState::Failed,
                    reason: "Unrecoverable dependency failure".to_string(),
                },
                None,
                None,
                None,
            ),
            RetryEligibility::RequiresIntervention => (
                RecoveryActionType::PauseForIntervention,
                StateTransition {
                    from_state: TaskState::Failed,
                    to_state: TaskState::Pending,
                    reason: "Manual intervention required".to_string(),
                },
                None,
                None,
                None,
            ),
            RetryEligibility::Indeterminate => (
                RecoveryActionType::DeferRetry,
                StateTransition {
                    from_state: TaskState::Failed,
                    to_state: TaskState::RetryWaiting,
                    reason: "Eligibility indeterminate; deferring decision".to_string(),
                },
                Some(request.current_attempt + 1),
                Some(request.retry_config.max_delay_ms),
                None,
            ),
        };

        // Build constraints
        let mut constraints = Vec::new();
        constraints.push(format!("retry:attempt_{}_of_{}", request.current_attempt + 1, request.max_attempts));
        constraints.push(format!("backoff:{:?}", request.retry_config.backoff_strategy).to_lowercase());
        if let Some(d) = delay_ms {
            constraints.push(format!("delay:{}ms", d));
        }
        if let Some(deadline) = request.workflow_deadline {
            constraints.push(format!("deadline:{}", deadline.to_rfc3339()));
        }

        // Build recommendations
        let recommendations = self.build_recommendations(&action, analysis);

        // Build reasoning
        let reasoning = self.build_reasoning(&action, eligibility, analysis);

        Ok(RecoveryDecision {
            request_id: request.request_id,
            decision_id,
            task_id: request.task_id.clone(),
            workflow_id: request.workflow_id,
            action,
            eligibility: eligibility.clone(),
            next_attempt,
            delay_ms,
            retry_at,
            backoff_strategy: request.retry_config.backoff_strategy.clone(),
            error_analysis: analysis.clone(),
            constraints_applied: constraints,
            state_transition,
            recommendations,
            reasoning,
            error: None,
            timestamp: now,
        })
    }

    /// Calculates delay for a given attempt.
    fn calculate_delay(&self, attempt: u32, config: &RetryConfig) -> u64 {
        let base_delay = match config.backoff_strategy {
            BackoffStrategy::Fixed => config.initial_delay_ms,
            BackoffStrategy::Exponential => {
                let factor = config.backoff_multiplier.powi((attempt - 1) as i32);
                (config.initial_delay_ms as f64 * factor) as u64
            }
            BackoffStrategy::Linear => {
                config.initial_delay_ms * attempt as u64
            }
            BackoffStrategy::Fibonacci => {
                self.fibonacci_delay(attempt, config.initial_delay_ms)
            }
            BackoffStrategy::Custom => {
                config.custom_delays_ms
                    .get((attempt - 1) as usize)
                    .copied()
                    .unwrap_or(config.max_delay_ms)
            }
            BackoffStrategy::Immediate => 0,
        };

        // Cap at max delay
        let capped_delay = std::cmp::min(base_delay, config.max_delay_ms);

        // Add jitter if enabled
        if config.jitter_enabled && capped_delay > 0 {
            self.add_jitter(capped_delay, config.jitter_factor)
        } else {
            capped_delay
        }
    }

    /// Calculates Fibonacci-based delay.
    fn fibonacci_delay(&self, attempt: u32, base_ms: u64) -> u64 {
        if attempt <= 1 {
            return base_ms;
        }

        let mut a = 1u64;
        let mut b = 1u64;
        for _ in 2..attempt {
            let next = a + b;
            a = b;
            b = next;
        }
        base_ms * b
    }

    /// Adds jitter to a delay.
    fn add_jitter(&self, delay: u64, jitter_factor: f64) -> u64 {
        // Use deterministic jitter based on delay value for reproducibility
        let jitter_range = (delay as f64 * jitter_factor) as u64;
        if jitter_range == 0 {
            return delay;
        }

        // Simple deterministic jitter
        let jitter = (delay % jitter_range).saturating_add(jitter_range / 2);
        delay.saturating_sub(jitter_range / 2).saturating_add(jitter % jitter_range)
    }

    /// Builds recommendations based on the action and analysis.
    fn build_recommendations(&self, action: &RecoveryActionType, analysis: &ErrorAnalysis) -> Vec<Recommendation> {
        let mut recommendations = Vec::new();

        match action {
            RecoveryActionType::Retry => {
                if matches!(analysis.category, ErrorCategory::Timeout) {
                    recommendations.push(Recommendation {
                        recommendation_type: RecommendationType::IncreaseTimeout,
                        message: "Consider increasing task timeout for future executions".to_string(),
                        priority: 60,
                        action: None,
                    });
                }
                if matches!(analysis.category, ErrorCategory::RateLimit) {
                    recommendations.push(Recommendation {
                        recommendation_type: RecommendationType::WaitForResources,
                        message: "Rate limit encountered; retry with increased delay".to_string(),
                        priority: 70,
                        action: None,
                    });
                }
            }
            RecoveryActionType::Fail | RecoveryActionType::FailWorkflow => {
                if matches!(analysis.category, ErrorCategory::Configuration) {
                    recommendations.push(Recommendation {
                        recommendation_type: RecommendationType::ModifyInputs,
                        message: "Review and correct input parameters before re-submitting".to_string(),
                        priority: 80,
                        action: Some(RecoveryActionType::RetryWithModification),
                    });
                }
                if matches!(analysis.category, ErrorCategory::Authentication) {
                    recommendations.push(Recommendation {
                        recommendation_type: RecommendationType::EscalateManual,
                        message: "Authentication failure requires credential verification".to_string(),
                        priority: 90,
                        action: Some(RecoveryActionType::PauseForIntervention),
                    });
                }
            }
            RecoveryActionType::PauseForIntervention => {
                recommendations.push(Recommendation {
                    recommendation_type: RecommendationType::EscalateManual,
                    message: "Task requires manual review before retry".to_string(),
                    priority: 100,
                    action: None,
                });
            }
            _ => {}
        }

        recommendations
    }

    /// Builds reasoning for the decision.
    fn build_reasoning(
        &self,
        action: &RecoveryActionType,
        eligibility: &RetryEligibility,
        analysis: &ErrorAnalysis,
    ) -> DecisionReasoning {
        let primary_reason = match eligibility {
            RetryEligibility::Eligible => {
                format!("Error is retryable ({:?}) with confidence {:.0}%",
                    analysis.category, analysis.retryability_confidence * 100.0)
            }
            RetryEligibility::ExhaustedAttempts => "Maximum retry attempts have been exhausted".to_string(),
            RetryEligibility::NonRetryableError => {
                format!("Error {:?} is not retryable", analysis.category)
            }
            RetryEligibility::DeadlineExceeded => "Workflow deadline would be exceeded by next retry".to_string(),
            RetryEligibility::DependencyBlocked => "Task is blocked by failed dependency that cannot be retried".to_string(),
            RetryEligibility::RequiresIntervention => "Repeated failures of same type require manual intervention".to_string(),
            RetryEligibility::Indeterminate => "Unable to determine eligibility; conservative deferral".to_string(),
        };

        let factors_considered = vec![
            format!("Error category: {:?}", analysis.category),
            format!("Retryability: {} (confidence: {:.0}%)",
                analysis.is_retryable, analysis.retryability_confidence * 100.0),
            format!("Similar errors in history: {}", analysis.similar_errors_count),
            format!("Historical success rate: {}",
                analysis.historical_success_rate.map_or("N/A".to_string(), |r| format!("{:.0}%", r * 100.0))),
        ];

        let mut alternatives_considered = Vec::new();

        if *action != RecoveryActionType::Retry && *eligibility != RetryEligibility::NonRetryableError {
            alternatives_considered.push(AlternativeAction {
                action: RecoveryActionType::Retry,
                rejection_reason: match eligibility {
                    RetryEligibility::ExhaustedAttempts => "Max attempts reached".to_string(),
                    RetryEligibility::DeadlineExceeded => "Would miss deadline".to_string(),
                    _ => "Other constraints prevented retry".to_string(),
                },
                confidence_if_taken: analysis.historical_success_rate.unwrap_or(0.3),
            });
        }

        if *action != RecoveryActionType::Skip {
            alternatives_considered.push(AlternativeAction {
                action: RecoveryActionType::Skip,
                rejection_reason: "Task is required for workflow completion".to_string(),
                confidence_if_taken: 0.0,
            });
        }

        let selection_rationale = format!(
            "Selected {:?} because: {}. Error analysis indicates {} retryability.",
            action,
            primary_reason,
            if analysis.is_retryable { "positive" } else { "negative" }
        );

        DecisionReasoning {
            primary_reason,
            factors_considered,
            alternatives_considered,
            selection_rationale,
        }
    }

    // ========================================================================
    // CONFIDENCE CALCULATION
    // ========================================================================

    /// Calculates confidence score for a recovery decision.
    ///
    /// Confidence represents execution certainty based on:
    /// - Error retryability determination confidence (0.3 weight)
    /// - Historical success rate for similar retries (0.3 weight)
    /// - Time constraints satisfaction (0.2 weight)
    /// - Dependency state clarity (0.2 weight)
    fn calculate_confidence(
        &self,
        request: &RecoveryRequest,
        decision: &RecoveryDecision,
        analysis: &ErrorAnalysis,
    ) -> f64 {
        let mut confidence = 0.0;

        // Retryability determination confidence (0.3 weight)
        confidence += analysis.retryability_confidence * 0.3;

        // Historical success rate (0.3 weight)
        let historical_score = analysis.historical_success_rate.unwrap_or(0.5);
        confidence += historical_score * 0.3;

        // Time constraints satisfaction (0.2 weight)
        let time_score = if let Some(deadline) = request.workflow_deadline {
            if let Some(retry_at) = decision.retry_at {
                if retry_at < deadline {
                    1.0
                } else {
                    0.0
                }
            } else {
                0.5 // No retry scheduled
            }
        } else {
            1.0 // No deadline constraint
        };
        confidence += time_score * 0.2;

        // Dependency clarity (0.2 weight)
        let dep_score = if request.dependencies.is_empty() {
            1.0
        } else {
            let clear_deps = request.dependencies.iter()
                .filter(|d| d.status != DependencyStatus::Pending && d.status != DependencyStatus::Running)
                .count();
            clear_deps as f64 / request.dependencies.len() as f64
        };
        confidence += dep_score * 0.2;

        confidence.clamp(0.0, 1.0)
    }

    // ========================================================================
    // PERSISTENCE & TELEMETRY
    // ========================================================================

    /// Emits a RetryDecisionEvent to ruvector-service.
    ///
    /// This MUST be called exactly once per agent invocation.
    /// All persistence occurs via ruvector-service client calls only.
    async fn emit_decision_event(&self, event: &RetryDecisionEvent) -> Result<(), RecoveryError> {
        tracing::debug!(
            agent_id = %event.agent_id,
            decision_type = %event.decision_type,
            confidence = event.confidence,
            action = ?event.outputs.action,
            "Emitting RetryDecisionEvent to ruvector-service"
        );

        // In production, this would call ruvector-service client:
        // let client = RuvectorClient::new(&self.ruvector_endpoint);
        // client.store_decision_event(event).await?;

        // For now, log the event
        tracing::info!(
            event_id = %event.id,
            task_id = %event.outputs.task_id,
            action = ?event.outputs.action,
            eligibility = ?event.outputs.eligibility,
            "RetryDecisionEvent emitted"
        );

        Ok(())
    }

    /// Emits telemetry for observability.
    async fn emit_telemetry(&self, request: &RecoveryRequest, decision: &RecoveryDecision) {
        tracing::info!(
            task_id = %request.task_id,
            workflow_id = %request.workflow_id,
            current_attempt = request.current_attempt,
            action = ?decision.action,
            eligibility = ?decision.eligibility,
            decision_id = %decision.decision_id,
            "retry_recovery.evaluate"
        );
    }
}

// ============================================================================
// ERROR TYPES
// ============================================================================

/// Errors that can occur during recovery processing.
#[derive(Debug, Clone)]
pub enum RecoveryError {
    /// Agent is disabled.
    AgentDisabled,
    /// Validation error.
    ValidationError(String),
    /// Analysis error.
    AnalysisError(String),
    /// Persistence error.
    PersistenceError(String),
    /// Timeout error.
    Timeout,
    /// Generic error.
    Other(String),
}

impl std::fmt::Display for RecoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AgentDisabled => write!(f, "Retry & Recovery Agent is disabled"),
            Self::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            Self::AnalysisError(msg) => write!(f, "Analysis error: {}", msg),
            Self::PersistenceError(msg) => write!(f, "Persistence error: {}", msg),
            Self::Timeout => write!(f, "Operation timed out"),
            Self::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for RecoveryError {}

// ============================================================================
// INSPECTION TYPES
// ============================================================================

/// Inspection result for retry state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryInspection {
    /// Task ID.
    pub task_id: String,
    /// Workflow ID.
    pub workflow_id: Uuid,
    /// Total retry attempts.
    pub total_attempts: u32,
    /// Successful retries.
    pub successful_retries: u32,
    /// Failed retries.
    pub failed_retries: u32,
    /// Last recovery decision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_decision: Option<RecoveryDecision>,
    /// Decision history.
    pub history: Vec<RetryDecisionEvent>,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request() -> RecoveryRequest {
        RecoveryRequest {
            request_id: Uuid::new_v4(),
            task_id: "test-task-1".to_string(),
            workflow_id: Uuid::new_v4(),
            step_id: Some("step1".to_string()),
            current_attempt: 1,
            max_attempts: 3,
            error: FailureDetails {
                error_code: "TIMEOUT".to_string(),
                error_message: "Request timed out after 30s".to_string(),
                category: ErrorCategory::Timeout,
                retryable: None,
                failed_at: chrono::Utc::now(),
                duration_ms: Some(30000),
                provider: Some("openai".to_string()),
                stack_trace: None,
                metadata: HashMap::new(),
            },
            retry_config: RetryConfig::default(),
            workflow_deadline: None,
            retry_history: Vec::new(),
            dependencies: Vec::new(),
            task_inputs: HashMap::new(),
            task_config: HashMap::new(),
            context: HashMap::new(),
            invoker: Some(InvokerInfo {
                system: "workflow-orchestrator".to_string(),
                version: Some("1.0.0".to_string()),
                correlation_id: Some(Uuid::new_v4().to_string()),
            }),
            timestamp: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_agent_disabled_by_default() {
        let agent = RetryRecoveryAgent::default();
        assert!(!agent.is_enabled());
    }

    #[test]
    fn test_agent_enabled_with_endpoint() {
        let agent = RetryRecoveryAgent::new("http://localhost:8080");
        assert!(agent.is_enabled());
        assert_eq!(agent.ruvector_endpoint(), "http://localhost:8080");
    }

    #[test]
    fn test_validate_request_empty_task_id() {
        let agent = RetryRecoveryAgent::new("http://localhost:8080");
        let mut request = sample_request();
        request.task_id = String::new();

        let result = agent.validate_request(&request);
        assert!(matches!(result, Err(RecoveryError::ValidationError(_))));
    }

    #[test]
    fn test_validate_request_empty_error_code() {
        let agent = RetryRecoveryAgent::new("http://localhost:8080");
        let mut request = sample_request();
        request.error.error_code = String::new();

        let result = agent.validate_request(&request);
        assert!(matches!(result, Err(RecoveryError::ValidationError(_))));
    }

    #[test]
    fn test_validate_request_attempt_exceeds_max() {
        let agent = RetryRecoveryAgent::new("http://localhost:8080");
        let mut request = sample_request();
        request.current_attempt = 5;
        request.max_attempts = 3;

        let result = agent.validate_request(&request);
        assert!(matches!(result, Err(RecoveryError::ValidationError(_))));
    }

    #[test]
    fn test_categorize_error_timeout() {
        let agent = RetryRecoveryAgent::new("http://localhost:8080");
        let category = agent.categorize_error("TIMEOUT", "Request timed out");
        assert_eq!(category, ErrorCategory::Timeout);
    }

    #[test]
    fn test_categorize_error_rate_limit() {
        let agent = RetryRecoveryAgent::new("http://localhost:8080");
        let category = agent.categorize_error("429", "Rate limit exceeded");
        assert_eq!(category, ErrorCategory::RateLimit);
    }

    #[test]
    fn test_categorize_error_authentication() {
        let agent = RetryRecoveryAgent::new("http://localhost:8080");
        let category = agent.categorize_error("401", "Unauthorized");
        assert_eq!(category, ErrorCategory::Authentication);
    }

    #[test]
    fn test_categorize_error_transient() {
        let agent = RetryRecoveryAgent::new("http://localhost:8080");
        let category = agent.categorize_error("503", "Service temporarily unavailable");
        assert_eq!(category, ErrorCategory::Transient);
    }

    #[test]
    fn test_calculate_delay_fixed() {
        let agent = RetryRecoveryAgent::new("http://localhost:8080");
        let mut config = RetryConfig::default();
        config.backoff_strategy = BackoffStrategy::Fixed;
        config.jitter_enabled = false;
        config.initial_delay_ms = 1000;

        assert_eq!(agent.calculate_delay(1, &config), 1000);
        assert_eq!(agent.calculate_delay(2, &config), 1000);
        assert_eq!(agent.calculate_delay(3, &config), 1000);
    }

    #[test]
    fn test_calculate_delay_exponential() {
        let agent = RetryRecoveryAgent::new("http://localhost:8080");
        let mut config = RetryConfig::default();
        config.backoff_strategy = BackoffStrategy::Exponential;
        config.jitter_enabled = false;
        config.initial_delay_ms = 1000;
        config.backoff_multiplier = 2.0;
        config.max_delay_ms = 60000;

        assert_eq!(agent.calculate_delay(1, &config), 1000);
        assert_eq!(agent.calculate_delay(2, &config), 2000);
        assert_eq!(agent.calculate_delay(3, &config), 4000);
        assert_eq!(agent.calculate_delay(4, &config), 8000);
    }

    #[test]
    fn test_calculate_delay_linear() {
        let agent = RetryRecoveryAgent::new("http://localhost:8080");
        let mut config = RetryConfig::default();
        config.backoff_strategy = BackoffStrategy::Linear;
        config.jitter_enabled = false;
        config.initial_delay_ms = 1000;

        assert_eq!(agent.calculate_delay(1, &config), 1000);
        assert_eq!(agent.calculate_delay(2, &config), 2000);
        assert_eq!(agent.calculate_delay(3, &config), 3000);
    }

    #[test]
    fn test_calculate_delay_max_cap() {
        let agent = RetryRecoveryAgent::new("http://localhost:8080");
        let mut config = RetryConfig::default();
        config.backoff_strategy = BackoffStrategy::Exponential;
        config.jitter_enabled = false;
        config.initial_delay_ms = 1000;
        config.backoff_multiplier = 2.0;
        config.max_delay_ms = 5000;

        assert_eq!(agent.calculate_delay(10, &config), 5000); // Capped at max
    }

    #[tokio::test]
    async fn test_evaluate_eligible_for_retry() {
        let agent = RetryRecoveryAgent::new("http://localhost:8080");
        let request = sample_request();

        let result = agent.evaluate(request).await;
        assert!(result.is_ok());

        let decision = result.unwrap();
        assert_eq!(decision.action, RecoveryActionType::Retry);
        assert_eq!(decision.eligibility, RetryEligibility::Eligible);
        assert!(decision.next_attempt.is_some());
        assert!(decision.delay_ms.is_some());
    }

    #[tokio::test]
    async fn test_evaluate_exhausted_attempts() {
        let agent = RetryRecoveryAgent::new("http://localhost:8080");
        let mut request = sample_request();
        request.current_attempt = 3;
        request.max_attempts = 3;

        let result = agent.evaluate(request).await;
        assert!(result.is_ok());

        let decision = result.unwrap();
        assert_eq!(decision.action, RecoveryActionType::Fail);
        assert_eq!(decision.eligibility, RetryEligibility::ExhaustedAttempts);
        assert!(decision.next_attempt.is_none());
    }

    #[tokio::test]
    async fn test_evaluate_non_retryable_error() {
        let agent = RetryRecoveryAgent::new("http://localhost:8080");
        let mut request = sample_request();
        request.error.category = ErrorCategory::Authentication;
        request.error.error_code = "401".to_string();

        let result = agent.evaluate(request).await;
        assert!(result.is_ok());

        let decision = result.unwrap();
        assert_eq!(decision.action, RecoveryActionType::Fail);
        assert_eq!(decision.eligibility, RetryEligibility::NonRetryableError);
    }

    #[tokio::test]
    async fn test_evaluate_disabled_agent() {
        let agent = RetryRecoveryAgent::disabled();
        let request = sample_request();

        let result = agent.evaluate(request).await;
        assert!(matches!(result, Err(RecoveryError::AgentDisabled)));
    }

    #[test]
    fn test_decision_event_creation() {
        let request = sample_request();
        let decision = RecoveryDecision {
            request_id: request.request_id,
            decision_id: Uuid::new_v4(),
            task_id: request.task_id.clone(),
            workflow_id: request.workflow_id,
            action: RecoveryActionType::Retry,
            eligibility: RetryEligibility::Eligible,
            next_attempt: Some(2),
            delay_ms: Some(2000),
            retry_at: Some(chrono::Utc::now() + chrono::Duration::milliseconds(2000)),
            backoff_strategy: BackoffStrategy::Exponential,
            error_analysis: ErrorAnalysis {
                category: ErrorCategory::Timeout,
                is_retryable: true,
                retryability_confidence: 0.85,
                similar_errors_count: 0,
                historical_success_rate: Some(0.8),
                root_cause: Some("Timeout occurred".to_string()),
                factors: vec!["category".to_string()],
            },
            constraints_applied: vec!["retry:attempt_2_of_3".to_string()],
            state_transition: StateTransition {
                from_state: TaskState::Failed,
                to_state: TaskState::RetryWaiting,
                reason: "Scheduling retry".to_string(),
            },
            recommendations: Vec::new(),
            reasoning: DecisionReasoning {
                primary_reason: "Error is retryable".to_string(),
                factors_considered: vec![],
                alternatives_considered: vec![],
                selection_rationale: "Selected Retry".to_string(),
            },
            error: None,
            timestamp: chrono::Utc::now(),
        };

        let event = RetryDecisionEvent::new(&request, &decision, 0.85);
        assert_eq!(event.agent_id, AGENT_ID);
        assert_eq!(event.agent_version, AGENT_VERSION);
        assert_eq!(event.decision_type, "retry_decision");
        assert_eq!(event.confidence, 0.85);
    }

    #[test]
    fn test_recovery_error_display() {
        let err = RecoveryError::ValidationError("test error".into());
        assert_eq!(err.to_string(), "Validation error: test error");
    }

    #[test]
    fn test_recovery_action_types_serialization() {
        let types = vec![
            RecoveryActionType::Retry,
            RecoveryActionType::RetryWithModification,
            RecoveryActionType::Skip,
            RecoveryActionType::Fail,
            RecoveryActionType::FailWorkflow,
            RecoveryActionType::PauseForIntervention,
            RecoveryActionType::Rollback,
            RecoveryActionType::AlternatePath,
            RecoveryActionType::DeferRetry,
        ];

        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let parsed: RecoveryActionType = serde_json::from_str(&json).unwrap();
            assert_eq!(t, parsed);
        }
    }

    #[test]
    fn test_backoff_strategies_serialization() {
        let strategies = vec![
            BackoffStrategy::Fixed,
            BackoffStrategy::Exponential,
            BackoffStrategy::Linear,
            BackoffStrategy::Fibonacci,
            BackoffStrategy::Custom,
            BackoffStrategy::Immediate,
        ];

        for s in strategies {
            let json = serde_json::to_string(&s).unwrap();
            let parsed: BackoffStrategy = serde_json::from_str(&json).unwrap();
            assert_eq!(s, parsed);
        }
    }
}
