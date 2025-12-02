// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Benchmark target implementations for LLM Orchestrator operations.
//!
//! This module contains concrete implementations of the BenchTarget trait,
//! each measuring a specific orchestration operation.

use super::BenchTarget;
use crate::benchmarks::result::BenchmarkResult;
use async_trait::async_trait;
use llm_orchestrator_core::{
    ExecutionContext, Workflow, WorkflowDAG,
    workflow::{Step, StepType, StepConfig, TransformConfig, LlmStepConfig},
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Instant;

// ============================================================================
// Workflow DAG Construction Benchmark
// ============================================================================

/// Benchmark target for measuring DAG construction performance.
///
/// This benchmark measures the time to build a Directed Acyclic Graph
/// from a workflow definition, including cycle detection.
pub struct WorkflowDagConstructionBenchmark {
    iterations: usize,
}

impl WorkflowDagConstructionBenchmark {
    pub fn new() -> Self {
        Self { iterations: 100 }
    }

    fn create_test_workflow(step_count: usize) -> Workflow {
        let mut steps = Vec::with_capacity(step_count);

        for i in 0..step_count {
            let depends_on = if i > 0 {
                vec![format!("step_{}", i - 1)]
            } else {
                vec![]
            };

            steps.push(Step {
                id: format!("step_{}", i),
                step_type: StepType::Transform,
                depends_on,
                condition: None,
                config: StepConfig::Transform(TransformConfig {
                    function: "identity".to_string(),
                    inputs: vec![],
                    params: HashMap::new(),
                }),
                output: vec![format!("output_{}", i)],
                timeout_seconds: None,
                retry: None,
            });
        }

        Workflow {
            id: uuid::Uuid::new_v4(),
            name: "dag_benchmark_workflow".to_string(),
            version: "1.0".to_string(),
            description: Some("Benchmark workflow for DAG construction".to_string()),
            timeout_seconds: None,
            steps,
            metadata: HashMap::new(),
        }
    }
}

impl Default for WorkflowDagConstructionBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for WorkflowDagConstructionBenchmark {
    fn id(&self) -> &str {
        "workflow_dag_construction"
    }

    fn description(&self) -> &str {
        "Measures DAG construction and cycle detection performance"
    }

    async fn run(&self) -> BenchmarkResult {
        let workflow_small = Self::create_test_workflow(10);
        let workflow_medium = Self::create_test_workflow(50);
        let workflow_large = Self::create_test_workflow(100);

        // Benchmark small workflow
        let start_small = Instant::now();
        for _ in 0..self.iterations {
            let _ = WorkflowDAG::from_workflow(&workflow_small);
        }
        let duration_small = start_small.elapsed();

        // Benchmark medium workflow
        let start_medium = Instant::now();
        for _ in 0..self.iterations {
            let _ = WorkflowDAG::from_workflow(&workflow_medium);
        }
        let duration_medium = start_medium.elapsed();

        // Benchmark large workflow
        let start_large = Instant::now();
        for _ in 0..self.iterations {
            let _ = WorkflowDAG::from_workflow(&workflow_large);
        }
        let duration_large = start_large.elapsed();

        let total_duration = duration_small + duration_medium + duration_large;
        let ops_per_sec = (self.iterations * 3) as f64 / total_duration.as_secs_f64();

        BenchmarkResult::new(
            self.id(),
            json!({
                "duration_ms": total_duration.as_secs_f64() * 1000.0,
                "iterations": self.iterations * 3,
                "ops_per_sec": ops_per_sec,
                "small_workflow": {
                    "steps": 10,
                    "duration_ms": duration_small.as_secs_f64() * 1000.0,
                    "avg_ms": duration_small.as_secs_f64() * 1000.0 / self.iterations as f64
                },
                "medium_workflow": {
                    "steps": 50,
                    "duration_ms": duration_medium.as_secs_f64() * 1000.0,
                    "avg_ms": duration_medium.as_secs_f64() * 1000.0 / self.iterations as f64
                },
                "large_workflow": {
                    "steps": 100,
                    "duration_ms": duration_large.as_secs_f64() * 1000.0,
                    "avg_ms": duration_large.as_secs_f64() * 1000.0 / self.iterations as f64
                }
            }),
        )
    }
}

// ============================================================================
// Workflow Validation Benchmark
// ============================================================================

/// Benchmark target for measuring workflow schema validation performance.
pub struct WorkflowValidationBenchmark {
    iterations: usize,
}

impl WorkflowValidationBenchmark {
    pub fn new() -> Self {
        Self { iterations: 1000 }
    }

    fn create_valid_workflow() -> Workflow {
        Workflow {
            id: uuid::Uuid::new_v4(),
            name: "validation_test".to_string(),
            version: "1.0".to_string(),
            description: Some("Test workflow for validation".to_string()),
            timeout_seconds: Some(300),
            steps: vec![
                Step {
                    id: "step1".to_string(),
                    step_type: StepType::Llm,
                    depends_on: vec![],
                    condition: None,
                    config: StepConfig::Llm(LlmStepConfig {
                        provider: "openai".to_string(),
                        model: "gpt-4".to_string(),
                        prompt: "Hello {{ name }}".to_string(),
                        temperature: Some(0.7),
                        max_tokens: Some(100),
                        system: Some("You are helpful.".to_string()),
                        stream: false,
                        extra: HashMap::new(),
                    }),
                    output: vec!["response".to_string()],
                    timeout_seconds: Some(30),
                    retry: None,
                },
                Step {
                    id: "step2".to_string(),
                    step_type: StepType::Transform,
                    depends_on: vec!["step1".to_string()],
                    condition: Some("true".to_string()),
                    config: StepConfig::Transform(TransformConfig {
                        function: "format".to_string(),
                        inputs: vec!["response".to_string()],
                        params: HashMap::new(),
                    }),
                    output: vec!["formatted".to_string()],
                    timeout_seconds: None,
                    retry: None,
                },
            ],
            metadata: HashMap::new(),
        }
    }
}

impl Default for WorkflowValidationBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for WorkflowValidationBenchmark {
    fn id(&self) -> &str {
        "workflow_validation"
    }

    fn description(&self) -> &str {
        "Measures workflow schema validation performance"
    }

    async fn run(&self) -> BenchmarkResult {
        let workflow = Self::create_valid_workflow();

        let start = Instant::now();
        let mut success_count = 0;

        for _ in 0..self.iterations {
            if workflow.validate().is_ok() {
                success_count += 1;
            }
        }

        let duration = start.elapsed();
        let ops_per_sec = self.iterations as f64 / duration.as_secs_f64();

        BenchmarkResult::new(
            self.id(),
            json!({
                "duration_ms": duration.as_secs_f64() * 1000.0,
                "iterations": self.iterations,
                "ops_per_sec": ops_per_sec,
                "success_rate": success_count as f64 / self.iterations as f64,
                "avg_validation_us": duration.as_micros() as f64 / self.iterations as f64
            }),
        )
    }
}

// ============================================================================
// Parallel Step Coordination Benchmark
// ============================================================================

/// Benchmark target for measuring parallel pipeline coordination overhead.
///
/// This measures the overhead of coordinating parallel task execution
/// using DashMap and Tokio synchronization primitives.
pub struct ParallelStepCoordinationBenchmark {
    iterations: usize,
}

impl ParallelStepCoordinationBenchmark {
    pub fn new() -> Self {
        Self { iterations: 100 }
    }
}

impl Default for ParallelStepCoordinationBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for ParallelStepCoordinationBenchmark {
    fn id(&self) -> &str {
        "parallel_step_coordination"
    }

    fn description(&self) -> &str {
        "Measures parallel pipeline coordination overhead using DashMap"
    }

    async fn run(&self) -> BenchmarkResult {
        use dashmap::DashMap;
        use std::sync::Arc;
        use tokio::sync::Notify;

        let step_count = 20;
        let total_ops = self.iterations * step_count;

        let start = Instant::now();

        for _ in 0..self.iterations {
            let status_map: Arc<DashMap<String, String>> = Arc::new(DashMap::new());
            let notify = Arc::new(Notify::new());

            // Simulate parallel step status updates
            let mut handles = Vec::new();

            for i in 0..step_count {
                let map = status_map.clone();
                let n = notify.clone();

                handles.push(tokio::spawn(async move {
                    // Simulate step lifecycle
                    map.insert(format!("step_{}", i), "pending".to_string());
                    map.insert(format!("step_{}", i), "running".to_string());

                    // Minimal work simulation
                    tokio::task::yield_now().await;

                    map.insert(format!("step_{}", i), "completed".to_string());
                    n.notify_waiters();
                }));
            }

            // Wait for all to complete
            for handle in handles {
                let _ = handle.await;
            }
        }

        let duration = start.elapsed();
        let ops_per_sec = total_ops as f64 / duration.as_secs_f64();

        BenchmarkResult::new(
            self.id(),
            json!({
                "duration_ms": duration.as_secs_f64() * 1000.0,
                "iterations": self.iterations,
                "ops_per_sec": ops_per_sec,
                "parallel_steps_per_iteration": step_count,
                "total_step_operations": total_ops,
                "avg_coordination_overhead_us": duration.as_micros() as f64 / self.iterations as f64
            }),
        )
    }
}

// ============================================================================
// Context Template Rendering Benchmark
// ============================================================================

/// Benchmark target for measuring Handlebars template rendering performance.
pub struct ContextTemplateRenderingBenchmark {
    iterations: usize,
}

impl ContextTemplateRenderingBenchmark {
    pub fn new() -> Self {
        Self { iterations: 1000 }
    }
}

impl Default for ContextTemplateRenderingBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for ContextTemplateRenderingBenchmark {
    fn id(&self) -> &str {
        "context_template_rendering"
    }

    fn description(&self) -> &str {
        "Measures Handlebars template rendering performance in execution context"
    }

    async fn run(&self) -> BenchmarkResult {
        // Create context with various data types
        let mut inputs: HashMap<String, Value> = HashMap::new();
        inputs.insert("name".to_string(), json!("World"));
        inputs.insert("count".to_string(), json!(42));
        inputs.insert("items".to_string(), json!(["apple", "banana", "cherry"]));
        inputs.insert("nested".to_string(), json!({
            "key1": "value1",
            "key2": "value2"
        }));

        let context = ExecutionContext::new(inputs);

        // Various template complexities
        let templates = vec![
            ("simple", "Hello {{ inputs.name }}!"),
            ("multiple", "{{ inputs.name }} has {{ inputs.count }} items"),
            ("nested_access", "Key1 is {{ inputs.nested.key1 }}"),
        ];

        let start = Instant::now();
        let mut render_count = 0;

        for _ in 0..self.iterations {
            for (_, template) in &templates {
                if context.render_template(template).is_ok() {
                    render_count += 1;
                }
            }
        }

        let duration = start.elapsed();
        let total_renders = self.iterations * templates.len();
        let ops_per_sec = total_renders as f64 / duration.as_secs_f64();

        BenchmarkResult::new(
            self.id(),
            json!({
                "duration_ms": duration.as_secs_f64() * 1000.0,
                "iterations": self.iterations,
                "templates_tested": templates.len(),
                "total_renders": total_renders,
                "successful_renders": render_count,
                "ops_per_sec": ops_per_sec,
                "avg_render_us": duration.as_micros() as f64 / total_renders as f64
            }),
        )
    }
}

// ============================================================================
// Multi-Model Routing Benchmark
// ============================================================================

/// Benchmark target for measuring multi-model/provider routing performance.
///
/// This measures the overhead of the provider registry lookup and
/// routing decisions in multi-provider scenarios.
pub struct MultiModelRoutingBenchmark {
    iterations: usize,
}

impl MultiModelRoutingBenchmark {
    pub fn new() -> Self {
        Self { iterations: 10000 }
    }
}

impl Default for MultiModelRoutingBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for MultiModelRoutingBenchmark {
    fn id(&self) -> &str {
        "multi_model_routing"
    }

    fn description(&self) -> &str {
        "Measures multi-provider registry lookup and routing performance"
    }

    async fn run(&self) -> BenchmarkResult {
        use dashmap::DashMap;
        use std::sync::Arc;

        // Simulate provider registry with multiple providers
        let registry: Arc<DashMap<String, String>> = Arc::new(DashMap::new());

        // Register providers
        let providers = vec![
            "openai", "anthropic", "cohere", "mistral", "llama",
            "gemini", "palm", "claude", "gpt4", "gpt35"
        ];

        for provider in &providers {
            registry.insert(provider.to_string(), format!("{}_endpoint", provider));
        }

        let lookup_targets = vec![
            "openai", "anthropic", "unknown", "cohere", "gemini"
        ];

        let start = Instant::now();
        let mut hit_count = 0;
        let mut miss_count = 0;

        for _ in 0..self.iterations {
            for target in &lookup_targets {
                if registry.get(*target).is_some() {
                    hit_count += 1;
                } else {
                    miss_count += 1;
                }
            }
        }

        let duration = start.elapsed();
        let total_lookups = self.iterations * lookup_targets.len();
        let ops_per_sec = total_lookups as f64 / duration.as_secs_f64();

        BenchmarkResult::new(
            self.id(),
            json!({
                "duration_ms": duration.as_secs_f64() * 1000.0,
                "iterations": self.iterations,
                "total_lookups": total_lookups,
                "ops_per_sec": ops_per_sec,
                "cache_hits": hit_count,
                "cache_misses": miss_count,
                "hit_rate": hit_count as f64 / total_lookups as f64,
                "avg_lookup_ns": duration.as_nanos() as f64 / total_lookups as f64,
                "registered_providers": providers.len()
            }),
        )
    }
}

// ============================================================================
// Evaluation Feedback Loop Benchmark
// ============================================================================

/// Benchmark target for measuring evaluation/feedback loop speed.
///
/// This measures the overhead of retry logic, error handling, and
/// feedback mechanisms used in orchestration.
pub struct EvaluationFeedbackLoopBenchmark {
    iterations: usize,
}

impl EvaluationFeedbackLoopBenchmark {
    pub fn new() -> Self {
        Self { iterations: 100 }
    }
}

impl Default for EvaluationFeedbackLoopBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchTarget for EvaluationFeedbackLoopBenchmark {
    fn id(&self) -> &str {
        "evaluation_feedback_loop"
    }

    fn description(&self) -> &str {
        "Measures retry/feedback loop overhead in orchestration"
    }

    async fn run(&self) -> BenchmarkResult {
        use llm_orchestrator_core::retry::{RetryExecutor, RetryPolicy};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Duration;

        // Configure retry policy similar to production use
        let retry_policy = RetryPolicy::new(
            3,                              // max_attempts
            Duration::from_micros(100),     // initial_delay (very short for benchmark)
            2.0,                            // multiplier
            Duration::from_millis(1),       // max_delay
        );

        let success_count = Arc::new(AtomicUsize::new(0));
        let retry_count = Arc::new(AtomicUsize::new(0));

        let start = Instant::now();

        for i in 0..self.iterations {
            let executor = RetryExecutor::new(retry_policy.clone());
            let success_counter = success_count.clone();
            let retry_counter = retry_count.clone();
            let attempt = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let attempt_clone = attempt.clone();

            // Simulate operation that sometimes fails initially
            let result = executor.execute(|| {
                let attempt_clone = attempt_clone.clone();
                let retry_counter = retry_counter.clone();
                async move {
                    let current_attempt = attempt_clone.fetch_add(1, Ordering::SeqCst);

                    // Fail first attempt for every other iteration
                    if current_attempt == 0 && i % 2 == 0 {
                        retry_counter.fetch_add(1, Ordering::SeqCst);
                        Err(llm_orchestrator_core::OrchestratorError::other("Simulated failure"))
                    } else {
                        Ok(42)
                    }
                }
            }).await;

            if result.is_ok() {
                success_counter.fetch_add(1, Ordering::SeqCst);
            }
        }

        let duration = start.elapsed();
        let successes = success_count.load(Ordering::SeqCst);
        let retries = retry_count.load(Ordering::SeqCst);

        BenchmarkResult::new(
            self.id(),
            json!({
                "duration_ms": duration.as_secs_f64() * 1000.0,
                "iterations": self.iterations,
                "ops_per_sec": self.iterations as f64 / duration.as_secs_f64(),
                "successful_operations": successes,
                "retry_attempts": retries,
                "success_rate": successes as f64 / self.iterations as f64,
                "avg_loop_overhead_us": duration.as_micros() as f64 / self.iterations as f64
            }),
        )
    }
}

use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dag_construction_benchmark() {
        let benchmark = WorkflowDagConstructionBenchmark::new();
        let result = benchmark.run().await;

        assert_eq!(result.target_id, "workflow_dag_construction");
        assert!(result.duration_ms().is_some());
    }

    #[tokio::test]
    async fn test_validation_benchmark() {
        let benchmark = WorkflowValidationBenchmark::new();
        let result = benchmark.run().await;

        assert_eq!(result.target_id, "workflow_validation");
        assert!(result.ops_per_sec().is_some());
    }

    #[tokio::test]
    async fn test_parallel_coordination_benchmark() {
        let benchmark = ParallelStepCoordinationBenchmark::new();
        let result = benchmark.run().await;

        assert_eq!(result.target_id, "parallel_step_coordination");
        assert!(result.metrics.get("parallel_steps_per_iteration").is_some());
    }

    #[tokio::test]
    async fn test_template_rendering_benchmark() {
        let benchmark = ContextTemplateRenderingBenchmark::new();
        let result = benchmark.run().await;

        assert_eq!(result.target_id, "context_template_rendering");
        assert!(result.metrics.get("successful_renders").is_some());
    }

    #[tokio::test]
    async fn test_multi_model_routing_benchmark() {
        let benchmark = MultiModelRoutingBenchmark::new();
        let result = benchmark.run().await;

        assert_eq!(result.target_id, "multi_model_routing");
        assert!(result.metrics.get("hit_rate").is_some());
    }

    #[tokio::test]
    async fn test_feedback_loop_benchmark() {
        let benchmark = EvaluationFeedbackLoopBenchmark::new();
        let result = benchmark.run().await;

        assert_eq!(result.target_id, "evaluation_feedback_loop");
        assert!(result.metrics.get("success_rate").is_some());
    }
}
