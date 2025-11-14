// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! PostgreSQL implementation of the StateStore trait.

use crate::models::{Checkpoint, StepState, WorkflowState, WorkflowStatus};
use crate::traits::{StateStore, StateStoreError, StateStoreResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{ConnectOptions, PgPool, Row};
use std::str::FromStr;
use std::time::Duration;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// PostgreSQL state store implementation.
pub struct PostgresStateStore {
    pool: PgPool,
}

impl PostgresStateStore {
    /// Create a new PostgreSQL state store with connection pooling.
    ///
    /// # Arguments
    /// * `database_url` - PostgreSQL connection string
    /// * `min_connections` - Minimum number of connections in pool (default: 5)
    /// * `max_connections` - Maximum number of connections in pool (default: 20)
    ///
    /// # Example
    /// ```no_run
    /// # use llm_orchestrator_state::postgres::PostgresStateStore;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = PostgresStateStore::new(
    ///     "postgresql://user:pass@localhost/workflows",
    ///     Some(5),
    ///     Some(20),
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(
        database_url: impl AsRef<str>,
        min_connections: Option<u32>,
        max_connections: Option<u32>,
    ) -> StateStoreResult<Self> {
        let min_conn = min_connections.unwrap_or(5);
        let max_conn = max_connections.unwrap_or(20);

        info!(
            "Initializing PostgreSQL state store (min_connections={}, max_connections={})",
            min_conn, max_conn
        );

        // Parse connection options
        let mut connect_opts = PgConnectOptions::from_str(database_url.as_ref())
            .map_err(|e| StateStoreError::Configuration(format!("Invalid database URL: {}", e)))?;

        // Configure logging
        connect_opts = connect_opts.log_statements(tracing::log::LevelFilter::Debug);

        // Build connection pool
        let pool = PgPoolOptions::new()
            .min_connections(min_conn)
            .max_connections(max_conn)
            .acquire_timeout(Duration::from_secs(5))
            .idle_timeout(Some(Duration::from_secs(300)))
            .max_lifetime(Some(Duration::from_secs(1800)))
            .connect_with(connect_opts)
            .await
            .map_err(|e| StateStoreError::Connection(format!("Failed to create connection pool: {}", e)))?;

        info!("PostgreSQL connection pool established");

        let store = Self { pool };

        // Run migrations
        store.run_migrations().await?;

        Ok(store)
    }

    /// Run database migrations.
    async fn run_migrations(&self) -> StateStoreResult<()> {
        info!("Running database migrations");

        // Read migration files
        let migration_001 = include_str!("../migrations/001_initial_schema.sql");
        let migration_002 = include_str!("../migrations/002_checkpoints.sql");

        // Execute migrations
        sqlx::query(migration_001)
            .execute(&self.pool)
            .await
            .map_err(|e| StateStoreError::Database(format!("Migration 001 failed: {}", e)))?;

        sqlx::query(migration_002)
            .execute(&self.pool)
            .await
            .map_err(|e| StateStoreError::Database(format!("Migration 002 failed: {}", e)))?;

        info!("Database migrations completed successfully");
        Ok(())
    }

    /// Get the connection pool (for advanced use cases).
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl StateStore for PostgresStateStore {
    async fn save_workflow_state(&self, state: &WorkflowState) -> StateStoreResult<()> {
        debug!("Saving workflow state: id={}, workflow_id={}", state.id, state.workflow_id);

        let mut tx = self.pool.begin().await?;

        // Serialize context to JSON string
        let context_json = serde_json::to_string(&state.context)?;

        // Upsert workflow state
        sqlx::query(
            r#"
            INSERT INTO workflow_states (
                id, workflow_id, workflow_name, status, user_id,
                started_at, updated_at, completed_at, context, error
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (id) DO UPDATE SET
                status = EXCLUDED.status,
                updated_at = EXCLUDED.updated_at,
                completed_at = EXCLUDED.completed_at,
                context = EXCLUDED.context,
                error = EXCLUDED.error
            "#
        )
        .bind(state.id)
        .bind(&state.workflow_id)
        .bind(&state.workflow_name)
        .bind(state.status.to_string())
        .bind(&state.user_id)
        .bind(state.started_at)
        .bind(state.updated_at)
        .bind(state.completed_at)
        .bind(context_json)
        .bind(&state.error)
        .execute(&mut *tx)
        .await?;

        // Save step states
        for (step_id, step_state) in &state.steps {
            let outputs_json = serde_json::to_string(&step_state.outputs)?;

            sqlx::query(
                r#"
                INSERT INTO step_states (
                    workflow_state_id, step_id, status, started_at, completed_at,
                    outputs, error, retry_count
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT (workflow_state_id, step_id) DO UPDATE SET
                    status = EXCLUDED.status,
                    started_at = EXCLUDED.started_at,
                    completed_at = EXCLUDED.completed_at,
                    outputs = EXCLUDED.outputs,
                    error = EXCLUDED.error,
                    retry_count = EXCLUDED.retry_count
                "#
            )
            .bind(state.id)
            .bind(step_id)
            .bind(step_state.status.to_string())
            .bind(step_state.started_at)
            .bind(step_state.completed_at)
            .bind(outputs_json)
            .bind(&step_state.error)
            .bind(step_state.retry_count)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        debug!("Workflow state saved successfully: id={}", state.id);
        Ok(())
    }

    async fn load_workflow_state(&self, id: &Uuid) -> StateStoreResult<WorkflowState> {
        debug!("Loading workflow state: id={}", id);

        // Load workflow state
        let row = sqlx::query(
            r#"
            SELECT id, workflow_id, workflow_name, status, user_id,
                   started_at, updated_at, completed_at, context, error
            FROM workflow_states
            WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        let workflow_id: Uuid = row.get("id");
        let status_str: String = row.get("status");
        let status = WorkflowStatus::from_str(&status_str)
            .map_err(StateStoreError::InvalidState)?;

        let context_str: String = row.get("context");
        let context = serde_json::from_str(&context_str)?;

        let mut state = WorkflowState {
            id: workflow_id,
            workflow_id: row.get("workflow_id"),
            workflow_name: row.get("workflow_name"),
            status,
            user_id: row.get("user_id"),
            started_at: row.get("started_at"),
            updated_at: row.get("updated_at"),
            completed_at: row.get("completed_at"),
            context,
            error: row.get("error"),
            steps: Default::default(),
        };

        // Load step states
        let step_rows = sqlx::query(
            r#"
            SELECT step_id, status, started_at, completed_at,
                   outputs, error, retry_count
            FROM step_states
            WHERE workflow_state_id = $1
            "#
        )
        .bind(workflow_id)
        .fetch_all(&self.pool)
        .await?;

        for step_row in step_rows {
            let step_id: String = step_row.get("step_id");
            let status_str: String = step_row.get("status");
            let status = crate::models::StepStatus::from_str(&status_str)
                .map_err(StateStoreError::InvalidState)?;

            let outputs_str: Option<String> = step_row.get("outputs");
            let outputs = if let Some(json_str) = outputs_str {
                serde_json::from_str(&json_str)?
            } else {
                serde_json::Value::Null
            };

            let step_state = StepState {
                step_id: step_id.clone(),
                status,
                started_at: step_row.get("started_at"),
                completed_at: step_row.get("completed_at"),
                outputs,
                error: step_row.get("error"),
                retry_count: step_row.get("retry_count"),
            };

            state.steps.insert(step_id, step_state);
        }

        debug!("Workflow state loaded successfully: id={}", id);
        Ok(state)
    }

    async fn load_workflow_state_by_workflow_id(&self, workflow_id: &str) -> StateStoreResult<WorkflowState> {
        debug!("Loading workflow state by workflow_id: {}", workflow_id);

        // Get the most recent state for this workflow_id
        let row = sqlx::query(
            r#"
            SELECT id
            FROM workflow_states
            WHERE workflow_id = $1
            ORDER BY updated_at DESC
            LIMIT 1
            "#
        )
        .bind(workflow_id)
        .fetch_one(&self.pool)
        .await?;

        let id: Uuid = row.get("id");
        self.load_workflow_state(&id).await
    }

    async fn list_active_workflows(&self) -> StateStoreResult<Vec<WorkflowState>> {
        debug!("Listing active workflows");

        let rows = sqlx::query(
            r#"
            SELECT id
            FROM workflow_states
            WHERE status IN ('running', 'pending', 'paused')
            ORDER BY updated_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut workflows = Vec::new();
        for row in rows {
            let id: Uuid = row.get("id");
            match self.load_workflow_state(&id).await {
                Ok(state) => workflows.push(state),
                Err(e) => {
                    warn!("Failed to load workflow state {}: {}", id, e);
                }
            }
        }

        debug!("Found {} active workflows", workflows.len());
        Ok(workflows)
    }

    async fn create_checkpoint(&self, checkpoint: &Checkpoint) -> StateStoreResult<()> {
        debug!("Creating checkpoint: id={}, workflow_state_id={}", checkpoint.id, checkpoint.workflow_state_id);

        let snapshot_json = serde_json::to_string(&checkpoint.snapshot)?;

        sqlx::query(
            r#"
            INSERT INTO checkpoints (id, workflow_state_id, step_id, timestamp, snapshot)
            VALUES ($1, $2, $3, $4, $5)
            "#
        )
        .bind(checkpoint.id)
        .bind(checkpoint.workflow_state_id)
        .bind(&checkpoint.step_id)
        .bind(checkpoint.timestamp)
        .bind(snapshot_json)
        .execute(&self.pool)
        .await?;

        // Cleanup old checkpoints (keep last 10)
        self.cleanup_old_checkpoints(&checkpoint.workflow_state_id, 10).await?;

        debug!("Checkpoint created successfully: id={}", checkpoint.id);
        Ok(())
    }

    async fn get_latest_checkpoint(&self, workflow_state_id: &Uuid) -> StateStoreResult<Option<Checkpoint>> {
        debug!("Getting latest checkpoint for workflow_state_id={}", workflow_state_id);

        let row_opt = sqlx::query(
            r#"
            SELECT id, workflow_state_id, step_id, timestamp, snapshot
            FROM checkpoints
            WHERE workflow_state_id = $1
            ORDER BY timestamp DESC
            LIMIT 1
            "#
        )
        .bind(workflow_state_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row_opt {
            let snapshot_str: String = row.get("snapshot");
            let snapshot = serde_json::from_str(&snapshot_str)?;

            let checkpoint = Checkpoint {
                id: row.get("id"),
                workflow_state_id: row.get("workflow_state_id"),
                step_id: row.get("step_id"),
                timestamp: row.get("timestamp"),
                snapshot,
            };

            debug!("Found latest checkpoint: id={}", checkpoint.id);
            Ok(Some(checkpoint))
        } else {
            debug!("No checkpoints found for workflow_state_id={}", workflow_state_id);
            Ok(None)
        }
    }

    async fn restore_from_checkpoint(&self, checkpoint_id: &Uuid) -> StateStoreResult<WorkflowState> {
        debug!("Restoring from checkpoint: id={}", checkpoint_id);

        let row = sqlx::query(
            r#"
            SELECT snapshot
            FROM checkpoints
            WHERE id = $1
            "#
        )
        .bind(checkpoint_id)
        .fetch_one(&self.pool)
        .await?;

        let snapshot_str: String = row.get("snapshot");
        let state: WorkflowState = serde_json::from_str(&snapshot_str)?;

        debug!("Successfully restored state from checkpoint: id={}", checkpoint_id);
        Ok(state)
    }

    async fn delete_old_states(&self, older_than: DateTime<Utc>) -> StateStoreResult<u64> {
        debug!("Deleting states older than: {}", older_than);

        let result = sqlx::query(
            r#"
            DELETE FROM workflow_states
            WHERE updated_at < $1
              AND status IN ('completed', 'failed')
            "#
        )
        .bind(older_than)
        .execute(&self.pool)
        .await?;

        let deleted = result.rows_affected();
        debug!("Deleted {} old workflow states", deleted);
        Ok(deleted)
    }

    async fn cleanup_old_checkpoints(&self, workflow_state_id: &Uuid, keep_count: usize) -> StateStoreResult<u64> {
        debug!("Cleaning up old checkpoints for workflow_state_id={}, keeping last {}", workflow_state_id, keep_count);

        // PostgreSQL approach: delete checkpoints not in the top N
        let result = sqlx::query(
            r#"
            DELETE FROM checkpoints
            WHERE workflow_state_id = $1
              AND id NOT IN (
                SELECT id FROM checkpoints
                WHERE workflow_state_id = $1
                ORDER BY timestamp DESC
                LIMIT $2
              )
            "#
        )
        .bind(workflow_state_id)
        .bind(keep_count as i64)
        .execute(&self.pool)
        .await?;

        let deleted = result.rows_affected();
        if deleted > 0 {
            debug!("Cleaned up {} old checkpoints", deleted);
        }
        Ok(deleted)
    }

    async fn health_check(&self) -> StateStoreResult<()> {
        debug!("Performing health check");

        // Simple query to verify database connectivity
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| StateStoreError::Connection(format!("Health check failed: {}", e)))?;

        debug!("Health check passed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::WorkflowState;
    use serde_json::json;

    // Integration tests require a running PostgreSQL instance
    // These are disabled by default - run with:
    // TEST_DATABASE_URL=postgresql://... cargo test -- --ignored

    #[tokio::test]
    #[ignore]
    async fn test_postgres_state_store_integration() {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost/test_workflows".to_string());

        let store = PostgresStateStore::new(&database_url, Some(2), Some(5))
            .await
            .expect("Failed to create state store");

        // Test health check
        store.health_check().await.expect("Health check failed");

        // Create test workflow state
        let mut state = WorkflowState::new(
            "test-workflow-1",
            "Test Workflow",
            Some("user-123".to_string()),
            json!({"inputs": {"test": "value"}}),
        );
        state.mark_running();

        // Save state
        store.save_workflow_state(&state).await.expect("Failed to save state");

        // Load state
        let loaded = store.load_workflow_state(&state.id).await.expect("Failed to load state");
        assert_eq!(loaded.workflow_id, state.workflow_id);
        assert_eq!(loaded.status, WorkflowStatus::Running);

        // List active workflows
        let active = store.list_active_workflows().await.expect("Failed to list active workflows");
        assert!(!active.is_empty());

        println!("✅ PostgreSQL integration test passed");
    }
}
