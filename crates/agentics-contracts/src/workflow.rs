// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Workflow and task envelope schemas for agent communication.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

/// Workflow envelope for agent invocations.
///
/// This is the standard input format for the Workflow Orchestrator Agent.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct WorkflowEnvelope {
    /// Unique envelope identifier.
    pub envelope_id: Uuid,

    /// Workflow definition ID.
    pub workflow_id: Uuid,

    /// Workflow name.
    #[validate(length(min = 1, max = 256))]
    pub workflow_name: String,

    /// Workflow version.
    pub workflow_version: String,

    /// Current execution state.
    pub state: WorkflowState,

    /// Input variables for the workflow.
    #[serde(default)]
    pub inputs: HashMap<String, serde_json::Value>,

    /// Tasks in this workflow.
    pub tasks: Vec<TaskEnvelope>,

    /// Execution configuration.
    pub config: WorkflowExecutionConfig,

    /// Envelope creation timestamp.
    pub created_at: DateTime<Utc>,

    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,

    /// Invoker identification.
    pub invoker: InvokerInfo,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl WorkflowEnvelope {
    /// Creates a new workflow envelope.
    pub fn new(
        workflow_id: Uuid,
        workflow_name: impl Into<String>,
        workflow_version: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            envelope_id: Uuid::new_v4(),
            workflow_id,
            workflow_name: workflow_name.into(),
            workflow_version: workflow_version.into(),
            state: WorkflowState::Pending,
            inputs: HashMap::new(),
            tasks: Vec::new(),
            config: WorkflowExecutionConfig::default(),
            created_at: now,
            updated_at: now,
            invoker: InvokerInfo::default(),
            metadata: HashMap::new(),
        }
    }

    /// Adds input variables.
    pub fn with_inputs(mut self, inputs: HashMap<String, serde_json::Value>) -> Self {
        self.inputs = inputs;
        self
    }

    /// Adds tasks.
    pub fn with_tasks(mut self, tasks: Vec<TaskEnvelope>) -> Self {
        self.tasks = tasks;
        self
    }

    /// Sets the invoker information.
    pub fn with_invoker(mut self, invoker: InvokerInfo) -> Self {
        self.invoker = invoker;
        self
    }
}

/// Workflow execution state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowState {
    /// Workflow is pending execution.
    Pending,
    /// Workflow is scheduled for execution.
    Scheduled,
    /// Workflow is currently running.
    Running,
    /// Workflow is paused.
    Paused,
    /// Workflow completed successfully.
    Completed,
    /// Workflow failed with error.
    Failed,
    /// Workflow was cancelled.
    Cancelled,
    /// Workflow is waiting for retry.
    RetryWaiting,
}

impl Default for WorkflowState {
    fn default() -> Self {
        Self::Pending
    }
}

/// Workflow execution configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionConfig {
    /// Maximum concurrent tasks.
    pub max_concurrency: usize,
    /// Workflow timeout in seconds.
    pub timeout_seconds: u64,
    /// Retry configuration.
    pub retry_config: RetryConfiguration,
    /// Whether to continue on task failure.
    pub continue_on_failure: bool,
    /// Execution strategy.
    pub strategy: ExecutionStrategy,
}

impl Default for WorkflowExecutionConfig {
    fn default() -> Self {
        Self {
            max_concurrency: 4,
            timeout_seconds: 3600,
            retry_config: RetryConfiguration::default(),
            continue_on_failure: false,
            strategy: ExecutionStrategy::Sequential,
        }
    }
}

/// Retry configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfiguration {
    /// Maximum retry attempts.
    pub max_attempts: u32,
    /// Initial delay in milliseconds.
    pub initial_delay_ms: u64,
    /// Maximum delay in milliseconds.
    pub max_delay_ms: u64,
    /// Backoff multiplier.
    pub backoff_multiplier: f64,
    /// Whether to add jitter.
    pub jitter: bool,
}

impl Default for RetryConfiguration {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

/// Execution strategy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStrategy {
    /// Execute tasks sequentially.
    Sequential,
    /// Execute tasks in parallel where possible.
    Parallel,
    /// Execute using DAG dependency resolution.
    DagBased,
    /// Adaptive strategy based on task characteristics.
    Adaptive,
}

/// Task envelope for individual tasks.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct TaskEnvelope {
    /// Unique task identifier.
    pub task_id: Uuid,

    /// Task name.
    #[validate(length(min = 1, max = 256))]
    pub name: String,

    /// Task type.
    pub task_type: TaskType,

    /// Current task state.
    pub state: TaskState,

    /// Task inputs.
    #[serde(default)]
    pub inputs: HashMap<String, serde_json::Value>,

    /// Task outputs (populated after completion).
    #[serde(default)]
    pub outputs: HashMap<String, serde_json::Value>,

    /// Dependencies on other tasks (task IDs).
    #[serde(default)]
    pub depends_on: Vec<Uuid>,

    /// Task configuration.
    pub config: TaskConfig,

    /// Execution attempt count.
    pub attempt: u32,

    /// Error details if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Task start timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,

    /// Task completion timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
}

impl TaskEnvelope {
    /// Creates a new task envelope.
    pub fn new(name: impl Into<String>, task_type: TaskType) -> Self {
        Self {
            task_id: Uuid::new_v4(),
            name: name.into(),
            task_type,
            state: TaskState::Pending,
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            depends_on: Vec::new(),
            config: TaskConfig::default(),
            attempt: 0,
            error: None,
            started_at: None,
            completed_at: None,
        }
    }
}

/// Task type classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    /// LLM completion task.
    LlmCompletion,
    /// Embedding generation task.
    Embedding,
    /// Vector search task.
    VectorSearch,
    /// Data transformation task.
    Transform,
    /// Side-effect action task.
    Action,
    /// Downstream agent invocation.
    AgentInvocation,
    /// Conditional branch.
    Conditional,
    /// Parallel group.
    Parallel,
}

/// Task state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    /// Task is pending execution.
    Pending,
    /// Task is queued for execution.
    Queued,
    /// Task is currently running.
    Running,
    /// Task completed successfully.
    Completed,
    /// Task failed with error.
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
        Self::Pending
    }
}

/// Task configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskConfig {
    /// Task timeout in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
    /// Retry configuration override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_config: Option<RetryConfiguration>,
    /// Provider to use (for LLM/embedding tasks).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Model to use (for LLM tasks).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Additional provider-specific config.
    #[serde(default)]
    pub provider_config: HashMap<String, serde_json::Value>,
}

/// Invoker information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InvokerInfo {
    /// Invoker system name.
    pub system: String,
    /// Invoker version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Request ID from invoker.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// User ID (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// State transition request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionRequest {
    /// Workflow execution ID.
    pub workflow_execution_id: Uuid,
    /// Target state.
    pub target_state: WorkflowState,
    /// Reason for transition.
    pub reason: String,
    /// Transition metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// State transition result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionResult {
    /// Whether transition was successful.
    pub success: bool,
    /// Previous state.
    pub previous_state: WorkflowState,
    /// New state.
    pub new_state: WorkflowState,
    /// Transition timestamp.
    pub timestamp: DateTime<Utc>,
    /// Error message if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_envelope_creation() {
        let envelope = WorkflowEnvelope::new(
            Uuid::new_v4(),
            "test-workflow",
            "1.0.0",
        );
        assert_eq!(envelope.workflow_name, "test-workflow");
        assert_eq!(envelope.state, WorkflowState::Pending);
    }

    #[test]
    fn test_task_envelope_creation() {
        let task = TaskEnvelope::new("test-task", TaskType::LlmCompletion);
        assert_eq!(task.name, "test-task");
        assert_eq!(task.state, TaskState::Pending);
        assert_eq!(task.attempt, 0);
    }

    #[test]
    fn test_state_transitions() {
        let states = vec![
            WorkflowState::Pending,
            WorkflowState::Running,
            WorkflowState::Completed,
        ];

        for state in states {
            let json = serde_json::to_string(&state).unwrap();
            let parsed: WorkflowState = serde_json::from_str(&json).unwrap();
            assert_eq!(state, parsed);
        }
    }
}
