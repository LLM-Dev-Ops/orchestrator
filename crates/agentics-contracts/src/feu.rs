// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Foundational Execution Unit (FEU) contract types.
//!
//! This module defines the NON-NEGOTIABLE contract for repos operating
//! as Foundational Execution Units within the Agentics execution system.
//!
//! # Invariants
//!
//! After instrumentation, the following span hierarchy MUST always hold:
//!
//! ```text
//! Core
//!   └─ Repo (this repo)
//!       └─ Agent (one or more)
//! ```
//!
//! - Every externally-invoked operation MUST receive an `FeuExecutionContext`
//!   with a valid `parent_span_id`.
//! - Execution MUST be rejected if `parent_span_id` is missing.
//! - At least one agent-level span MUST be emitted per execution.
//! - Artifacts MUST be attached at the agent level, never at the Core level.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The execution context passed from a Core to this FEU.
///
/// Contains the `execution_id` and `parent_span_id` that anchor
/// this repo's spans into the Core's ExecutionGraph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeuExecutionContext {
    /// Execution ID assigned by the Core.
    pub execution_id: String,

    /// The parent span ID from the Core. REQUIRED — execution is
    /// rejected if this is missing or empty.
    pub parent_span_id: String,

    /// Trace ID for the overall distributed trace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,

    /// Additional metadata from the Core.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl FeuExecutionContext {
    /// Validates that the context has the required fields.
    pub fn validate(&self) -> Result<(), crate::error::ContractError> {
        if self.execution_id.is_empty() {
            return Err(crate::error::ContractError::Validation(
                "FEU execution_id must not be empty".to_string(),
            ));
        }
        if self.parent_span_id.is_empty() {
            return Err(crate::error::ContractError::Validation(
                "FEU parent_span_id must not be empty".to_string(),
            ));
        }
        Ok(())
    }
}

/// Type of span in the Core → Repo → Agent hierarchy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanType {
    /// Repo-level span, parented under the Core span.
    Repo,
    /// Agent-level span, parented under the repo span.
    Agent,
}

/// Status of a span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanStatus {
    /// Span completed successfully.
    Ok,
    /// Span failed.
    Failed,
    /// Span is still running.
    Running,
}

/// A single execution span in the FEU hierarchy.
///
/// Used for both repo-level and agent-level spans.
/// Spans are append-only and causally ordered via `parent_span_id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSpan {
    /// The type of this span (repo or agent).
    pub span_type: SpanType,

    /// Unique span ID (UUID).
    pub span_id: String,

    /// Parent span ID. For repo spans this is the Core's span_id.
    /// For agent spans this is the repo span_id.
    pub parent_span_id: String,

    /// Repository name.
    pub repo_name: String,

    /// Agent name (only set for agent-level spans).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,

    /// Start time of the span (UTC).
    pub start_time: DateTime<Utc>,

    /// End time of the span (None if still running).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<DateTime<Utc>>,

    /// Status of this span.
    pub status: SpanStatus,

    /// Artifacts attached to this span.
    #[serde(default)]
    pub artifacts: Vec<SpanArtifact>,

    /// Machine-verifiable evidence attached to this span.
    #[serde(default)]
    pub evidence: Vec<serde_json::Value>,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// An artifact attached to a span.
///
/// Artifacts MUST include a stable reference (ID, URI, hash, or filename).
/// They MUST be attached at the agent or repo level only — never at Core.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanArtifact {
    /// Artifact identifier.
    pub id: String,
    /// Artifact type (e.g., "decision_event", "metric", "report").
    pub artifact_type: String,
    /// Storage location or URI.
    pub location: String,
    /// Content hash for integrity verification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
}

/// The output envelope for a FEU execution.
///
/// Contains the repo-level span with nested agent-level spans.
/// This is the complete span structure returned to the Core.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoExecutionResult {
    /// The execution_id from the Core.
    pub execution_id: String,

    /// The repo-level span.
    pub repo_span: ExecutionSpan,

    /// All agent-level spans emitted during this execution.
    pub agent_spans: Vec<ExecutionSpan>,

    /// Whether this execution is valid (at least one agent span emitted).
    pub valid: bool,
}

impl RepoExecutionResult {
    /// Validates the FEU output contract.
    ///
    /// Execution is INVALID if no agent-level spans were emitted.
    pub fn validate(&self) -> Result<(), crate::error::ContractError> {
        if self.agent_spans.is_empty() {
            return Err(crate::error::ContractError::Validation(
                "FEU execution invalid: no agent-level spans were emitted. \
                 At least one agent must execute and produce a span."
                    .to_string(),
            ));
        }
        // Verify all agent spans reference the repo span as parent
        for span in &self.agent_spans {
            if span.parent_span_id != self.repo_span.span_id {
                return Err(crate::error::ContractError::Validation(format!(
                    "Agent span {} has parent_span_id {} but repo span_id is {}",
                    span.span_id, span.parent_span_id, self.repo_span.span_id
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feu_context_validation() {
        let ctx = FeuExecutionContext {
            execution_id: "exec-123".to_string(),
            parent_span_id: "span-456".to_string(),
            trace_id: None,
            metadata: HashMap::new(),
        };
        assert!(ctx.validate().is_ok());
    }

    #[test]
    fn test_feu_context_rejects_empty_parent_span_id() {
        let ctx = FeuExecutionContext {
            execution_id: "exec-123".to_string(),
            parent_span_id: "".to_string(),
            trace_id: None,
            metadata: HashMap::new(),
        };
        assert!(ctx.validate().is_err());
    }

    #[test]
    fn test_feu_context_rejects_empty_execution_id() {
        let ctx = FeuExecutionContext {
            execution_id: "".to_string(),
            parent_span_id: "span-456".to_string(),
            trace_id: None,
            metadata: HashMap::new(),
        };
        assert!(ctx.validate().is_err());
    }

    #[test]
    fn test_execution_span_serialization() {
        let span = ExecutionSpan {
            span_type: SpanType::Agent,
            span_id: "agent-span-1".to_string(),
            parent_span_id: "repo-span-1".to_string(),
            repo_name: "llm-orchestrator".to_string(),
            agent_name: Some("WorkflowOrchestratorAgent".to_string()),
            start_time: Utc::now(),
            end_time: Some(Utc::now()),
            status: SpanStatus::Ok,
            artifacts: vec![SpanArtifact {
                id: "artifact-1".to_string(),
                artifact_type: "decision_event".to_string(),
                location: "ruvector://events/123".to_string(),
                content_hash: Some("abc123".to_string()),
            }],
            evidence: vec![serde_json::json!({"decision_id": "d-1"})],
            metadata: HashMap::new(),
        };

        let json = serde_json::to_string(&span).unwrap();
        let deserialized: ExecutionSpan = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.span_id, "agent-span-1");
        assert_eq!(deserialized.span_type, SpanType::Agent);
        assert_eq!(deserialized.agent_name, Some("WorkflowOrchestratorAgent".to_string()));
    }

    #[test]
    fn test_repo_execution_result_validates_agent_spans() {
        let result = RepoExecutionResult {
            execution_id: "exec-1".to_string(),
            repo_span: ExecutionSpan {
                span_type: SpanType::Repo,
                span_id: "repo-1".to_string(),
                parent_span_id: "core-1".to_string(),
                repo_name: "llm-orchestrator".to_string(),
                agent_name: None,
                start_time: Utc::now(),
                end_time: Some(Utc::now()),
                status: SpanStatus::Ok,
                artifacts: Vec::new(),
                evidence: Vec::new(),
                metadata: HashMap::new(),
            },
            agent_spans: Vec::new(),
            valid: false,
        };
        assert!(result.validate().is_err());
    }

    #[test]
    fn test_repo_execution_result_valid_with_agent_span() {
        let repo_span_id = "repo-1".to_string();
        let result = RepoExecutionResult {
            execution_id: "exec-1".to_string(),
            repo_span: ExecutionSpan {
                span_type: SpanType::Repo,
                span_id: repo_span_id.clone(),
                parent_span_id: "core-1".to_string(),
                repo_name: "llm-orchestrator".to_string(),
                agent_name: None,
                start_time: Utc::now(),
                end_time: Some(Utc::now()),
                status: SpanStatus::Ok,
                artifacts: Vec::new(),
                evidence: Vec::new(),
                metadata: HashMap::new(),
            },
            agent_spans: vec![ExecutionSpan {
                span_type: SpanType::Agent,
                span_id: "agent-1".to_string(),
                parent_span_id: repo_span_id,
                repo_name: "llm-orchestrator".to_string(),
                agent_name: Some("TestAgent".to_string()),
                start_time: Utc::now(),
                end_time: Some(Utc::now()),
                status: SpanStatus::Ok,
                artifacts: Vec::new(),
                evidence: Vec::new(),
                metadata: HashMap::new(),
            }],
            valid: true,
        };
        assert!(result.validate().is_ok());
    }

    #[test]
    fn test_repo_execution_result_detects_misparented_agent_span() {
        let result = RepoExecutionResult {
            execution_id: "exec-1".to_string(),
            repo_span: ExecutionSpan {
                span_type: SpanType::Repo,
                span_id: "repo-1".to_string(),
                parent_span_id: "core-1".to_string(),
                repo_name: "llm-orchestrator".to_string(),
                agent_name: None,
                start_time: Utc::now(),
                end_time: Some(Utc::now()),
                status: SpanStatus::Ok,
                artifacts: Vec::new(),
                evidence: Vec::new(),
                metadata: HashMap::new(),
            },
            agent_spans: vec![ExecutionSpan {
                span_type: SpanType::Agent,
                span_id: "agent-1".to_string(),
                parent_span_id: "wrong-parent".to_string(),
                repo_name: "llm-orchestrator".to_string(),
                agent_name: Some("TestAgent".to_string()),
                start_time: Utc::now(),
                end_time: Some(Utc::now()),
                status: SpanStatus::Ok,
                artifacts: Vec::new(),
                evidence: Vec::new(),
                metadata: HashMap::new(),
            }],
            valid: true,
        };
        assert!(result.validate().is_err());
    }

    #[test]
    fn test_feu_context_serialization_roundtrip() {
        let ctx = FeuExecutionContext {
            execution_id: "exec-abc".to_string(),
            parent_span_id: "span-xyz".to_string(),
            trace_id: Some("trace-999".to_string()),
            metadata: {
                let mut m = HashMap::new();
                m.insert("key".to_string(), serde_json::json!("value"));
                m
            },
        };

        let json = serde_json::to_string(&ctx).unwrap();
        let deserialized: FeuExecutionContext = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.execution_id, "exec-abc");
        assert_eq!(deserialized.parent_span_id, "span-xyz");
        assert_eq!(deserialized.trace_id, Some("trace-999".to_string()));
    }
}
