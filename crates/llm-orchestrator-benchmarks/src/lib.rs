// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Canonical benchmark suite for LLM Orchestrator.
//!
//! This crate provides the standard benchmark interface used across all
//! benchmark-target repositories, enabling consistent performance measurement
//! and reporting for orchestration operations.
//!
//! # Components
//!
//! - `benchmarks`: Core benchmark result types and I/O operations
//! - `adapters`: Benchmark target trait and registry for orchestration operations
//!
//! # Example
//!
//! ```rust,no_run
//! use llm_orchestrator_benchmarks::run_all_benchmarks;
//!
//! #[tokio::main]
//! async fn main() {
//!     let results = run_all_benchmarks().await;
//!     for result in &results {
//!         println!("{}: {:?}", result.target_id, result.metrics);
//!     }
//! }
//! ```

pub mod adapters;
pub mod benchmarks;

// Re-export commonly used types
pub use adapters::{all_targets, BenchTarget};
pub use benchmarks::{
    io::{write_raw_results, write_summary},
    markdown::generate_markdown_report,
    result::BenchmarkResult,
};

/// Runs all registered benchmark targets and returns their results.
///
/// This is the canonical entrypoint for the benchmark suite. It iterates
/// through all registered `BenchTarget` implementations, executes each one,
/// and collects the results into a `Vec<BenchmarkResult>`.
///
/// # Returns
///
/// A vector of `BenchmarkResult` containing the metrics and timing data
/// from each benchmark target.
///
/// # Example
///
/// ```rust,no_run
/// use llm_orchestrator_benchmarks::run_all_benchmarks;
///
/// #[tokio::main]
/// async fn main() {
///     let results = run_all_benchmarks().await;
///     println!("Completed {} benchmarks", results.len());
/// }
/// ```
pub async fn run_all_benchmarks() -> Vec<BenchmarkResult> {
    let targets = all_targets();
    let mut results = Vec::with_capacity(targets.len());

    for target in targets {
        let result = target.run().await;
        results.push(result);
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_all_benchmarks() {
        let results = run_all_benchmarks().await;
        // Should have at least one benchmark target registered
        assert!(!results.is_empty(), "Should have at least one benchmark target");

        // Verify all results have valid structure
        for result in &results {
            assert!(!result.target_id.is_empty(), "target_id should not be empty");
            assert!(result.metrics.is_object(), "metrics should be a JSON object");
        }
    }
}
