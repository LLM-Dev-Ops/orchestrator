// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Comprehensive unit and integration tests for state persistence.

#[cfg(test)]
mod unit_tests {
    use crate::models::*;
    use serde_json::json;

    #[test]
    fn test_workflow_status_display() {
        assert_eq!(WorkflowStatus::Pending.to_string(), "pending");
        assert_eq!(WorkflowStatus::Running.to_string(), "running");
        assert_eq!(WorkflowStatus::Paused.to_string(), "paused");
        assert_eq!(WorkflowStatus::Completed.to_string(), "completed");
        assert_eq!(WorkflowStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_workflow_status_from_str() {
        use std::str::FromStr;

        assert_eq!(WorkflowStatus::from_str("pending").unwrap(), WorkflowStatus::Pending);
        assert_eq!(WorkflowStatus::from_str("RUNNING").unwrap(), WorkflowStatus::Running);
        assert_eq!(WorkflowStatus::from_str("Completed").unwrap(), WorkflowStatus::Completed);
        assert!(WorkflowStatus::from_str("invalid").is_err());
    }

    #[test]
    fn test_step_status_display() {
        assert_eq!(StepStatus::Pending.to_string(), "pending");
        assert_eq!(StepStatus::Running.to_string(), "running");
        assert_eq!(StepStatus::Completed.to_string(), "completed");
        assert_eq!(StepStatus::Failed.to_string(), "failed");
        assert_eq!(StepStatus::Skipped.to_string(), "skipped");
    }

    #[test]
    fn test_step_status_from_str() {
        use std::str::FromStr;

        assert_eq!(StepStatus::from_str("pending").unwrap(), StepStatus::Pending);
        assert_eq!(StepStatus::from_str("RUNNING").unwrap(), StepStatus::Running);
        assert_eq!(StepStatus::from_str("Skipped").unwrap(), StepStatus::Skipped);
        assert!(StepStatus::from_str("invalid").is_err());
    }

    #[test]
    fn test_workflow_state_creation() {
        let state = WorkflowState::new(
            "test-wf-001",
            "Test Workflow",
            Some("user-123".to_string()),
            json!({"key": "value"}),
        );

        assert_eq!(state.workflow_id, "test-wf-001");
        assert_eq!(state.workflow_name, "Test Workflow");
        assert_eq!(state.status, WorkflowStatus::Pending);
        assert_eq!(state.user_id, Some("user-123".to_string()));
        assert!(state.is_active());
        assert!(state.error.is_none());
        assert!(state.completed_at.is_none());
    }

    #[test]
    fn test_workflow_state_lifecycle() {
        let mut state = WorkflowState::new(
            "test-wf",
            "Test",
            None,
            json!({}),
        );

        // Initial state
        assert_eq!(state.status, WorkflowStatus::Pending);
        assert!(state.is_active());

        // Mark as running
        state.mark_running();
        assert_eq!(state.status, WorkflowStatus::Running);
        assert!(state.is_active());
        assert!(state.completed_at.is_none());

        // Mark as completed
        state.mark_completed();
        assert_eq!(state.status, WorkflowStatus::Completed);
        assert!(!state.is_active());
        assert!(state.completed_at.is_some());
        assert!(state.error.is_none());
    }

    #[test]
    fn test_workflow_state_failure() {
        let mut state = WorkflowState::new(
            "test-wf",
            "Test",
            None,
            json!({}),
        );

        state.mark_running();
        state.mark_failed("Something went wrong");

        assert_eq!(state.status, WorkflowStatus::Failed);
        assert!(!state.is_active());
        assert!(state.completed_at.is_some());
        assert_eq!(state.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_step_state_creation() {
        let step = StepState::new("step-1");

        assert_eq!(step.step_id, "step-1");
        assert_eq!(step.status, StepStatus::Pending);
        assert!(step.started_at.is_none());
        assert!(step.completed_at.is_none());
        assert_eq!(step.outputs, serde_json::Value::Null);
        assert!(step.error.is_none());
        assert_eq!(step.retry_count, 0);
    }

    #[test]
    fn test_step_state_lifecycle() {
        let mut step = StepState::new("step-1");

        // Mark as running
        step.mark_running();
        assert_eq!(step.status, StepStatus::Running);
        assert!(step.started_at.is_some());
        assert!(step.completed_at.is_none());

        // Mark as completed
        let outputs = json!({"result": "success", "count": 42});
        step.mark_completed(outputs.clone());
        assert_eq!(step.status, StepStatus::Completed);
        assert!(step.completed_at.is_some());
        assert_eq!(step.outputs, outputs);
        assert!(step.error.is_none());
    }

    #[test]
    fn test_step_state_failure() {
        let mut step = StepState::new("step-1");

        step.mark_running();
        step.mark_failed("Network timeout");

        assert_eq!(step.status, StepStatus::Failed);
        assert!(step.completed_at.is_some());
        assert_eq!(step.error, Some("Network timeout".to_string()));
    }

    #[test]
    fn test_step_state_retry_count() {
        let mut step = StepState::new("step-1");

        assert_eq!(step.retry_count, 0);

        step.increment_retry();
        assert_eq!(step.retry_count, 1);

        step.increment_retry();
        step.increment_retry();
        assert_eq!(step.retry_count, 3);
    }

    #[test]
    fn test_checkpoint_creation() {
        use uuid::Uuid;

        let workflow_id = Uuid::new_v4();
        let snapshot = json!({"state": "data", "progress": 50});

        let checkpoint = Checkpoint::new(workflow_id, "step-5", snapshot.clone());

        assert_eq!(checkpoint.workflow_state_id, workflow_id);
        assert_eq!(checkpoint.step_id, "step-5");
        assert_eq!(checkpoint.snapshot, snapshot);
        // Timestamp should be recent
        assert!((chrono::Utc::now() - checkpoint.timestamp).num_seconds() < 5);
    }

    #[test]
    fn test_workflow_state_serialization() {
        let state = WorkflowState::new(
            "wf-001",
            "Workflow 1",
            Some("user-1".to_string()),
            json!({"test": true}),
        );

        // Serialize
        let json_str = serde_json::to_string(&state).unwrap();
        assert!(json_str.contains("wf-001"));
        assert!(json_str.contains("Workflow 1"));

        // Deserialize
        let deserialized: WorkflowState = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.workflow_id, state.workflow_id);
        assert_eq!(deserialized.workflow_name, state.workflow_name);
        assert_eq!(deserialized.status, state.status);
    }

    #[test]
    fn test_workflow_state_with_steps() {
        let mut state = WorkflowState::new(
            "wf-001",
            "Workflow with steps",
            None,
            json!({}),
        );

        // Add step states
        let mut step1 = StepState::new("step-1");
        step1.mark_running();
        step1.mark_completed(json!({"result": "done"}));

        let mut step2 = StepState::new("step-2");
        step2.mark_running();

        state.steps.insert("step-1".to_string(), step1);
        state.steps.insert("step-2".to_string(), step2);

        // Verify
        assert_eq!(state.steps.len(), 2);
        assert_eq!(state.steps.get("step-1").unwrap().status, StepStatus::Completed);
        assert_eq!(state.steps.get("step-2").unwrap().status, StepStatus::Running);
    }

    #[test]
    fn test_workflow_state_is_active() {
        let mut state = WorkflowState::new("wf", "test", None, json!({}));

        // Pending is active
        assert!(state.is_active());

        // Running is active
        state.mark_running();
        assert!(state.is_active());

        // Paused is active
        state.status = WorkflowStatus::Paused;
        assert!(state.is_active());

        // Completed is not active
        state.mark_completed();
        assert!(!state.is_active());

        // Failed is not active
        state.status = WorkflowStatus::Failed;
        assert!(!state.is_active());
    }
}

#[cfg(test)]
mod sqlite_integration_tests {
    use crate::{StateStore, SqliteStateStore, WorkflowState, Checkpoint};
    use serde_json::json;
    

    #[tokio::test]
    async fn test_sqlite_store_creation() {
        let store = SqliteStateStore::new(":memory:")
            .await
            .expect("Failed to create store");

        store.health_check().await.expect("Health check failed");
    }

    #[tokio::test]
    async fn test_save_and_load_workflow_state() {
        let store = SqliteStateStore::new(":memory:").await.unwrap();

        let mut state = WorkflowState::new(
            "test-wf-123",
            "Test Workflow",
            Some("user-456".to_string()),
            json!({"inputs": {"query": "test"}}),
        );
        state.mark_running();

        // Save
        store.save_workflow_state(&state).await.unwrap();

        // Load by ID
        let loaded = store.load_workflow_state(&state.id).await.unwrap();
        assert_eq!(loaded.id, state.id);
        assert_eq!(loaded.workflow_id, state.workflow_id);
        assert_eq!(loaded.status, state.status);

        // Load by workflow_id
        let loaded_by_wf_id = store.load_workflow_state_by_workflow_id("test-wf-123").await.unwrap();
        assert_eq!(loaded_by_wf_id.id, state.id);
    }

    #[tokio::test]
    async fn test_update_workflow_state() {
        let store = SqliteStateStore::new(":memory:").await.unwrap();

        let mut state = WorkflowState::new(
            "wf-update",
            "Update Test",
            None,
            json!({}),
        );

        // Save initial state
        store.save_workflow_state(&state).await.unwrap();

        // Update state
        state.mark_running();
        store.save_workflow_state(&state).await.unwrap();

        // Load and verify
        let loaded = store.load_workflow_state(&state.id).await.unwrap();
        assert_eq!(loaded.status, crate::WorkflowStatus::Running);

        // Update again
        state.mark_completed();
        store.save_workflow_state(&state).await.unwrap();

        let loaded = store.load_workflow_state(&state.id).await.unwrap();
        assert_eq!(loaded.status, crate::WorkflowStatus::Completed);
        assert!(loaded.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_list_active_workflows() {
        let store = SqliteStateStore::new(":memory:").await.unwrap();

        // Create multiple workflows
        let mut wf1 = WorkflowState::new("wf-1", "WF 1", None, json!({}));
        wf1.mark_running();
        store.save_workflow_state(&wf1).await.unwrap();

        let mut wf2 = WorkflowState::new("wf-2", "WF 2", None, json!({}));
        wf2.mark_running();
        store.save_workflow_state(&wf2).await.unwrap();

        let mut wf3 = WorkflowState::new("wf-3", "WF 3", None, json!({}));
        wf3.mark_completed();
        store.save_workflow_state(&wf3).await.unwrap();

        // List active (should get wf1 and wf2, not wf3)
        let active = store.list_active_workflows().await.unwrap();
        assert_eq!(active.len(), 2);

        let active_ids: Vec<_> = active.iter().map(|w| w.workflow_id.as_str()).collect();
        assert!(active_ids.contains(&"wf-1"));
        assert!(active_ids.contains(&"wf-2"));
        assert!(!active_ids.contains(&"wf-3"));
    }

    #[tokio::test]
    async fn test_checkpoint_operations() {
        let store = SqliteStateStore::new(":memory:").await.unwrap();

        let state = WorkflowState::new("wf-cp", "Checkpoint Test", None, json!({}));
        store.save_workflow_state(&state).await.unwrap();

        // Create checkpoint
        let snapshot = serde_json::to_value(&state).unwrap();
        let checkpoint = Checkpoint::new(state.id, "step-1", snapshot);
        store.create_checkpoint(&checkpoint).await.unwrap();

        // Get latest checkpoint
        let latest = store.get_latest_checkpoint(&state.id).await.unwrap();
        assert!(latest.is_some());
        let latest = latest.unwrap();
        assert_eq!(latest.step_id, "step-1");

        // Restore from checkpoint
        let restored = store.restore_from_checkpoint(&checkpoint.id).await.unwrap();
        assert_eq!(restored.id, state.id);
    }

    #[tokio::test]
    async fn test_checkpoint_cleanup() {
        let store = SqliteStateStore::new(":memory:").await.unwrap();

        let state = WorkflowState::new("wf-cleanup", "Cleanup Test", None, json!({}));
        store.save_workflow_state(&state).await.unwrap();

        // Create 15 checkpoints
        for i in 1..=15 {
            let snapshot = json!({"checkpoint": i});
            let cp = Checkpoint::new(state.id, format!("step-{}", i), snapshot);
            store.create_checkpoint(&cp).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await; // Ensure different timestamps
        }

        // Should keep only last 10 (due to auto-cleanup in create_checkpoint)
        // Verify by trying to get latest - should exist
        let latest = store.get_latest_checkpoint(&state.id).await.unwrap();
        assert!(latest.is_some());
    }

    #[tokio::test]
    async fn test_delete_old_states() {
        let store = SqliteStateStore::new(":memory:").await.unwrap();

        // Create old completed workflow
        let mut old_wf = WorkflowState::new("old-wf", "Old WF", None, json!({}));
        old_wf.mark_completed();
        old_wf.completed_at = Some(chrono::Utc::now() - chrono::Duration::days(30));
        old_wf.updated_at = chrono::Utc::now() - chrono::Duration::days(30); // Set updated_at to match
        store.save_workflow_state(&old_wf).await.unwrap();

        // Create recent workflow
        let mut new_wf = WorkflowState::new("new-wf", "New WF", None, json!({}));
        new_wf.mark_running();
        store.save_workflow_state(&new_wf).await.unwrap();

        // Delete states older than 7 days
        let cutoff = chrono::Utc::now() - chrono::Duration::days(7);
        let deleted = store.delete_old_states(cutoff).await.unwrap();
        assert_eq!(deleted, 1);

        // Verify new workflow still exists
        let result = store.load_workflow_state(&new_wf.id).await;
        assert!(result.is_ok());

        // Verify old workflow is gone
        let result = store.load_workflow_state(&old_wf.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_workflow_with_step_states() {
        let store = SqliteStateStore::new(":memory:").await.unwrap();

        let mut state = WorkflowState::new("wf-steps", "WF with Steps", None, json!({}));

        // Add step states
        let mut step1 = crate::StepState::new("step-1");
        step1.mark_running();
        step1.mark_completed(json!({"result": "success"}));

        let mut step2 = crate::StepState::new("step-2");
        step2.mark_running();

        state.steps.insert("step-1".to_string(), step1);
        state.steps.insert("step-2".to_string(), step2);

        // Save
        store.save_workflow_state(&state).await.unwrap();

        // Load and verify
        let loaded = store.load_workflow_state(&state.id).await.unwrap();
        assert_eq!(loaded.steps.len(), 2);
        assert_eq!(loaded.steps.get("step-1").unwrap().status, crate::StepStatus::Completed);
        assert_eq!(loaded.steps.get("step-2").unwrap().status, crate::StepStatus::Running);
    }
}
