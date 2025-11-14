// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! State persistence and recovery for LLM workflow orchestrator.
//!
//! This crate provides database-backed state management for workflows with support for:
//! - Workflow state persistence (PostgreSQL and SQLite)
//! - Automatic checkpointing for recovery
//! - Connection pooling and transactions
//! - Workflow resumption after crashes
//!
//! # Examples
//!
//! ## PostgreSQL
//!
//! ```no_run
//! # use llm_orchestrator_state::{PostgresStateStore, StateStore, WorkflowState};
//! # use serde_json::json;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create PostgreSQL state store
//! let store = PostgresStateStore::new(
//!     "postgresql://user:pass@localhost/workflows",
//!     Some(5),  // min connections
//!     Some(20), // max connections
//! ).await?;
//!
//! // Create workflow state
//! let state = WorkflowState::new(
//!     "my-workflow",
//!     "My Workflow",
//!     Some("user-123".to_string()),
//!     json!({"inputs": {"key": "value"}}),
//! );
//!
//! // Save state
//! store.save_workflow_state(&state).await?;
//!
//! // Load state
//! let loaded = store.load_workflow_state(&state.id).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## SQLite
//!
//! ```no_run
//! # use llm_orchestrator_state::{SqliteStateStore, StateStore};
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create SQLite state store
//! let store = SqliteStateStore::new("./workflows.db").await?;
//!
//! // Use same API as PostgreSQL
//! store.health_check().await?;
//! # Ok(())
//! # }
//! ```

pub mod models;
pub mod postgres;
pub mod sqlite;
pub mod traits;

#[cfg(test)]
mod tests;

// Re-export commonly used types
pub use models::{Checkpoint, StepState, StepStatus, WorkflowState, WorkflowStatus};
pub use postgres::PostgresStateStore;
pub use sqlite::SqliteStateStore;
pub use traits::{StateStore, StateStoreError, StateStoreResult};

/// Library version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
