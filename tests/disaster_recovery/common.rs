// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Common utilities for disaster recovery tests.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Disaster recovery metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrMetrics {
    /// Scenario name
    pub scenario: String,

    /// Test start time
    pub start_time: DateTime<Utc>,

    /// Test end time
    pub end_time: DateTime<Utc>,

    /// Detection time (time to detect failure)
    pub detection_time: Duration,

    /// Recovery time objective (RTO) - actual
    pub actual_rto: Duration,

    /// Recovery time objective (RTO) - target
    pub target_rto: Duration,

    /// Recovery point objective (RPO) - actual data loss
    pub actual_rpo: Duration,

    /// Recovery point objective (RPO) - target
    pub target_rpo: Duration,

    /// Test result
    pub result: TestResult,

    /// Data loss occurred
    pub data_loss: bool,

    /// Number of workflows affected
    pub workflows_affected: usize,

    /// Number of workflows recovered
    pub workflows_recovered: usize,

    /// Additional notes
    pub notes: Vec<String>,
}

impl DrMetrics {
    pub fn new(scenario: &str, target_rto: Duration, target_rpo: Duration) -> Self {
        Self {
            scenario: scenario.to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            detection_time: Duration::ZERO,
            actual_rto: Duration::ZERO,
            target_rto,
            actual_rpo: Duration::ZERO,
            target_rpo,
            result: TestResult::Pending,
            data_loss: false,
            workflows_affected: 0,
            workflows_recovered: 0,
            notes: Vec::new(),
        }
    }

    pub fn add_note(&mut self, note: impl Into<String>) {
        self.notes.push(note.into());
    }

    pub fn meets_rto(&self) -> bool {
        self.actual_rto <= self.target_rto
    }

    pub fn meets_rpo(&self) -> bool {
        self.actual_rpo <= self.target_rpo
    }

    pub fn is_successful(&self) -> bool {
        matches!(self.result, TestResult::Success)
            && self.meets_rto()
            && self.meets_rpo()
            && !self.data_loss
            && self.workflows_recovered == self.workflows_affected
    }
}

/// Test result status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestResult {
    Pending,
    Success,
    Partial,
    Failed,
}

/// Timer for measuring operation duration.
pub struct DrTimer {
    start: Instant,
    label: String,
}

impl DrTimer {
    pub fn start(label: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            label: label.into(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    pub fn stop(&self) -> Duration {
        let duration = self.elapsed();
        tracing::info!("{} completed in {:?}", self.label, duration);
        duration
    }
}

/// Test workflow state for DR tests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestWorkflowState {
    pub id: Uuid,
    pub workflow_id: String,
    pub name: String,
    pub status: String,
    pub current_step: usize,
    pub total_steps: usize,
    pub context_data: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TestWorkflowState {
    pub fn new(workflow_id: impl Into<String>, name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            workflow_id: workflow_id.into(),
            name: name.into(),
            status: "running".to_string(),
            current_step: 0,
            total_steps: 5,
            context_data: serde_json::json!({"test": true}),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn is_recoverable(&self) -> bool {
        self.status == "running" || self.status == "paused"
    }
}

/// Health check helper.
pub async fn wait_for_health(
    health_check: impl Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send>>,
    timeout: Duration,
) -> bool {
    let start = Instant::now();

    while start.elapsed() < timeout {
        if health_check().await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    false
}

/// Generate test data for DR scenarios.
pub fn generate_test_workflows(count: usize) -> Vec<TestWorkflowState> {
    (0..count)
        .map(|i| {
            TestWorkflowState::new(
                format!("test-workflow-{}", i),
                format!("Test Workflow {}", i),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dr_metrics_success_criteria() {
        let mut metrics = DrMetrics::new(
            "test_scenario",
            Duration::from_secs(300), // 5 min RTO target
            Duration::from_secs(60),  // 1 min RPO target
        );

        metrics.actual_rto = Duration::from_secs(120); // 2 min actual
        metrics.actual_rpo = Duration::from_secs(30);  // 30s actual
        metrics.workflows_affected = 10;
        metrics.workflows_recovered = 10;
        metrics.result = TestResult::Success;

        assert!(metrics.meets_rto());
        assert!(metrics.meets_rpo());
        assert!(metrics.is_successful());
    }

    #[test]
    fn test_dr_metrics_failure_rto_exceeded() {
        let mut metrics = DrMetrics::new(
            "test_scenario",
            Duration::from_secs(300),
            Duration::from_secs(60),
        );

        metrics.actual_rto = Duration::from_secs(400); // Exceeds target
        metrics.actual_rpo = Duration::from_secs(30);
        metrics.workflows_affected = 10;
        metrics.workflows_recovered = 10;
        metrics.result = TestResult::Success;

        assert!(!metrics.meets_rto());
        assert!(!metrics.is_successful());
    }

    #[test]
    fn test_dr_metrics_data_loss() {
        let mut metrics = DrMetrics::new(
            "test_scenario",
            Duration::from_secs(300),
            Duration::from_secs(60),
        );

        metrics.actual_rto = Duration::from_secs(120);
        metrics.actual_rpo = Duration::from_secs(30);
        metrics.workflows_affected = 10;
        metrics.workflows_recovered = 10;
        metrics.data_loss = true; // Data loss occurred
        metrics.result = TestResult::Success;

        assert!(!metrics.is_successful());
    }
}
