// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Data models for workflow state persistence.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// Workflow execution status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "snake_case")]
pub enum WorkflowStatus {
    /// Workflow is pending execution.
    Pending,
    /// Workflow is currently running.
    Running,
    /// Workflow is paused.
    Paused,
    /// Workflow completed successfully.
    Completed,
    /// Workflow failed with an error.
    Failed,
}

impl std::fmt::Display for WorkflowStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Paused => write!(f, "paused"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

impl std::str::FromStr for WorkflowStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "paused" => Ok(Self::Paused),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            _ => Err(format!("Invalid workflow status: {}", s)),
        }
    }
}

/// Step execution status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "snake_case")]
pub enum StepStatus {
    /// Step is pending execution.
    Pending,
    /// Step is currently running.
    Running,
    /// Step completed successfully.
    Completed,
    /// Step failed with an error.
    Failed,
    /// Step was skipped.
    Skipped,
}

impl std::fmt::Display for StepStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Skipped => write!(f, "skipped"),
        }
    }
}

impl std::str::FromStr for StepStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "skipped" => Ok(Self::Skipped),
            _ => Err(format!("Invalid step status: {}", s)),
        }
    }
}

/// Workflow state snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    /// Unique identifier for this state record.
    pub id: Uuid,
    /// Workflow ID.
    pub workflow_id: String,
    /// Workflow name.
    pub workflow_name: String,
    /// Execution status.
    pub status: WorkflowStatus,
    /// User ID who initiated the workflow.
    pub user_id: Option<String>,
    /// Timestamp when workflow started.
    pub started_at: DateTime<Utc>,
    /// Timestamp when workflow was last updated.
    pub updated_at: DateTime<Utc>,
    /// Timestamp when workflow completed (if completed).
    pub completed_at: Option<DateTime<Utc>>,
    /// Execution context (inputs, outputs, metadata).
    pub context: Value,
    /// Error message if failed.
    pub error: Option<String>,
    /// Individual step states.
    #[serde(default)]
    pub steps: HashMap<String, StepState>,
}

impl WorkflowState {
    /// Create a new workflow state.
    pub fn new(
        workflow_id: impl Into<String>,
        workflow_name: impl Into<String>,
        user_id: Option<String>,
        context: Value,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            workflow_id: workflow_id.into(),
            workflow_name: workflow_name.into(),
            status: WorkflowStatus::Pending,
            user_id,
            started_at: now,
            updated_at: now,
            completed_at: None,
            context,
            error: None,
            steps: HashMap::new(),
        }
    }

    /// Mark workflow as running.
    pub fn mark_running(&mut self) {
        self.status = WorkflowStatus::Running;
        self.updated_at = Utc::now();
    }

    /// Mark workflow as completed.
    pub fn mark_completed(&mut self) {
        self.status = WorkflowStatus::Completed;
        let now = Utc::now();
        self.updated_at = now;
        self.completed_at = Some(now);
    }

    /// Mark workflow as failed.
    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.status = WorkflowStatus::Failed;
        let now = Utc::now();
        self.updated_at = now;
        self.completed_at = Some(now);
        self.error = Some(error.into());
    }

    /// Check if workflow is active (running or pending).
    pub fn is_active(&self) -> bool {
        matches!(self.status, WorkflowStatus::Running | WorkflowStatus::Pending | WorkflowStatus::Paused)
    }
}

/// Step execution state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepState {
    /// Step ID.
    pub step_id: String,
    /// Execution status.
    pub status: StepStatus,
    /// Timestamp when step started.
    pub started_at: Option<DateTime<Utc>>,
    /// Timestamp when step completed.
    pub completed_at: Option<DateTime<Utc>>,
    /// Step outputs.
    pub outputs: Value,
    /// Error message if failed.
    pub error: Option<String>,
    /// Number of retry attempts.
    pub retry_count: i32,
}

impl StepState {
    /// Create a new step state.
    pub fn new(step_id: impl Into<String>) -> Self {
        Self {
            step_id: step_id.into(),
            status: StepStatus::Pending,
            started_at: None,
            completed_at: None,
            outputs: Value::Null,
            error: None,
            retry_count: 0,
        }
    }

    /// Mark step as running.
    pub fn mark_running(&mut self) {
        self.status = StepStatus::Running;
        self.started_at = Some(Utc::now());
    }

    /// Mark step as completed.
    pub fn mark_completed(&mut self, outputs: Value) {
        self.status = StepStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.outputs = outputs;
    }

    /// Mark step as failed.
    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.status = StepStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.error = Some(error.into());
    }

    /// Increment retry count.
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }
}

/// Checkpoint for workflow recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Unique checkpoint ID.
    pub id: Uuid,
    /// Workflow state ID this checkpoint belongs to.
    pub workflow_state_id: Uuid,
    /// Step ID where checkpoint was created.
    pub step_id: String,
    /// Timestamp when checkpoint was created.
    pub timestamp: DateTime<Utc>,
    /// Complete state snapshot.
    pub snapshot: Value,
}

impl Checkpoint {
    /// Create a new checkpoint.
    pub fn new(
        workflow_state_id: Uuid,
        step_id: impl Into<String>,
        snapshot: Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            workflow_state_id,
            step_id: step_id.into(),
            timestamp: Utc::now(),
            snapshot,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_workflow_status_conversion() {
        assert_eq!(WorkflowStatus::Running.to_string(), "running");
        assert_eq!("completed".parse::<WorkflowStatus>().unwrap(), WorkflowStatus::Completed);
    }

    #[test]
    fn test_step_status_conversion() {
        assert_eq!(StepStatus::Pending.to_string(), "pending");
        assert_eq!("failed".parse::<StepStatus>().unwrap(), StepStatus::Failed);
    }

    #[test]
    fn test_workflow_state_lifecycle() {
        let mut state = WorkflowState::new(
            "wf-123",
            "test-workflow",
            Some("user-1".to_string()),
            json!({"inputs": {"key": "value"}}),
        );

        assert_eq!(state.status, WorkflowStatus::Pending);
        assert!(state.is_active());

        state.mark_running();
        assert_eq!(state.status, WorkflowStatus::Running);
        assert!(state.is_active());

        state.mark_completed();
        assert_eq!(state.status, WorkflowStatus::Completed);
        assert!(!state.is_active());
        assert!(state.completed_at.is_some());
    }

    #[test]
    fn test_step_state_lifecycle() {
        let mut step = StepState::new("step-1");

        assert_eq!(step.status, StepStatus::Pending);
        assert!(step.started_at.is_none());

        step.mark_running();
        assert_eq!(step.status, StepStatus::Running);
        assert!(step.started_at.is_some());

        step.mark_completed(json!({"result": "success"}));
        assert_eq!(step.status, StepStatus::Completed);
        assert!(step.completed_at.is_some());
        assert_eq!(step.outputs, json!({"result": "success"}));
    }

    #[test]
    fn test_checkpoint_creation() {
        let workflow_id = Uuid::new_v4();
        let checkpoint = Checkpoint::new(
            workflow_id,
            "step-1",
            json!({"state": "data"}),
        );

        assert_eq!(checkpoint.workflow_state_id, workflow_id);
        assert_eq!(checkpoint.step_id, "step-1");
        assert_eq!(checkpoint.snapshot, json!({"state": "data"}));
    }
}
