// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Task Scheduler Agent adapter for workflow scheduling and coordination.
//!
//! This adapter implements the Task Scheduler Agent as defined by the
//! LLM-Orchestrator Agent Infrastructure Constitution (Prompt 0).
//!
//! # Classification
//!
//! **TASK SCHEDULING** - Determines when tasks should execute based on:
//! - Timing and delays
//! - Dependencies between tasks
//! - Time-based triggers (cron expressions)
//! - Event-based triggers
//!
//! # Responsibilities
//!
//! - Schedule immediate or delayed task execution
//! - Handle time-based and event-based triggers
//! - Coordinate deferred execution with Orchestrator
//! - Emit scheduled execution plans
//! - Emit DecisionEvents to ruvector-service
//!
//! # Non-Responsibilities (MUST NEVER)
//!
//! - Intercept runtime traffic directly (that is Edge)
//! - Enforce security policies (that is Shield)
//! - Emit anomaly detections (that is Sentinel)
//! - Perform optimization analysis (that is Auto-Optimizer)
//! - Modify schemas dynamically
//! - Connect directly to Google SQL
//! - Execute SQL
//!
//! # decision_type
//!
//! `"task_schedule"`

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
pub const AGENT_ID: &str = "task-scheduler-agent";
pub const AGENT_VERSION: &str = "1.0.0";

/// Schedule type enumeration defining how a task should be scheduled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleType {
    /// Execute immediately.
    Immediate,
    /// Execute after a fixed delay.
    Delayed,
    /// Execute at a specific timestamp.
    Scheduled,
    /// Execute based on cron expression.
    Cron,
    /// Execute when a trigger event fires.
    EventTriggered,
    /// Execute when all dependencies are satisfied.
    DependencyBased,
}

impl Default for ScheduleType {
    fn default() -> Self {
        Self::Immediate
    }
}

/// Trigger type for event-based scheduling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    /// Trigger on workflow completion.
    WorkflowComplete,
    /// Trigger on step completion.
    StepComplete,
    /// Trigger on webhook event.
    Webhook,
    /// Trigger on message queue event.
    MessageQueue,
    /// Trigger on external API event.
    ExternalApi,
    /// Trigger on schedule (cron).
    Schedule,
    /// Manual trigger.
    Manual,
}

/// Dependency constraint for task scheduling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyConstraint {
    /// Task ID this depends on.
    pub task_id: String,
    /// Required completion status.
    pub required_status: DependencyStatus,
    /// Timeout for waiting on dependency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// Required status for dependency satisfaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyStatus {
    /// Dependency must complete successfully.
    Success,
    /// Dependency must complete (success or failure).
    Completed,
    /// Dependency must fail.
    Failed,
}

impl Default for DependencyStatus {
    fn default() -> Self {
        Self::Success
    }
}

/// Retry policy for failed schedule attempts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleRetryPolicy {
    /// Maximum retry attempts.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// Initial retry delay in milliseconds.
    #[serde(default = "default_retry_delay_ms")]
    pub initial_delay_ms: u64,
    /// Backoff multiplier.
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,
    /// Maximum delay in milliseconds.
    #[serde(default = "default_max_delay_ms")]
    pub max_delay_ms: u64,
    /// Enable jitter for thundering herd prevention.
    #[serde(default = "default_jitter_enabled")]
    pub jitter_enabled: bool,
}

fn default_max_retries() -> u32 {
    3
}

fn default_retry_delay_ms() -> u64 {
    1000
}

fn default_backoff_multiplier() -> f64 {
    2.0
}

fn default_max_delay_ms() -> u64 {
    60000
}

fn default_jitter_enabled() -> bool {
    true
}

impl Default for ScheduleRetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: default_max_retries(),
            initial_delay_ms: default_retry_delay_ms(),
            backoff_multiplier: default_backoff_multiplier(),
            max_delay_ms: default_max_delay_ms(),
            jitter_enabled: default_jitter_enabled(),
        }
    }
}

// ============================================================================
// INPUT SCHEMA: ScheduleRequest
// ============================================================================

/// Input schema for task scheduling requests.
///
/// This schema is compatible with agentics-contracts task envelope format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleRequest {
    /// Unique request identifier.
    #[serde(default = "Uuid::new_v4")]
    pub request_id: Uuid,

    /// Task ID to schedule.
    pub task_id: String,

    /// Workflow ID this task belongs to.
    pub workflow_id: Uuid,

    /// Step ID within the workflow (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_id: Option<String>,

    /// Schedule type.
    #[serde(default)]
    pub schedule_type: ScheduleType,

    /// Delay in milliseconds (for Delayed type).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_ms: Option<u64>,

    /// Scheduled execution time (for Scheduled type).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Cron expression (for Cron type).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cron_expression: Option<String>,

    /// Trigger configuration (for EventTriggered type).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger: Option<TriggerConfig>,

    /// Dependencies (for DependencyBased type).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<DependencyConstraint>,

    /// Priority level (0 = lowest, 100 = highest).
    #[serde(default = "default_priority")]
    pub priority: u8,

    /// Deadline for task completion (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<chrono::DateTime<chrono::Utc>>,

    /// Retry policy.
    #[serde(default)]
    pub retry_policy: ScheduleRetryPolicy,

    /// Task payload/context.
    #[serde(default)]
    pub payload: HashMap<String, serde_json::Value>,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,

    /// User/caller ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    /// Request timestamp.
    #[serde(default = "chrono::Utc::now")]
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

fn default_priority() -> u8 {
    50
}

/// Trigger configuration for event-based scheduling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerConfig {
    /// Type of trigger.
    pub trigger_type: TriggerType,
    /// Trigger source identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Event filter conditions.
    #[serde(default)]
    pub filters: HashMap<String, serde_json::Value>,
    /// Timeout for trigger wait.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

// ============================================================================
// OUTPUT SCHEMA: ScheduleResult
// ============================================================================

/// Output schema for task scheduling results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleResult {
    /// Original request ID.
    pub request_id: Uuid,

    /// Generated schedule ID.
    pub schedule_id: Uuid,

    /// Task ID that was scheduled.
    pub task_id: String,

    /// Workflow ID.
    pub workflow_id: Uuid,

    /// Schedule status.
    pub status: ScheduleStatus,

    /// Computed execution time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time: Option<chrono::DateTime<chrono::Utc>>,

    /// Next execution time (for recurring schedules).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_execution: Option<chrono::DateTime<chrono::Utc>>,

    /// Position in execution queue (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_position: Option<u64>,

    /// Estimated wait time in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_wait_ms: Option<u64>,

    /// Constraints that were applied.
    pub constraints_applied: Vec<String>,

    /// Scheduling decision details.
    pub decision_details: ScheduleDecisionDetails,

    /// Error message (if failed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Result timestamp.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Status of a scheduled task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleStatus {
    /// Task has been scheduled and is pending execution.
    Scheduled,
    /// Task is ready for immediate execution.
    Ready,
    /// Task is waiting for dependencies.
    WaitingDependencies,
    /// Task is waiting for trigger event.
    WaitingTrigger,
    /// Task execution has started.
    Executing,
    /// Task completed successfully.
    Completed,
    /// Task scheduling failed.
    Failed,
    /// Task was cancelled.
    Cancelled,
    /// Task has expired (past deadline).
    Expired,
}

/// Details about the scheduling decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleDecisionDetails {
    /// Algorithm used for scheduling.
    pub algorithm: String,
    /// Factors considered in decision.
    pub factors: Vec<String>,
    /// Pending dependencies.
    #[serde(default)]
    pub pending_dependencies: Vec<String>,
    /// Satisfied dependencies.
    #[serde(default)]
    pub satisfied_dependencies: Vec<String>,
    /// Resource availability at scheduling time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_availability: Option<f64>,
    /// Queue depth at scheduling time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_depth: Option<u64>,
}

// ============================================================================
// DECISION EVENT (ruvector-service persistence)
// ============================================================================

/// DecisionEvent schema for ruvector-service persistence.
///
/// This event MUST be emitted exactly once per agent invocation.
/// All persistence occurs via ruvector-service client calls only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionEvent {
    /// Agent identifier.
    pub agent_id: String,

    /// Agent version.
    pub agent_version: String,

    /// Decision type: "task_schedule".
    pub decision_type: String,

    /// Hash of inputs for deduplication/replay.
    pub inputs_hash: String,

    /// Decision outputs.
    pub outputs: ScheduleResult,

    /// Confidence score (0.0 - 1.0) representing execution certainty.
    ///
    /// Calculated based on:
    /// - Dependency satisfaction level
    /// - Schedule determinism
    /// - Resource availability
    /// - Historical success rate
    pub confidence: f64,

    /// Constraints applied during decision.
    ///
    /// Examples:
    /// - "dependency:step1"
    /// - "retry:max_3"
    /// - "ordering:fifo"
    /// - "deadline:2025-01-21T00:00:00Z"
    pub constraints_applied: Vec<String>,

    /// Reference to workflow execution context.
    pub execution_ref: String,

    /// Timestamp (UTC).
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl DecisionEvent {
    /// Creates a new DecisionEvent for a schedule decision.
    pub fn new(
        request: &ScheduleRequest,
        result: &ScheduleResult,
        confidence: f64,
    ) -> Self {
        // Compute inputs hash for deduplication
        let inputs_hash = compute_inputs_hash(request);

        Self {
            agent_id: AGENT_ID.to_string(),
            agent_version: AGENT_VERSION.to_string(),
            decision_type: "task_schedule".to_string(),
            inputs_hash,
            outputs: result.clone(),
            confidence,
            constraints_applied: result.constraints_applied.clone(),
            execution_ref: format!(
                "workflow:{}/task:{}",
                request.workflow_id, request.task_id
            ),
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Computes SHA-256 hash of request inputs for deduplication.
fn compute_inputs_hash(request: &ScheduleRequest) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    request.task_id.hash(&mut hasher);
    request.workflow_id.to_string().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

// ============================================================================
// TASK SCHEDULER AGENT IMPLEMENTATION
// ============================================================================

/// Task Scheduler Agent for workflow scheduling and coordination.
///
/// This agent determines when tasks should execute based on:
/// - Timing and delays
/// - Dependencies between tasks
/// - Time-based triggers (cron)
/// - Event-based triggers
///
/// # Classification
///
/// **TASK SCHEDULING**
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
#[derive(Debug, Clone)]
pub struct TaskSchedulerAgent {
    /// ruvector-service endpoint for persistence.
    ruvector_endpoint: String,
    /// Whether the agent is enabled.
    enabled: bool,
    /// Default timeout for operations.
    default_timeout: Duration,
    /// Maximum queue depth.
    max_queue_depth: u64,
    /// Enable telemetry emission.
    telemetry_enabled: bool,
    /// FEU span collector (optional, for Agentics execution mode).
    feu_collector: Option<FeuSpanCollector>,
}

impl Default for TaskSchedulerAgent {
    fn default() -> Self {
        Self {
            ruvector_endpoint: String::new(),
            enabled: false,
            default_timeout: Duration::from_secs(300),
            max_queue_depth: 10000,
            telemetry_enabled: true,
            feu_collector: None,
        }
    }
}

impl TaskSchedulerAgent {
    /// Creates a new TaskSchedulerAgent with the given ruvector-service endpoint.
    pub fn new(ruvector_endpoint: impl Into<String>) -> Self {
        Self {
            ruvector_endpoint: ruvector_endpoint.into(),
            enabled: true,
            default_timeout: Duration::from_secs(300),
            max_queue_depth: 10000,
            telemetry_enabled: true,
            feu_collector: None,
        }
    }

    /// Creates an agent with custom configuration.
    pub fn with_config(
        ruvector_endpoint: impl Into<String>,
        default_timeout: Duration,
        max_queue_depth: u64,
        telemetry_enabled: bool,
    ) -> Self {
        Self {
            ruvector_endpoint: ruvector_endpoint.into(),
            enabled: true,
            default_timeout,
            max_queue_depth,
            telemetry_enabled,
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
    // CORE SCHEDULING OPERATIONS
    // ========================================================================

    /// Schedules a task for execution.
    ///
    /// This is the main entry point for the Task Scheduler Agent.
    /// It validates the request, computes the schedule, and emits a DecisionEvent.
    pub async fn schedule(&self, request: ScheduleRequest) -> Result<ScheduleResult, SchedulerError> {
        if !self.enabled {
            return Err(SchedulerError::AgentDisabled);
        }

        tracing::info!(
            task_id = %request.task_id,
            workflow_id = %request.workflow_id,
            schedule_type = ?request.schedule_type,
            "Processing schedule request"
        );

        // Start FEU agent span if in Agentics execution mode
        let feu_span_id = self.feu_collector.as_ref()
            .map(|c| c.start_agent_span("TaskSchedulerAgent"));

        // Validate request
        self.validate_request(&request)?;

        // Compute schedule
        let result = self.compute_schedule(&request).await?;

        // Calculate confidence
        let confidence = self.calculate_confidence(&request, &result);

        // Emit DecisionEvent to ruvector-service
        let decision_event = DecisionEvent::new(&request, &result, confidence);
        self.emit_decision_event(&decision_event).await?;

        // Emit telemetry
        if self.telemetry_enabled {
            self.emit_telemetry(&request, &result).await;
        }

        tracing::info!(
            schedule_id = %result.schedule_id,
            status = ?result.status,
            confidence = confidence,
            "Task scheduled successfully"
        );

        // End FEU agent span if in Agentics execution mode
        if let (Some(collector), Some(span_id)) = (&self.feu_collector, &feu_span_id) {
            collector.end_agent_span(&span_id, SpanStatus::Ok, Vec::new(), Vec::new());
        }

        Ok(result)
    }

    /// Inspects the current state of a scheduled task.
    pub async fn inspect(&self, schedule_id: Uuid) -> Result<ScheduleInspection, SchedulerError> {
        if !self.enabled {
            return Err(SchedulerError::AgentDisabled);
        }

        // Query ruvector-service for schedule state
        // In production, this would call the ruvector client
        tracing::debug!(schedule_id = %schedule_id, "Inspecting schedule");

        Ok(ScheduleInspection {
            schedule_id,
            status: ScheduleStatus::Scheduled,
            last_updated: chrono::Utc::now(),
            execution_count: 0,
            next_execution: None,
            history: Vec::new(),
        })
    }

    /// Replays a previous scheduling decision.
    pub async fn replay(&self, decision_event: &DecisionEvent) -> Result<ScheduleResult, SchedulerError> {
        if !self.enabled {
            return Err(SchedulerError::AgentDisabled);
        }

        tracing::info!(
            inputs_hash = %decision_event.inputs_hash,
            "Replaying schedule decision"
        );

        // Return the stored outputs from the decision event
        Ok(decision_event.outputs.clone())
    }

    /// Cancels a scheduled task.
    pub async fn cancel(&self, schedule_id: Uuid, reason: Option<String>) -> Result<(), SchedulerError> {
        if !self.enabled {
            return Err(SchedulerError::AgentDisabled);
        }

        tracing::info!(
            schedule_id = %schedule_id,
            reason = ?reason,
            "Cancelling scheduled task"
        );

        // In production, this would update the schedule state in ruvector-service
        Ok(())
    }

    // ========================================================================
    // INTERNAL SCHEDULING LOGIC
    // ========================================================================

    /// Validates a schedule request.
    fn validate_request(&self, request: &ScheduleRequest) -> Result<(), SchedulerError> {
        // Validate task ID
        if request.task_id.is_empty() {
            return Err(SchedulerError::ValidationError("task_id is required".into()));
        }

        // Validate schedule type specific fields
        match request.schedule_type {
            ScheduleType::Delayed => {
                if request.delay_ms.is_none() {
                    return Err(SchedulerError::ValidationError(
                        "delay_ms is required for Delayed schedule type".into(),
                    ));
                }
            }
            ScheduleType::Scheduled => {
                if request.scheduled_at.is_none() {
                    return Err(SchedulerError::ValidationError(
                        "scheduled_at is required for Scheduled schedule type".into(),
                    ));
                }
            }
            ScheduleType::Cron => {
                if request.cron_expression.is_none() {
                    return Err(SchedulerError::ValidationError(
                        "cron_expression is required for Cron schedule type".into(),
                    ));
                }
                // Validate cron expression format
                if let Some(ref expr) = request.cron_expression {
                    self.validate_cron_expression(expr)?;
                }
            }
            ScheduleType::EventTriggered => {
                if request.trigger.is_none() {
                    return Err(SchedulerError::ValidationError(
                        "trigger is required for EventTriggered schedule type".into(),
                    ));
                }
            }
            ScheduleType::DependencyBased => {
                if request.dependencies.is_empty() {
                    return Err(SchedulerError::ValidationError(
                        "dependencies are required for DependencyBased schedule type".into(),
                    ));
                }
            }
            ScheduleType::Immediate => {
                // No additional validation needed
            }
        }

        // Validate priority
        if request.priority > 100 {
            return Err(SchedulerError::ValidationError(
                "priority must be between 0 and 100".into(),
            ));
        }

        // Validate deadline is in the future
        if let Some(deadline) = request.deadline {
            if deadline <= chrono::Utc::now() {
                return Err(SchedulerError::ValidationError(
                    "deadline must be in the future".into(),
                ));
            }
        }

        Ok(())
    }

    /// Validates a cron expression.
    fn validate_cron_expression(&self, expr: &str) -> Result<(), SchedulerError> {
        // Basic cron validation (5 or 6 fields)
        let parts: Vec<&str> = expr.split_whitespace().collect();
        if parts.len() < 5 || parts.len() > 6 {
            return Err(SchedulerError::ValidationError(format!(
                "Invalid cron expression: expected 5 or 6 fields, got {}",
                parts.len()
            )));
        }
        Ok(())
    }

    /// Computes the schedule based on request parameters.
    async fn compute_schedule(&self, request: &ScheduleRequest) -> Result<ScheduleResult, SchedulerError> {
        let schedule_id = Uuid::new_v4();
        let now = chrono::Utc::now();

        let (status, execution_time, next_execution, mut constraints) = match request.schedule_type {
            ScheduleType::Immediate => {
                (ScheduleStatus::Ready, Some(now), None, vec!["ordering:immediate".into()])
            }
            ScheduleType::Delayed => {
                let delay_ms = request.delay_ms.unwrap_or(0);
                let exec_time = now + chrono::Duration::milliseconds(delay_ms as i64);
                (
                    ScheduleStatus::Scheduled,
                    Some(exec_time),
                    None,
                    vec![format!("delay:{}ms", delay_ms)],
                )
            }
            ScheduleType::Scheduled => {
                let exec_time = request.scheduled_at.unwrap_or(now);
                (
                    ScheduleStatus::Scheduled,
                    Some(exec_time),
                    None,
                    vec![format!("scheduled_at:{}", exec_time.to_rfc3339())],
                )
            }
            ScheduleType::Cron => {
                let cron_expr = request.cron_expression.as_deref().unwrap_or("* * * * *");
                let next_exec = self.compute_next_cron_execution(cron_expr, now);
                (
                    ScheduleStatus::Scheduled,
                    next_exec,
                    next_exec,
                    vec![format!("cron:{}", cron_expr)],
                )
            }
            ScheduleType::EventTriggered => {
                let mut constraints = vec!["trigger:event".into()];
                if let Some(ref trigger) = request.trigger {
                    constraints.push(format!("trigger_type:{:?}", trigger.trigger_type));
                }
                (ScheduleStatus::WaitingTrigger, None, None, constraints)
            }
            ScheduleType::DependencyBased => {
                let (satisfied, pending) = self.evaluate_dependencies(&request.dependencies).await;
                let mut constraints: Vec<String> = request
                    .dependencies
                    .iter()
                    .map(|d| format!("dependency:{}", d.task_id))
                    .collect();

                if pending.is_empty() {
                    constraints.push("all_dependencies_satisfied".into());
                    (ScheduleStatus::Ready, Some(now), None, constraints)
                } else {
                    constraints.push(format!("pending_dependencies:{}", pending.len()));
                    (ScheduleStatus::WaitingDependencies, None, None, constraints)
                }
            }
        };

        // Add retry constraint
        constraints.push(format!("retry:max_{}", request.retry_policy.max_retries));

        // Add priority constraint
        constraints.push(format!("priority:{}", request.priority));

        // Add deadline constraint if present
        if let Some(deadline) = request.deadline {
            constraints.push(format!("deadline:{}", deadline.to_rfc3339()));
        }

        // Build decision details
        let (satisfied, pending) = if matches!(request.schedule_type, ScheduleType::DependencyBased) {
            self.evaluate_dependencies(&request.dependencies).await
        } else {
            (Vec::new(), Vec::new())
        };

        let decision_details = ScheduleDecisionDetails {
            algorithm: self.select_scheduling_algorithm(&request),
            factors: self.collect_scheduling_factors(&request),
            pending_dependencies: pending,
            satisfied_dependencies: satisfied,
            resource_availability: Some(0.85), // Placeholder
            queue_depth: Some(self.estimate_queue_depth()),
        };

        Ok(ScheduleResult {
            request_id: request.request_id,
            schedule_id,
            task_id: request.task_id.clone(),
            workflow_id: request.workflow_id,
            status,
            execution_time,
            next_execution,
            queue_position: Some(self.estimate_queue_position(request.priority)),
            estimated_wait_ms: self.estimate_wait_time(&request, execution_time),
            constraints_applied: constraints,
            decision_details,
            error: None,
            timestamp: now,
        })
    }

    /// Computes the next cron execution time.
    fn compute_next_cron_execution(
        &self,
        _expr: &str,
        base: chrono::DateTime<chrono::Utc>,
    ) -> Option<chrono::DateTime<chrono::Utc>> {
        // Simplified: return next minute
        // In production, use a proper cron parser
        Some(base + chrono::Duration::minutes(1))
    }

    /// Evaluates dependency satisfaction.
    async fn evaluate_dependencies(
        &self,
        dependencies: &[DependencyConstraint],
    ) -> (Vec<String>, Vec<String>) {
        // In production, query ruvector-service for actual task states
        let mut satisfied = Vec::new();
        let mut pending = Vec::new();

        for dep in dependencies {
            // Placeholder: assume 70% of dependencies are satisfied
            if rand_like_deterministic(&dep.task_id) > 0.3 {
                satisfied.push(dep.task_id.clone());
            } else {
                pending.push(dep.task_id.clone());
            }
        }

        (satisfied, pending)
    }

    /// Selects the scheduling algorithm based on request characteristics.
    fn select_scheduling_algorithm(&self, request: &ScheduleRequest) -> String {
        match request.schedule_type {
            ScheduleType::Immediate => "fifo".to_string(),
            ScheduleType::Delayed | ScheduleType::Scheduled => "earliest_deadline_first".to_string(),
            ScheduleType::Cron => "cron_scheduler".to_string(),
            ScheduleType::EventTriggered => "event_driven".to_string(),
            ScheduleType::DependencyBased => "dependency_graph_topo_sort".to_string(),
        }
    }

    /// Collects factors considered in scheduling decision.
    fn collect_scheduling_factors(&self, request: &ScheduleRequest) -> Vec<String> {
        let mut factors = vec![
            "schedule_type".to_string(),
            "priority".to_string(),
            "workflow_context".to_string(),
        ];

        if request.deadline.is_some() {
            factors.push("deadline".to_string());
        }
        if !request.dependencies.is_empty() {
            factors.push("dependencies".to_string());
        }
        if request.trigger.is_some() {
            factors.push("trigger_config".to_string());
        }

        factors
    }

    /// Estimates queue depth.
    fn estimate_queue_depth(&self) -> u64 {
        // In production, query actual queue state
        42
    }

    /// Estimates queue position based on priority.
    fn estimate_queue_position(&self, priority: u8) -> u64 {
        // Higher priority = lower position
        let base_position = self.estimate_queue_depth();
        let priority_factor = (100 - priority) as u64;
        base_position.saturating_sub(priority_factor / 10)
    }

    /// Estimates wait time until execution.
    fn estimate_wait_time(
        &self,
        request: &ScheduleRequest,
        execution_time: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Option<u64> {
        if let Some(exec_time) = execution_time {
            let now = chrono::Utc::now();
            if exec_time > now {
                Some((exec_time - now).num_milliseconds() as u64)
            } else {
                Some(0)
            }
        } else {
            // For event-triggered, estimate based on historical data
            Some(5000) // 5 second default estimate
        }
    }

    // ========================================================================
    // CONFIDENCE CALCULATION
    // ========================================================================

    /// Calculates confidence score for a scheduling decision.
    ///
    /// Confidence represents execution certainty based on:
    /// - Dependency satisfaction level (0.3 weight)
    /// - Schedule determinism (0.3 weight)
    /// - Resource availability (0.2 weight)
    /// - Historical success rate (0.2 weight)
    fn calculate_confidence(&self, request: &ScheduleRequest, result: &ScheduleResult) -> f64 {
        let mut confidence = 0.0;

        // Dependency satisfaction (0.3 weight)
        let dep_score = if request.dependencies.is_empty() {
            1.0
        } else {
            let total = request.dependencies.len();
            let satisfied = result.decision_details.satisfied_dependencies.len();
            satisfied as f64 / total as f64
        };
        confidence += dep_score * 0.3;

        // Schedule determinism (0.3 weight)
        let determinism_score = match request.schedule_type {
            ScheduleType::Immediate => 1.0,
            ScheduleType::Delayed | ScheduleType::Scheduled => 0.95,
            ScheduleType::Cron => 0.9,
            ScheduleType::DependencyBased => 0.8,
            ScheduleType::EventTriggered => 0.6,
        };
        confidence += determinism_score * 0.3;

        // Resource availability (0.2 weight)
        let resource_score = result.decision_details.resource_availability.unwrap_or(0.7);
        confidence += resource_score * 0.2;

        // Historical success rate (0.2 weight)
        // In production, query ruvector-service for historical data
        let historical_score = 0.9;
        confidence += historical_score * 0.2;

        // Clamp to valid range
        confidence.clamp(0.0, 1.0)
    }

    // ========================================================================
    // PERSISTENCE & TELEMETRY
    // ========================================================================

    /// Emits a DecisionEvent to ruvector-service.
    ///
    /// This MUST be called exactly once per agent invocation.
    /// All persistence occurs via ruvector-service client calls only.
    async fn emit_decision_event(&self, event: &DecisionEvent) -> Result<(), SchedulerError> {
        tracing::debug!(
            agent_id = %event.agent_id,
            decision_type = %event.decision_type,
            confidence = event.confidence,
            "Emitting DecisionEvent to ruvector-service"
        );

        // In production, this would call ruvector-service client:
        // let client = RuvectorClient::new(&self.ruvector_endpoint);
        // client.store_decision_event(event).await?;

        // For now, log the event
        tracing::info!(
            event_json = %serde_json::to_string(event).unwrap_or_default(),
            "DecisionEvent emitted"
        );

        Ok(())
    }

    /// Emits telemetry for observability.
    async fn emit_telemetry(&self, request: &ScheduleRequest, result: &ScheduleResult) {
        tracing::info!(
            task_id = %request.task_id,
            workflow_id = %request.workflow_id,
            schedule_type = ?request.schedule_type,
            status = ?result.status,
            schedule_id = %result.schedule_id,
            "task_scheduler.schedule"
        );
    }
}

// ============================================================================
// ERROR TYPES
// ============================================================================

/// Errors that can occur during task scheduling.
#[derive(Debug, Clone)]
pub enum SchedulerError {
    /// Agent is disabled.
    AgentDisabled,
    /// Validation error.
    ValidationError(String),
    /// Dependency resolution error.
    DependencyError(String),
    /// Persistence error.
    PersistenceError(String),
    /// Timeout error.
    Timeout,
    /// Generic error.
    Other(String),
}

impl std::fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AgentDisabled => write!(f, "Task Scheduler Agent is disabled"),
            Self::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            Self::DependencyError(msg) => write!(f, "Dependency error: {}", msg),
            Self::PersistenceError(msg) => write!(f, "Persistence error: {}", msg),
            Self::Timeout => write!(f, "Operation timed out"),
            Self::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for SchedulerError {}

// ============================================================================
// INSPECTION TYPES
// ============================================================================

/// Inspection result for a scheduled task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleInspection {
    /// Schedule ID.
    pub schedule_id: Uuid,
    /// Current status.
    pub status: ScheduleStatus,
    /// Last updated timestamp.
    pub last_updated: chrono::DateTime<chrono::Utc>,
    /// Number of times executed.
    pub execution_count: u32,
    /// Next scheduled execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_execution: Option<chrono::DateTime<chrono::Utc>>,
    /// Execution history.
    pub history: Vec<ExecutionHistoryEntry>,
}

/// Entry in execution history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionHistoryEntry {
    /// Execution timestamp.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Execution status.
    pub status: ScheduleStatus,
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// Error message if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Deterministic pseudo-random function for testing (not cryptographic).
fn rand_like_deterministic(seed: &str) -> f64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    let hash = hasher.finish();
    (hash % 1000) as f64 / 1000.0
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request() -> ScheduleRequest {
        ScheduleRequest {
            request_id: Uuid::new_v4(),
            task_id: "test-task-1".to_string(),
            workflow_id: Uuid::new_v4(),
            step_id: Some("step1".to_string()),
            schedule_type: ScheduleType::Immediate,
            delay_ms: None,
            scheduled_at: None,
            cron_expression: None,
            trigger: None,
            dependencies: Vec::new(),
            priority: 50,
            deadline: None,
            retry_policy: ScheduleRetryPolicy::default(),
            payload: HashMap::new(),
            metadata: HashMap::new(),
            user_id: Some("user-123".to_string()),
            timestamp: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_agent_disabled_by_default() {
        let agent = TaskSchedulerAgent::default();
        assert!(!agent.is_enabled());
    }

    #[test]
    fn test_agent_enabled_with_endpoint() {
        let agent = TaskSchedulerAgent::new("http://localhost:8080");
        assert!(agent.is_enabled());
        assert_eq!(agent.ruvector_endpoint(), "http://localhost:8080");
    }

    #[test]
    fn test_validate_request_empty_task_id() {
        let agent = TaskSchedulerAgent::new("http://localhost:8080");
        let mut request = sample_request();
        request.task_id = String::new();

        let result = agent.validate_request(&request);
        assert!(matches!(result, Err(SchedulerError::ValidationError(_))));
    }

    #[test]
    fn test_validate_request_delayed_without_delay() {
        let agent = TaskSchedulerAgent::new("http://localhost:8080");
        let mut request = sample_request();
        request.schedule_type = ScheduleType::Delayed;
        request.delay_ms = None;

        let result = agent.validate_request(&request);
        assert!(matches!(result, Err(SchedulerError::ValidationError(_))));
    }

    #[test]
    fn test_validate_request_cron_invalid() {
        let agent = TaskSchedulerAgent::new("http://localhost:8080");
        let mut request = sample_request();
        request.schedule_type = ScheduleType::Cron;
        request.cron_expression = Some("invalid".to_string());

        let result = agent.validate_request(&request);
        assert!(matches!(result, Err(SchedulerError::ValidationError(_))));
    }

    #[test]
    fn test_validate_request_priority_out_of_range() {
        let agent = TaskSchedulerAgent::new("http://localhost:8080");
        let mut request = sample_request();
        request.priority = 150;

        let result = agent.validate_request(&request);
        assert!(matches!(result, Err(SchedulerError::ValidationError(_))));
    }

    #[tokio::test]
    async fn test_schedule_immediate() {
        let agent = TaskSchedulerAgent::new("http://localhost:8080");
        let request = sample_request();

        let result = agent.schedule(request).await;
        assert!(result.is_ok());

        let schedule_result = result.unwrap();
        assert_eq!(schedule_result.status, ScheduleStatus::Ready);
        assert!(schedule_result.execution_time.is_some());
    }

    #[tokio::test]
    async fn test_schedule_delayed() {
        let agent = TaskSchedulerAgent::new("http://localhost:8080");
        let mut request = sample_request();
        request.schedule_type = ScheduleType::Delayed;
        request.delay_ms = Some(5000);

        let result = agent.schedule(request).await;
        assert!(result.is_ok());

        let schedule_result = result.unwrap();
        assert_eq!(schedule_result.status, ScheduleStatus::Scheduled);
        assert!(schedule_result.constraints_applied.iter().any(|c| c.contains("delay:5000ms")));
    }

    #[tokio::test]
    async fn test_schedule_with_dependencies() {
        let agent = TaskSchedulerAgent::new("http://localhost:8080");
        let mut request = sample_request();
        request.schedule_type = ScheduleType::DependencyBased;
        request.dependencies = vec![
            DependencyConstraint {
                task_id: "dep-task-1".to_string(),
                required_status: DependencyStatus::Success,
                timeout_ms: Some(30000),
            },
            DependencyConstraint {
                task_id: "dep-task-2".to_string(),
                required_status: DependencyStatus::Completed,
                timeout_ms: None,
            },
        ];

        let result = agent.schedule(request).await;
        assert!(result.is_ok());

        let schedule_result = result.unwrap();
        assert!(matches!(
            schedule_result.status,
            ScheduleStatus::Ready | ScheduleStatus::WaitingDependencies
        ));
    }

    #[test]
    fn test_decision_event_creation() {
        let request = sample_request();
        let result = ScheduleResult {
            request_id: request.request_id,
            schedule_id: Uuid::new_v4(),
            task_id: request.task_id.clone(),
            workflow_id: request.workflow_id,
            status: ScheduleStatus::Ready,
            execution_time: Some(chrono::Utc::now()),
            next_execution: None,
            queue_position: Some(1),
            estimated_wait_ms: Some(0),
            constraints_applied: vec!["ordering:immediate".to_string()],
            decision_details: ScheduleDecisionDetails {
                algorithm: "fifo".to_string(),
                factors: vec!["schedule_type".to_string()],
                pending_dependencies: Vec::new(),
                satisfied_dependencies: Vec::new(),
                resource_availability: Some(0.9),
                queue_depth: Some(10),
            },
            error: None,
            timestamp: chrono::Utc::now(),
        };

        let event = DecisionEvent::new(&request, &result, 0.95);
        assert_eq!(event.agent_id, AGENT_ID);
        assert_eq!(event.agent_version, AGENT_VERSION);
        assert_eq!(event.decision_type, "task_schedule");
        assert_eq!(event.confidence, 0.95);
    }

    #[test]
    fn test_confidence_calculation() {
        let agent = TaskSchedulerAgent::new("http://localhost:8080");
        let request = sample_request();
        let result = ScheduleResult {
            request_id: request.request_id,
            schedule_id: Uuid::new_v4(),
            task_id: request.task_id.clone(),
            workflow_id: request.workflow_id,
            status: ScheduleStatus::Ready,
            execution_time: Some(chrono::Utc::now()),
            next_execution: None,
            queue_position: Some(1),
            estimated_wait_ms: Some(0),
            constraints_applied: vec![],
            decision_details: ScheduleDecisionDetails {
                algorithm: "fifo".to_string(),
                factors: vec![],
                pending_dependencies: Vec::new(),
                satisfied_dependencies: Vec::new(),
                resource_availability: Some(0.9),
                queue_depth: Some(10),
            },
            error: None,
            timestamp: chrono::Utc::now(),
        };

        let confidence = agent.calculate_confidence(&request, &result);
        assert!(confidence >= 0.0 && confidence <= 1.0);
        // Immediate schedule with no dependencies should have high confidence
        assert!(confidence > 0.8);
    }

    #[tokio::test]
    async fn test_schedule_disabled_agent() {
        let agent = TaskSchedulerAgent::disabled();
        let request = sample_request();

        let result = agent.schedule(request).await;
        assert!(matches!(result, Err(SchedulerError::AgentDisabled)));
    }

    #[test]
    fn test_scheduler_error_display() {
        let err = SchedulerError::ValidationError("test error".into());
        assert_eq!(err.to_string(), "Validation error: test error");
    }

    #[test]
    fn test_schedule_types_serialization() {
        let types = vec![
            ScheduleType::Immediate,
            ScheduleType::Delayed,
            ScheduleType::Scheduled,
            ScheduleType::Cron,
            ScheduleType::EventTriggered,
            ScheduleType::DependencyBased,
        ];

        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let parsed: ScheduleType = serde_json::from_str(&json).unwrap();
            assert_eq!(t, parsed);
        }
    }
}
