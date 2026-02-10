// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! FEU Span Collector — runtime component for collecting execution spans.
//!
//! The `FeuSpanCollector` is a thread-safe, append-only collector that
//! accumulates agent-level spans during execution and produces the
//! final `RepoExecutionResult` envelope.

use agentics_contracts::feu::{
    ExecutionSpan, FeuExecutionContext, RepoExecutionResult, SpanArtifact, SpanStatus, SpanType,
};
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Repository name used in all spans emitted by this FEU.
pub const REPO_NAME: &str = "llm-orchestrator";

/// Thread-safe collector for FEU execution spans.
///
/// Created at the entry point of a FEU execution, threaded through
/// agents, and finalized to produce the `RepoExecutionResult`.
#[derive(Debug, Clone)]
pub struct FeuSpanCollector {
    /// The execution context from the Core.
    ctx: FeuExecutionContext,
    /// The repo-level span ID (generated at creation).
    repo_span_id: String,
    /// The start time of the repo span.
    repo_start_time: DateTime<Utc>,
    /// Accumulated agent-level spans.
    spans: Arc<Mutex<Vec<ExecutionSpan>>>,
}

impl FeuSpanCollector {
    /// Creates a new collector and records the repo-level span start time.
    ///
    /// The `ctx` must have been validated before calling this.
    pub fn new(ctx: FeuExecutionContext) -> Self {
        Self {
            ctx,
            repo_span_id: Uuid::new_v4().to_string(),
            repo_start_time: Utc::now(),
            spans: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Returns the repo span_id.
    ///
    /// Agents use this as their `parent_span_id`.
    pub fn repo_span_id(&self) -> &str {
        &self.repo_span_id
    }

    /// Returns the execution context.
    pub fn context(&self) -> &FeuExecutionContext {
        &self.ctx
    }

    /// Starts an agent-level span and returns its span_id.
    ///
    /// The span is recorded immediately with `status: Running`.
    /// Call `end_agent_span` when the agent completes.
    pub fn start_agent_span(&self, agent_name: &str) -> String {
        let span_id = Uuid::new_v4().to_string();
        let span = ExecutionSpan {
            span_type: SpanType::Agent,
            span_id: span_id.clone(),
            parent_span_id: self.repo_span_id.clone(),
            repo_name: REPO_NAME.to_string(),
            agent_name: Some(agent_name.to_string()),
            start_time: Utc::now(),
            end_time: None,
            status: SpanStatus::Running,
            artifacts: Vec::new(),
            evidence: Vec::new(),
            metadata: HashMap::new(),
        };
        self.spans.lock().push(span);
        span_id
    }

    /// Ends an agent-level span by span_id.
    ///
    /// Sets the end_time, status, artifacts, and evidence.
    pub fn end_agent_span(
        &self,
        span_id: &str,
        status: SpanStatus,
        artifacts: Vec<SpanArtifact>,
        evidence: Vec<serde_json::Value>,
    ) {
        let mut spans = self.spans.lock();
        if let Some(span) = spans.iter_mut().find(|s| s.span_id == span_id) {
            span.end_time = Some(Utc::now());
            span.status = status;
            span.artifacts = artifacts;
            span.evidence = evidence;
        }
    }

    /// Finalizes the execution, building the `RepoExecutionResult`.
    ///
    /// Constructs the repo-level span and collects all agent-level spans.
    /// The result includes `valid = true` only if at least one agent span
    /// was emitted.
    pub fn finalize(self, repo_status: SpanStatus) -> RepoExecutionResult {
        let agent_spans = self.spans.lock().clone();

        let repo_span = ExecutionSpan {
            span_type: SpanType::Repo,
            span_id: self.repo_span_id.clone(),
            parent_span_id: self.ctx.parent_span_id.clone(),
            repo_name: REPO_NAME.to_string(),
            agent_name: None,
            start_time: self.repo_start_time,
            end_time: Some(Utc::now()),
            status: repo_status,
            artifacts: Vec::new(),
            evidence: Vec::new(),
            metadata: self.ctx.metadata.clone(),
        };

        let valid = !agent_spans.is_empty();

        RepoExecutionResult {
            execution_id: self.ctx.execution_id.clone(),
            repo_span,
            agent_spans,
            valid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ctx() -> FeuExecutionContext {
        FeuExecutionContext {
            execution_id: "exec-test".to_string(),
            parent_span_id: "core-span-1".to_string(),
            trace_id: Some("trace-1".to_string()),
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_collector_creates_repo_span_id() {
        let collector = FeuSpanCollector::new(test_ctx());
        assert!(!collector.repo_span_id().is_empty());
    }

    #[test]
    fn test_agent_span_lifecycle() {
        let collector = FeuSpanCollector::new(test_ctx());

        let span_id = collector.start_agent_span("TestAgent");
        assert!(!span_id.is_empty());

        // Verify span is in running state
        {
            let spans = collector.spans.lock();
            assert_eq!(spans.len(), 1);
            assert_eq!(spans[0].status, SpanStatus::Running);
            assert_eq!(spans[0].agent_name, Some("TestAgent".to_string()));
            assert_eq!(spans[0].parent_span_id, collector.repo_span_id().to_string());
        }

        // End the span
        collector.end_agent_span(
            &span_id,
            SpanStatus::Ok,
            vec![SpanArtifact {
                id: "a1".to_string(),
                artifact_type: "decision_event".to_string(),
                location: "ruvector://events/1".to_string(),
                content_hash: None,
            }],
            vec![serde_json::json!({"decision_id": "d-1"})],
        );

        {
            let spans = collector.spans.lock();
            assert_eq!(spans[0].status, SpanStatus::Ok);
            assert!(spans[0].end_time.is_some());
            assert_eq!(spans[0].artifacts.len(), 1);
            assert_eq!(spans[0].evidence.len(), 1);
        }
    }

    #[test]
    fn test_finalize_produces_valid_result() {
        let collector = FeuSpanCollector::new(test_ctx());
        let span_id = collector.start_agent_span("Agent1");
        collector.end_agent_span(&span_id, SpanStatus::Ok, Vec::new(), Vec::new());

        let result = collector.finalize(SpanStatus::Ok);

        assert!(result.valid);
        assert_eq!(result.execution_id, "exec-test");
        assert_eq!(result.repo_span.span_type, SpanType::Repo);
        assert_eq!(result.repo_span.parent_span_id, "core-span-1");
        assert_eq!(result.agent_spans.len(), 1);
        assert_eq!(result.agent_spans[0].span_type, SpanType::Agent);
        assert!(result.validate().is_ok());
    }

    #[test]
    fn test_finalize_without_agents_is_invalid() {
        let collector = FeuSpanCollector::new(test_ctx());
        let result = collector.finalize(SpanStatus::Ok);

        assert!(!result.valid);
        assert!(result.validate().is_err());
    }

    #[test]
    fn test_multiple_agent_spans() {
        let collector = FeuSpanCollector::new(test_ctx());

        let s1 = collector.start_agent_span("Agent1");
        let s2 = collector.start_agent_span("Agent2");
        let s3 = collector.start_agent_span("Agent3");

        collector.end_agent_span(&s1, SpanStatus::Ok, Vec::new(), Vec::new());
        collector.end_agent_span(&s2, SpanStatus::Failed, Vec::new(), Vec::new());
        collector.end_agent_span(&s3, SpanStatus::Ok, Vec::new(), Vec::new());

        let result = collector.finalize(SpanStatus::Failed);

        assert!(result.valid);
        assert_eq!(result.agent_spans.len(), 3);
        assert_eq!(result.repo_span.status, SpanStatus::Failed);
    }

    #[test]
    fn test_concurrent_span_collection() {
        use std::thread;

        let collector = FeuSpanCollector::new(test_ctx());
        let mut handles = Vec::new();

        for i in 0..10 {
            let c = collector.clone();
            handles.push(thread::spawn(move || {
                let name = format!("Agent{}", i);
                let span_id = c.start_agent_span(&name);
                c.end_agent_span(&span_id, SpanStatus::Ok, Vec::new(), Vec::new());
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let result = collector.finalize(SpanStatus::Ok);
        assert_eq!(result.agent_spans.len(), 10);
        assert!(result.valid);
    }

    #[test]
    fn test_result_is_json_serializable() {
        let collector = FeuSpanCollector::new(test_ctx());
        let span_id = collector.start_agent_span("SerializationAgent");
        collector.end_agent_span(&span_id, SpanStatus::Ok, Vec::new(), Vec::new());

        let result = collector.finalize(SpanStatus::Ok);
        let json = serde_json::to_string_pretty(&result).unwrap();
        let deserialized: RepoExecutionResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.execution_id, result.execution_id);
        assert_eq!(deserialized.agent_spans.len(), 1);
    }
}
