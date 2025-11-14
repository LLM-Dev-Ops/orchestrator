// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Demonstration of state persistence and workflow recovery.
//!
//! This example shows:
//! - Creating a state store
//! - Saving workflow states
//! - Creating checkpoints
//! - Listing active workflows
//! - Recovering from crashes

use llm_orchestrator_state::{
    Checkpoint, SqliteStateStore, StateStore, StepState, WorkflowState,
};
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();

    println!("=== State Persistence Demo ===\n");

    // Create an in-memory SQLite state store (use file path in production)
    let state_store: Arc<dyn StateStore> = Arc::new(SqliteStateStore::new(":memory:").await?);

    println!("✓ State store initialized\n");

    // Simulate workflow execution
    simulate_workflow_execution(&state_store).await?;

    // Simulate crash and recovery
    simulate_crash_recovery(&state_store).await?;

    // Demonstrate checkpoint operations
    demonstrate_checkpoints(&state_store).await?;

    // Cleanup old data
    demonstrate_cleanup(&state_store).await?;

    println!("\n=== Demo Complete ===");
    Ok(())
}

async fn simulate_workflow_execution(
    state_store: &Arc<dyn StateStore>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Simulating Workflow Execution ---");

    // Create a workflow state
    let mut workflow = WorkflowState::new(
        "data-pipeline-001",
        "Data Processing Pipeline",
        Some("user-123".to_string()),
        json!({
            "inputs": {
                "source_file": "data.csv",
                "output_format": "json"
            }
        }),
    );

    println!("Created workflow: {}", workflow.workflow_name);
    println!("  ID: {}", workflow.id);
    println!("  Status: {:?}", workflow.status);

    // Mark as running
    workflow.mark_running();
    state_store.save_workflow_state(&workflow).await?;
    println!("✓ Saved initial state (running)\n");

    // Simulate step execution
    let mut step1 = StepState::new("load_data");
    step1.mark_running();
    step1.mark_completed(json!({
        "rows_loaded": 1000,
        "columns": ["id", "name", "value"]
    }));
    workflow.steps.insert("load_data".to_string(), step1);
    state_store.save_workflow_state(&workflow).await?;
    println!("✓ Step 1 completed: load_data");

    let mut step2 = StepState::new("transform_data");
    step2.mark_running();
    step2.mark_completed(json!({
        "rows_transformed": 1000,
        "transformations_applied": 5
    }));
    workflow.steps.insert("transform_data".to_string(), step2);
    state_store.save_workflow_state(&workflow).await?;
    println!("✓ Step 2 completed: transform_data");

    let mut step3 = StepState::new("export_data");
    step3.mark_running();
    step3.mark_completed(json!({
        "output_file": "output.json",
        "bytes_written": 45000
    }));
    workflow.steps.insert("export_data".to_string(), step3);
    workflow.mark_completed();
    state_store.save_workflow_state(&workflow).await?;
    println!("✓ Step 3 completed: export_data");
    println!("✓ Workflow completed successfully\n");

    Ok(())
}

async fn simulate_crash_recovery(
    state_store: &Arc<dyn StateStore>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Simulating Crash Recovery ---");

    // Create a workflow that will "crash" mid-execution
    let mut workflow = WorkflowState::new(
        "long-running-workflow",
        "Long Running Analysis",
        Some("user-456".to_string()),
        json!({
            "inputs": {
                "dataset_size": "large",
                "analysis_type": "deep"
            }
        }),
    );

    workflow.mark_running();
    state_store.save_workflow_state(&workflow).await?;
    println!("Started workflow: {}", workflow.workflow_name);

    // Complete first 2 steps
    let mut step1 = StepState::new("prepare_data");
    step1.mark_running();
    step1.mark_completed(json!({"status": "ready"}));
    workflow.steps.insert("prepare_data".to_string(), step1);

    let mut step2 = StepState::new("analyze_phase1");
    step2.mark_running();
    step2.mark_completed(json!({"results": "preliminary analysis"}));
    workflow.steps.insert("analyze_phase1".to_string(), step2);

    state_store.save_workflow_state(&workflow).await?;
    println!("✓ Completed 2 steps");

    // Create checkpoint before "crash"
    let snapshot = serde_json::to_value(&workflow)?;
    let checkpoint = Checkpoint::new(workflow.id, "analyze_phase1", snapshot);
    state_store.create_checkpoint(&checkpoint).await?;
    println!("✓ Created checkpoint before crash\n");

    // Simulate crash - orchestrator restarts
    println!("💥 Simulating crash...");
    println!("🔄 Orchestrator restarting...\n");

    // On restart: discover active workflows
    let active_workflows = state_store.list_active_workflows().await?;
    println!(
        "Found {} active workflow(s) to recover:",
        active_workflows.len()
    );

    for wf in &active_workflows {
        println!("  - {} (ID: {})", wf.workflow_name, wf.workflow_id);
        println!("    Status: {:?}", wf.status);
        println!("    Steps completed: {}", wf.steps.len());

        // Get latest checkpoint
        if let Some(cp) = state_store.get_latest_checkpoint(&wf.id).await? {
            println!("    Latest checkpoint: step {}", cp.step_id);

            // Restore from checkpoint
            let restored = state_store.restore_from_checkpoint(&cp.id).await?;
            println!("    ✓ Restored from checkpoint");
            println!(
                "    ✓ Ready to resume from step: {}",
                restored.steps.len() + 1
            );
        }
    }

    println!();
    Ok(())
}

async fn demonstrate_checkpoints(
    state_store: &Arc<dyn StateStore>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Checkpoint Management ---");

    let workflow = WorkflowState::new(
        "checkpoint-demo",
        "Checkpoint Demo Workflow",
        None,
        json!({}),
    );
    state_store.save_workflow_state(&workflow).await?;

    // Create multiple checkpoints
    for i in 1..=15 {
        let snapshot = json!({
            "checkpoint_number": i,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "progress": i * 10
        });
        let cp = Checkpoint::new(workflow.id, format!("step-{}", i), snapshot);
        state_store.create_checkpoint(&cp).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    println!("Created 15 checkpoints");

    // Get latest checkpoint
    if let Some(latest) = state_store.get_latest_checkpoint(&workflow.id).await? {
        println!(
            "✓ Latest checkpoint: step {} at {}",
            latest.step_id,
            latest.timestamp.format("%H:%M:%S")
        );

        // Verify automatic cleanup (should keep last 10)
        println!("✓ Auto-cleanup keeps last 10 checkpoints\n");
    }

    Ok(())
}

async fn demonstrate_cleanup(
    state_store: &Arc<dyn StateStore>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Data Cleanup ---");

    // Create some old completed workflows
    for i in 1..=5 {
        let mut workflow = WorkflowState::new(
            format!("old-workflow-{}", i),
            "Old Completed Workflow",
            None,
            json!({}),
        );
        workflow.mark_completed();

        // Make them appear old
        workflow.completed_at = Some(chrono::Utc::now() - chrono::Duration::days(45));
        state_store.save_workflow_state(&workflow).await?;
    }

    println!("Created 5 old completed workflows");

    // Delete workflows older than 30 days
    let cutoff = chrono::Utc::now() - chrono::Duration::days(30);
    let deleted = state_store.delete_old_states(cutoff).await?;
    println!("✓ Deleted {} old workflow states", deleted);

    // Health check
    state_store.health_check().await?;
    println!("✓ State store health check passed\n");

    Ok(())
}
