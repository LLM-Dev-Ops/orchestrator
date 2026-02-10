// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for FEU (Foundational Execution Unit) span instrumentation.
//!
//! Verifies the Core → Repo → Agent span hierarchy is correctly produced
//! when agents are invoked with a FeuSpanCollector.

use agentics_contracts::feu::{FeuExecutionContext, SpanStatus, SpanType};
use llm_orchestrator_core::feu_collector::FeuSpanCollector;
use llm_orchestrator_core::{
    AgentConfig, ExecuteRequest as AgentExecuteRequest, WorkflowOrchestratorAgent,
};
use llm_orchestrator_core::workflow::{
    LlmStepConfig, Step, StepConfig, StepType, Workflow,
};
use std::collections::HashMap;

/// Helper to create a valid FEU execution context.
fn test_feu_context() -> FeuExecutionContext {
    FeuExecutionContext {
        execution_id: "exec-integration-test".to_string(),
        parent_span_id: "core-span-123".to_string(),
        trace_id: Some("trace-abc".to_string()),
        metadata: HashMap::new(),
    }
}

/// Helper to create a simple workflow for testing.
fn test_workflow() -> Workflow {
    let mut workflow = Workflow::new("feu-test-workflow");
    workflow.version = "1.0".to_string();
    workflow.steps.push(Step {
        id: "step1".to_string(),
        step_type: StepType::Llm,
        depends_on: vec![],
        condition: None,
        config: StepConfig::Llm(LlmStepConfig {
            provider: "mock".to_string(),
            model: "gpt-4".to_string(),
            prompt: "Hello {{ name }}".to_string(),
            temperature: None,
            max_tokens: Some(50),
            system: None,
            stream: false,
            extra: HashMap::new(),
        }),
        output: vec!["greeting".to_string()],
        timeout_seconds: None,
        retry: None,
    });
    workflow
}

#[tokio::test]
async fn test_workflow_agent_feu_span_hierarchy() {
    // Create FEU context and collector
    let ctx = test_feu_context();
    let collector = FeuSpanCollector::new(ctx);

    // Verify repo span ID is set
    let repo_span_id = collector.repo_span_id().to_string();
    assert!(!repo_span_id.is_empty());

    // Create agent with FEU collector
    let config = AgentConfig::default();
    let agent = WorkflowOrchestratorAgent::new(config)
        .with_feu_collector(collector.clone());

    // Create execute request
    let workflow = test_workflow();
    let inputs = {
        let mut m = HashMap::new();
        m.insert("name".to_string(), serde_json::json!("World"));
        m
    };
    let request = AgentExecuteRequest { workflow, inputs };

    // Execute (will fail because no providers registered, but span should still be emitted)
    let _result = agent.execute(request).await;

    // Finalize and verify span hierarchy
    let result = collector.finalize(SpanStatus::Ok);

    // Repo span should reference the Core's parent_span_id
    assert_eq!(result.execution_id, "exec-integration-test");
    assert_eq!(result.repo_span.span_type, SpanType::Repo);
    assert_eq!(result.repo_span.parent_span_id, "core-span-123");
    assert_eq!(result.repo_span.repo_name, "llm-orchestrator");
    assert!(result.repo_span.end_time.is_some());

    // At least one agent span should have been emitted
    assert!(
        result.valid,
        "FEU result should be valid (at least one agent span emitted)"
    );
    assert!(
        !result.agent_spans.is_empty(),
        "Expected at least one agent-level span"
    );

    // Agent spans should reference the repo span as parent
    for span in &result.agent_spans {
        assert_eq!(span.span_type, SpanType::Agent);
        assert_eq!(span.parent_span_id, repo_span_id);
        assert_eq!(span.repo_name, "llm-orchestrator");
        assert!(span.agent_name.is_some());
        assert!(span.end_time.is_some());
    }

    // Validate the result structure
    assert!(result.validate().is_ok(), "FEU result should pass validation");
}

#[tokio::test]
async fn test_feu_result_serialization_roundtrip() {
    let ctx = test_feu_context();
    let collector = FeuSpanCollector::new(ctx);

    // Start and end an agent span
    let span_id = collector.start_agent_span("TestSerializationAgent");
    collector.end_agent_span(&span_id, SpanStatus::Ok, Vec::new(), Vec::new());

    let result = collector.finalize(SpanStatus::Ok);

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&result).expect("Serialization failed");

    // Deserialize back
    let deserialized: agentics_contracts::feu::RepoExecutionResult =
        serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(deserialized.execution_id, "exec-integration-test");
    assert_eq!(deserialized.repo_span.span_type, SpanType::Repo);
    assert_eq!(deserialized.agent_spans.len(), 1);
    assert_eq!(
        deserialized.agent_spans[0].agent_name,
        Some("TestSerializationAgent".to_string())
    );
    assert!(deserialized.valid);
    assert!(deserialized.validate().is_ok());
}

#[tokio::test]
async fn test_feu_context_validation_rejects_empty_fields() {
    // Empty execution_id
    let ctx = FeuExecutionContext {
        execution_id: "".to_string(),
        parent_span_id: "span-1".to_string(),
        trace_id: None,
        metadata: HashMap::new(),
    };
    assert!(ctx.validate().is_err());

    // Empty parent_span_id
    let ctx = FeuExecutionContext {
        execution_id: "exec-1".to_string(),
        parent_span_id: "".to_string(),
        trace_id: None,
        metadata: HashMap::new(),
    };
    assert!(ctx.validate().is_err());

    // Both valid
    let ctx = FeuExecutionContext {
        execution_id: "exec-1".to_string(),
        parent_span_id: "span-1".to_string(),
        trace_id: None,
        metadata: HashMap::new(),
    };
    assert!(ctx.validate().is_ok());
}

#[tokio::test]
async fn test_feu_result_invalid_without_agent_spans() {
    let ctx = test_feu_context();
    let collector = FeuSpanCollector::new(ctx);

    // Finalize without any agent spans
    let result = collector.finalize(SpanStatus::Ok);

    assert!(!result.valid);
    assert!(result.validate().is_err());
}

#[tokio::test]
async fn test_multiple_agents_produce_multiple_spans() {
    let ctx = test_feu_context();
    let collector = FeuSpanCollector::new(ctx);

    // Simulate multiple agents
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

    // Verify each agent span has the correct parent
    let repo_span_id = &result.repo_span.span_id;
    for span in &result.agent_spans {
        assert_eq!(&span.parent_span_id, repo_span_id);
    }

    assert!(result.validate().is_ok());
}

#[tokio::test]
async fn test_feu_collector_is_threadsafe() {
    use std::sync::Arc;

    let ctx = test_feu_context();
    let collector = FeuSpanCollector::new(ctx);
    let collector = Arc::new(collector);

    let mut handles = Vec::new();
    for i in 0..5 {
        let c = collector.clone();
        handles.push(tokio::spawn(async move {
            let name = format!("AsyncAgent{}", i);
            let span_id = c.start_agent_span(&name);
            // Simulate work
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            c.end_agent_span(&span_id, SpanStatus::Ok, Vec::new(), Vec::new());
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    // Clone the inner collector for finalization
    let inner = (*collector).clone();
    let result = inner.finalize(SpanStatus::Ok);

    assert_eq!(result.agent_spans.len(), 5);
    assert!(result.valid);
    assert!(result.validate().is_ok());
}
