// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Memory Graph adapter for lineage tracking and context history ingestion.
//!
//! This adapter provides a thin integration layer to consume lineage data
//! and context history from LLM-Memory-Graph without modifying core workflow logic.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Record of workflow lineage for tracking execution ancestry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageRecord {
    /// Unique identifier for this lineage record.
    pub id: Uuid,
    /// Workflow execution ID this lineage belongs to.
    pub workflow_id: Uuid,
    /// Step ID within the workflow.
    pub step_id: String,
    /// Parent lineage record ID (if any).
    pub parent_id: Option<Uuid>,
    /// Timestamp of the lineage event.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Type of lineage event (execution, transformation, etc.).
    pub event_type: String,
    /// Additional metadata for the lineage record.
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Entry in the context history for tracking execution state over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextHistoryEntry {
    /// Unique identifier for this history entry.
    pub id: Uuid,
    /// Workflow execution ID.
    pub workflow_id: Uuid,
    /// Step ID that produced this context.
    pub step_id: String,
    /// Sequence number in the execution order.
    pub sequence: u64,
    /// Snapshot of context variables at this point.
    pub context_snapshot: HashMap<String, serde_json::Value>,
    /// Timestamp of the context capture.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Adapter for consuming lineage and context history from LLM-Memory-Graph.
///
/// This adapter provides methods to ingest lineage records and context history
/// from the memory graph service for workflow execution tracking.
#[derive(Debug, Clone)]
pub struct MemoryGraphAdapter {
    /// Base URL for the memory graph service.
    endpoint: String,
    /// Whether the adapter is enabled.
    enabled: bool,
}

impl Default for MemoryGraphAdapter {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            enabled: false,
        }
    }
}

impl MemoryGraphAdapter {
    /// Creates a new memory graph adapter with the given endpoint.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            enabled: true,
        }
    }

    /// Creates a disabled adapter (no-op mode).
    pub fn disabled() -> Self {
        Self::default()
    }

    /// Returns whether the adapter is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Returns the configured endpoint.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Ingests a lineage record from the memory graph.
    ///
    /// This method consumes lineage data for workflow execution tracking
    /// without modifying workflow engine behavior.
    pub async fn ingest_lineage(&self, workflow_id: Uuid, step_id: &str) -> Option<LineageRecord> {
        if !self.enabled {
            return None;
        }

        // Placeholder: In production, this would call llm-memory-graph client
        // to retrieve lineage data for the given workflow and step.
        Some(LineageRecord {
            id: Uuid::new_v4(),
            workflow_id,
            step_id: step_id.to_string(),
            parent_id: None,
            timestamp: chrono::Utc::now(),
            event_type: "step_execution".to_string(),
            metadata: HashMap::new(),
        })
    }

    /// Retrieves context history for a workflow execution.
    ///
    /// Returns historical context snapshots for replay or analysis.
    pub async fn get_context_history(&self, workflow_id: Uuid) -> Vec<ContextHistoryEntry> {
        if !self.enabled {
            return Vec::new();
        }

        // Placeholder: In production, this would call llm-memory-graph client
        // to retrieve context history for the given workflow.
        Vec::new()
    }

    /// Records a new context snapshot to the memory graph.
    ///
    /// This captures the current execution context for historical tracking.
    pub async fn record_context(
        &self,
        workflow_id: Uuid,
        step_id: &str,
        sequence: u64,
        context: HashMap<String, serde_json::Value>,
    ) -> Option<ContextHistoryEntry> {
        if !self.enabled {
            return None;
        }

        // Placeholder: In production, this would call llm-memory-graph client
        // to store the context snapshot.
        Some(ContextHistoryEntry {
            id: Uuid::new_v4(),
            workflow_id,
            step_id: step_id.to_string(),
            sequence,
            context_snapshot: context,
            timestamp: chrono::Utc::now(),
        })
    }

    /// Queries lineage ancestors for a given step.
    pub async fn get_lineage_ancestors(&self, lineage_id: Uuid) -> Vec<LineageRecord> {
        if !self.enabled {
            return Vec::new();
        }

        // Placeholder: Would traverse the lineage graph upstream
        Vec::new()
    }

    /// Queries lineage descendants for a given step.
    pub async fn get_lineage_descendants(&self, lineage_id: Uuid) -> Vec<LineageRecord> {
        if !self.enabled {
            return Vec::new();
        }

        // Placeholder: Would traverse the lineage graph downstream
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_disabled_by_default() {
        let adapter = MemoryGraphAdapter::default();
        assert!(!adapter.is_enabled());
    }

    #[test]
    fn test_adapter_enabled_with_endpoint() {
        let adapter = MemoryGraphAdapter::new("http://localhost:8080");
        assert!(adapter.is_enabled());
        assert_eq!(adapter.endpoint(), "http://localhost:8080");
    }

    #[tokio::test]
    async fn test_disabled_adapter_returns_none() {
        let adapter = MemoryGraphAdapter::disabled();
        let result = adapter.ingest_lineage(Uuid::new_v4(), "step1").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_enabled_adapter_returns_lineage() {
        let adapter = MemoryGraphAdapter::new("http://localhost:8080");
        let workflow_id = Uuid::new_v4();
        let result = adapter.ingest_lineage(workflow_id, "step1").await;
        assert!(result.is_some());
        let record = result.unwrap();
        assert_eq!(record.workflow_id, workflow_id);
        assert_eq!(record.step_id, "step1");
    }
}
