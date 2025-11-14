// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration tests for workflow execution.

use llm_orchestrator_core::providers::{
    CompletionRequest, CompletionResponse, LLMProvider, ProviderError,
};
use llm_orchestrator_core::workflow::{LlmStepConfig, StepConfig, StepType, Workflow};
use llm_orchestrator_core::{Step, WorkflowExecutor};
use std::collections::HashMap;
use std::sync::Arc;

/// Mock LLM provider for testing.
struct MockProvider {
    responses: HashMap<String, String>,
}

impl MockProvider {
    fn new() -> Self {
        let mut responses = HashMap::new();
        responses.insert("gpt-4".to_string(), "Hello from GPT-4!".to_string());
        responses.insert(
            "gpt-3.5-turbo".to_string(),
            "Hello from GPT-3.5!".to_string(),
        );
        Self { responses }
    }
}

#[async_trait::async_trait]
impl LLMProvider for MockProvider {
    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, ProviderError> {
        let text = self
            .responses
            .get(&request.model)
            .cloned()
            .unwrap_or_else(|| format!("Mock response for {}", request.model));

        let mut metadata = HashMap::new();
        metadata.insert(
            "usage".to_string(),
            serde_json::json!({
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15,
            }),
        );

        Ok(CompletionResponse {
            text,
            model: request.model,
            tokens_used: Some(15),
            metadata,
        })
    }

    fn name(&self) -> &str {
        "mock"
    }
}

#[tokio::test]
async fn test_simple_workflow_execution() {
    // Create a simple workflow with one LLM step
    let mut workflow = Workflow::new("test-workflow");
    workflow.version = "1.0".to_string();
    workflow.steps.push(Step {
        id: "greet".to_string(),
        step_type: StepType::Llm,
        depends_on: vec![],
        condition: None,
        config: StepConfig::Llm(LlmStepConfig {
            provider: "mock".to_string(),
            model: "gpt-4".to_string(),
            prompt: "Say hello".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(100),
            system: None,
            stream: false,
            extra: HashMap::new(),
        }),
        output: vec!["greeting".to_string()],
        timeout_seconds: None,
        retry: None,
    });

    // Create inputs
    let mut inputs = HashMap::new();
    inputs.insert("name".to_string(), serde_json::json!("World"));

    // Create executor
    let executor = WorkflowExecutor::new(workflow, inputs)
        .expect("Failed to create executor")
        .with_provider("mock", Arc::new(MockProvider::new()));

    // Execute workflow
    let results = executor
        .execute()
        .await
        .expect("Workflow execution failed");

    // Verify results
    assert_eq!(results.len(), 1);
    assert!(results.contains_key("greet"));

    let greet_result = &results["greet"];
    assert_eq!(greet_result.step_id, "greet");
    assert!(greet_result.outputs.contains_key("greeting"));
}

#[tokio::test]
async fn test_workflow_with_dependencies() {
    // Create a workflow with two steps where second depends on first
    let mut workflow = Workflow::new("dependent-workflow");
    workflow.version = "1.0".to_string();

    workflow.steps.push(Step {
        id: "step1".to_string(),
        step_type: StepType::Llm,
        depends_on: vec![],
        condition: None,
        config: StepConfig::Llm(LlmStepConfig {
            provider: "mock".to_string(),
            model: "gpt-4".to_string(),
            prompt: "Step 1".to_string(),
            temperature: None,
            max_tokens: Some(50),
            system: None,
            stream: false,
            extra: HashMap::new(),
        }),
        output: vec!["result1".to_string()],
        timeout_seconds: None,
        retry: None,
    });

    workflow.steps.push(Step {
        id: "step2".to_string(),
        step_type: StepType::Llm,
        depends_on: vec!["step1".to_string()],
        condition: None,
        config: StepConfig::Llm(LlmStepConfig {
            provider: "mock".to_string(),
            model: "gpt-3.5-turbo".to_string(),
            prompt: "Step 2 using {{steps.step1.result1}}".to_string(),
            temperature: None,
            max_tokens: Some(50),
            system: None,
            stream: false,
            extra: HashMap::new(),
        }),
        output: vec!["result2".to_string()],
        timeout_seconds: None,
        retry: None,
    });

    let inputs = HashMap::new();

    let executor = WorkflowExecutor::new(workflow, inputs)
        .expect("Failed to create executor")
        .with_provider("mock", Arc::new(MockProvider::new()));

    let results = executor
        .execute()
        .await
        .expect("Workflow execution failed");

    // Both steps should have executed
    assert_eq!(results.len(), 2);
    assert!(results.contains_key("step1"));
    assert!(results.contains_key("step2"));
}

#[tokio::test]
async fn test_workflow_with_parallel_steps() {
    // Create a workflow with parallel steps
    let mut workflow = Workflow::new("parallel-workflow");
    workflow.version = "1.0".to_string();

    // Three independent steps that can run in parallel
    for i in 1..=3 {
        workflow.steps.push(Step {
            id: format!("parallel_{}", i),
            step_type: StepType::Llm,
            depends_on: vec![],
            condition: None,
            config: StepConfig::Llm(LlmStepConfig {
                provider: "mock".to_string(),
                model: "gpt-4".to_string(),
                prompt: format!("Parallel step {}", i),
                temperature: None,
                max_tokens: Some(50),
                system: None,
                stream: false,
                extra: HashMap::new(),
            }),
            output: vec![format!("result{}", i)],
            timeout_seconds: None,
            retry: None,
        });
    }

    let inputs = HashMap::new();

    let executor = WorkflowExecutor::new(workflow, inputs)
        .expect("Failed to create executor")
        .with_max_concurrency(3)
        .with_provider("mock", Arc::new(MockProvider::new()));

    let results = executor
        .execute()
        .await
        .expect("Workflow execution failed");

    // All three steps should have executed
    assert_eq!(results.len(), 3);
    assert!(results.contains_key("parallel_1"));
    assert!(results.contains_key("parallel_2"));
    assert!(results.contains_key("parallel_3"));
}

#[tokio::test]
async fn test_workflow_with_condition() {
    // Create a workflow with a conditional step
    let mut workflow = Workflow::new("conditional-workflow");
    workflow.version = "1.0".to_string();

    workflow.steps.push(Step {
        id: "conditional".to_string(),
        step_type: StepType::Llm,
        depends_on: vec![],
        condition: Some("inputs.execute == true".to_string()),
        config: StepConfig::Llm(LlmStepConfig {
            provider: "mock".to_string(),
            model: "gpt-4".to_string(),
            prompt: "Conditional step".to_string(),
            temperature: None,
            max_tokens: Some(50),
            system: None,
            stream: false,
            extra: HashMap::new(),
        }),
        output: vec!["result".to_string()],
        timeout_seconds: None,
        retry: None,
    });

    // Test with condition true
    let mut inputs = HashMap::new();
    inputs.insert("execute".to_string(), serde_json::json!(true));

    let executor = WorkflowExecutor::new(workflow.clone(), inputs)
        .expect("Failed to create executor")
        .with_provider("mock", Arc::new(MockProvider::new()));

    let results = executor
        .execute()
        .await
        .expect("Workflow execution failed");

    assert_eq!(results.len(), 1);
    assert!(results.contains_key("conditional"));

    // Test with condition false
    let mut inputs = HashMap::new();
    inputs.insert("execute".to_string(), serde_json::json!(false));

    let executor = WorkflowExecutor::new(workflow, inputs)
        .expect("Failed to create executor")
        .with_provider("mock", Arc::new(MockProvider::new()));

    let results = executor
        .execute()
        .await
        .expect("Workflow execution failed");

    // Step should be skipped
    assert_eq!(results.len(), 1);
    let result = &results["conditional"];
    assert_eq!(
        result.status,
        llm_orchestrator_core::executor::StepStatus::Skipped
    );
}
