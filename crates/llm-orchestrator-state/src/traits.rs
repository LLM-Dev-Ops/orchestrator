// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Traits for state persistence.

use crate::models::{Checkpoint, WorkflowState};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;

/// Error types for state store operations.
#[derive(Error, Debug)]
pub enum StateStoreError {
    /// Database error.
    #[error("Database error: {0}")]
    Database(String),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Not found error.
    #[error("Not found: {0}")]
    NotFound(String),

    /// Invalid state error.
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Connection error.
    #[error("Connection error: {0}")]
    Connection(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Other error.
    #[error("Other error: {0}")]
    Other(String),
}

impl From<sqlx::Error> for StateStoreError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => StateStoreError::NotFound("Row not found".to_string()),
            sqlx::Error::PoolTimedOut => StateStoreError::Connection("Connection pool timed out".to_string()),
            _ => StateStoreError::Database(err.to_string()),
        }
    }
}

impl From<serde_json::Error> for StateStoreError {
    fn from(err: serde_json::Error) -> Self {
        StateStoreError::Serialization(err.to_string())
    }
}

/// Result type for state store operations.
pub type StateStoreResult<T> = Result<T, StateStoreError>;

/// Trait for workflow state persistence and recovery.
#[async_trait]
pub trait StateStore: Send + Sync {
    /// Save or update a workflow state.
    async fn save_workflow_state(&self, state: &WorkflowState) -> StateStoreResult<()>;

    /// Load a workflow state by ID.
    async fn load_workflow_state(&self, id: &uuid::Uuid) -> StateStoreResult<WorkflowState>;

    /// Load a workflow state by workflow ID (gets the most recent).
    async fn load_workflow_state_by_workflow_id(&self, workflow_id: &str) -> StateStoreResult<WorkflowState>;

    /// List all active workflows (running or paused).
    async fn list_active_workflows(&self) -> StateStoreResult<Vec<WorkflowState>>;

    /// Create a checkpoint.
    async fn create_checkpoint(&self, checkpoint: &Checkpoint) -> StateStoreResult<()>;

    /// Get the latest checkpoint for a workflow.
    async fn get_latest_checkpoint(&self, workflow_state_id: &uuid::Uuid) -> StateStoreResult<Option<Checkpoint>>;

    /// Restore state from a checkpoint.
    async fn restore_from_checkpoint(&self, checkpoint_id: &uuid::Uuid) -> StateStoreResult<WorkflowState>;

    /// Delete old states (cleanup).
    async fn delete_old_states(&self, older_than: DateTime<Utc>) -> StateStoreResult<u64>;

    /// Delete old checkpoints for a workflow (keep only the last N).
    async fn cleanup_old_checkpoints(&self, workflow_state_id: &uuid::Uuid, keep_count: usize) -> StateStoreResult<u64>;

    /// Health check for the state store.
    async fn health_check(&self) -> StateStoreResult<()>;
}
