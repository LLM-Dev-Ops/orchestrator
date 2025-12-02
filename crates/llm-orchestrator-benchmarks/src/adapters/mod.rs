// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Adapters module containing the canonical BenchTarget trait and target registry.
//!
//! This module provides the interface for defining benchmarkable operations
//! and a registry of all available benchmark targets.

mod targets;

use crate::benchmarks::result::BenchmarkResult;
use async_trait::async_trait;

pub use targets::*;

/// Canonical trait for benchmark targets.
///
/// Implement this trait to expose an operation as a benchmarkable target.
/// Each implementation should:
///
/// 1. Return a unique identifier via `id()`
/// 2. Execute the benchmark and return results via `run()`
///
/// # Example
///
/// ```rust
/// use llm_orchestrator_benchmarks::adapters::BenchTarget;
/// use llm_orchestrator_benchmarks::benchmarks::result::BenchmarkResult;
/// use async_trait::async_trait;
/// use serde_json::json;
///
/// struct MyBenchmark;
///
/// #[async_trait]
/// impl BenchTarget for MyBenchmark {
///     fn id(&self) -> &str {
///         "my_benchmark"
///     }
///
///     async fn run(&self) -> BenchmarkResult {
///         let start = std::time::Instant::now();
///
///         // Perform benchmarked operation
///         std::thread::sleep(std::time::Duration::from_millis(10));
///
///         let duration = start.elapsed();
///
///         BenchmarkResult::new(
///             self.id(),
///             json!({
///                 "duration_ms": duration.as_secs_f64() * 1000.0,
///                 "iterations": 1
///             })
///         )
///     }
/// }
/// ```
#[async_trait]
pub trait BenchTarget: Send + Sync {
    /// Returns the unique identifier for this benchmark target.
    ///
    /// This ID is used in result files and reports to identify the benchmark.
    fn id(&self) -> &str;

    /// Executes the benchmark and returns the results.
    ///
    /// Implementations should:
    /// - Perform any necessary setup
    /// - Execute the operation being benchmarked (potentially multiple iterations)
    /// - Collect timing and other metrics
    /// - Return a BenchmarkResult with the collected data
    async fn run(&self) -> BenchmarkResult;

    /// Returns a description of this benchmark target.
    ///
    /// Override this to provide more detailed information about what
    /// the benchmark measures.
    fn description(&self) -> &str {
        "No description provided"
    }
}

/// Returns all registered benchmark targets.
///
/// This function is the canonical registry for benchmark targets.
/// It returns a vector of boxed trait objects, each representing
/// a benchmarkable operation in the LLM Orchestrator.
///
/// # Registered Targets
///
/// - `workflow_dag_construction`: Measures DAG building performance
/// - `workflow_validation`: Measures workflow schema validation
/// - `parallel_step_coordination`: Measures parallel execution coordination
/// - `context_template_rendering`: Measures Handlebars template rendering
/// - `multi_model_routing`: Measures provider registry operations
/// - `evaluation_feedback_loop`: Measures retry/feedback loop overhead
///
/// # Example
///
/// ```rust
/// use llm_orchestrator_benchmarks::adapters::all_targets;
///
/// let targets = all_targets();
/// for target in &targets {
///     println!("Available benchmark: {}", target.id());
/// }
/// ```
pub fn all_targets() -> Vec<Box<dyn BenchTarget>> {
    vec![
        Box::new(WorkflowDagConstructionBenchmark::new()),
        Box::new(WorkflowValidationBenchmark::new()),
        Box::new(ParallelStepCoordinationBenchmark::new()),
        Box::new(ContextTemplateRenderingBenchmark::new()),
        Box::new(MultiModelRoutingBenchmark::new()),
        Box::new(EvaluationFeedbackLoopBenchmark::new()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_targets_registered() {
        let targets = all_targets();

        // Should have at least one target
        assert!(!targets.is_empty(), "Should have registered benchmark targets");

        // All targets should have unique IDs
        let ids: Vec<&str> = targets.iter().map(|t| t.id()).collect();
        let unique_ids: std::collections::HashSet<&str> = ids.iter().cloned().collect();
        assert_eq!(ids.len(), unique_ids.len(), "All target IDs should be unique");
    }

    #[tokio::test]
    async fn test_all_targets_runnable() {
        let targets = all_targets();

        for target in targets {
            let result = target.run().await;
            assert!(!result.target_id.is_empty(), "Target {} should have non-empty ID", target.id());
            assert!(result.metrics.is_object(), "Target {} should return object metrics", target.id());
        }
    }
}
