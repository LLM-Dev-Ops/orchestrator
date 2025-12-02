// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Canonical BenchmarkResult struct with standardized fields.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Canonical benchmark result structure.
///
/// This struct represents a single benchmark execution result with standardized
/// fields that are consistent across all 25 benchmark-target repositories.
///
/// # Fields
///
/// * `target_id` - Unique identifier for the benchmark target
/// * `metrics` - JSON object containing performance metrics (timing, memory, etc.)
/// * `timestamp` - UTC timestamp when the benchmark was executed
///
/// # Example
///
/// ```rust
/// use llm_orchestrator_benchmarks::benchmarks::result::BenchmarkResult;
/// use serde_json::json;
/// use chrono::Utc;
///
/// let result = BenchmarkResult {
///     target_id: "workflow_execution".to_string(),
///     metrics: json!({
///         "duration_ms": 150.5,
///         "steps_executed": 5,
///         "memory_bytes": 1024000
///     }),
///     timestamp: Utc::now(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Unique identifier for the benchmark target.
    ///
    /// This should be a descriptive name that identifies what operation
    /// or component was benchmarked (e.g., "workflow_execution", "dag_construction").
    pub target_id: String,

    /// JSON object containing performance metrics.
    ///
    /// Common metrics include:
    /// - `duration_ms`: Execution time in milliseconds
    /// - `iterations`: Number of iterations performed
    /// - `ops_per_sec`: Operations per second
    /// - `memory_bytes`: Memory usage in bytes
    /// - `p50_ms`, `p95_ms`, `p99_ms`: Latency percentiles
    pub metrics: Value,

    /// UTC timestamp when the benchmark was executed.
    pub timestamp: DateTime<Utc>,
}

impl BenchmarkResult {
    /// Creates a new BenchmarkResult with the given target ID and metrics.
    ///
    /// The timestamp is automatically set to the current UTC time.
    ///
    /// # Arguments
    ///
    /// * `target_id` - Unique identifier for the benchmark target
    /// * `metrics` - JSON value containing performance metrics
    ///
    /// # Example
    ///
    /// ```rust
    /// use llm_orchestrator_benchmarks::benchmarks::result::BenchmarkResult;
    /// use serde_json::json;
    ///
    /// let result = BenchmarkResult::new(
    ///     "my_benchmark",
    ///     json!({"duration_ms": 42.5}),
    /// );
    /// ```
    pub fn new(target_id: impl Into<String>, metrics: Value) -> Self {
        Self {
            target_id: target_id.into(),
            metrics,
            timestamp: Utc::now(),
        }
    }

    /// Creates a new BenchmarkResult with a specific timestamp.
    ///
    /// Useful for testing or when replaying historical benchmark data.
    pub fn with_timestamp(
        target_id: impl Into<String>,
        metrics: Value,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            target_id: target_id.into(),
            metrics,
            timestamp,
        }
    }

    /// Extracts the duration in milliseconds from metrics if present.
    pub fn duration_ms(&self) -> Option<f64> {
        self.metrics.get("duration_ms").and_then(|v| v.as_f64())
    }

    /// Extracts the iterations count from metrics if present.
    pub fn iterations(&self) -> Option<u64> {
        self.metrics.get("iterations").and_then(|v| v.as_u64())
    }

    /// Extracts operations per second from metrics if present.
    pub fn ops_per_sec(&self) -> Option<f64> {
        self.metrics.get("ops_per_sec").and_then(|v| v.as_f64())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_benchmark_result_creation() {
        let result = BenchmarkResult::new(
            "test_target",
            json!({
                "duration_ms": 100.5,
                "iterations": 1000
            }),
        );

        assert_eq!(result.target_id, "test_target");
        assert_eq!(result.duration_ms(), Some(100.5));
        assert_eq!(result.iterations(), Some(1000));
    }

    #[test]
    fn test_benchmark_result_serialization() {
        let result = BenchmarkResult::new("serialize_test", json!({"value": 42}));

        let json_str = serde_json::to_string(&result).expect("Failed to serialize");
        let deserialized: BenchmarkResult =
            serde_json::from_str(&json_str).expect("Failed to deserialize");

        assert_eq!(deserialized.target_id, result.target_id);
        assert_eq!(deserialized.metrics, result.metrics);
    }

    #[test]
    fn test_with_timestamp() {
        use chrono::TimeZone;

        let fixed_time = Utc.with_ymd_and_hms(2025, 1, 15, 12, 0, 0).unwrap();
        let result = BenchmarkResult::with_timestamp("timed_test", json!({}), fixed_time);

        assert_eq!(result.timestamp, fixed_time);
    }
}
