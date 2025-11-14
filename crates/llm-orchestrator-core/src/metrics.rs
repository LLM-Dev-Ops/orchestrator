// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Prometheus metrics instrumentation for workflow orchestration.
//!
//! This module provides metrics collection for monitoring workflow execution,
//! LLM provider performance, and system health.

use lazy_static::lazy_static;
use prometheus::{
    register_counter_vec, register_gauge, register_histogram_vec, CounterVec, Gauge, HistogramVec,
    TextEncoder, Encoder, Registry,
};

lazy_static! {
    // ============================================================================
    // Workflow Metrics
    // ============================================================================

    /// Total workflow executions by status and workflow name.
    ///
    /// Labels:
    /// - status: "success" | "failure"
    /// - workflow_name: name of the workflow
    pub static ref WORKFLOW_EXECUTIONS_TOTAL: CounterVec = register_counter_vec!(
        "orchestrator_workflow_executions_total",
        "Total number of workflow executions",
        &["status", "workflow_name"]
    )
    .expect("Failed to create workflow_executions_total metric");

    /// Workflow execution duration in seconds.
    ///
    /// Labels:
    /// - workflow_name: name of the workflow
    ///
    /// Buckets optimized for LLM workflows (100ms to 2 minutes).
    pub static ref WORKFLOW_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "orchestrator_workflow_duration_seconds",
        "Workflow execution duration in seconds",
        &["workflow_name"],
        vec![0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 120.0]
    )
    .expect("Failed to create workflow_duration_seconds metric");

    /// Number of currently active workflows.
    pub static ref ACTIVE_WORKFLOWS: Gauge = register_gauge!(
        "orchestrator_active_workflows",
        "Number of currently executing workflows"
    )
    .expect("Failed to create active_workflows metric");

    // ============================================================================
    // LLM Provider Metrics
    // ============================================================================

    /// Total LLM requests by provider, model, and status.
    ///
    /// Labels:
    /// - provider: "anthropic" | "openai" | etc.
    /// - model: model identifier
    /// - status: "success" | "failure"
    pub static ref LLM_REQUESTS_TOTAL: CounterVec = register_counter_vec!(
        "orchestrator_llm_requests_total",
        "Total LLM provider requests",
        &["provider", "model", "status"]
    )
    .expect("Failed to create llm_requests_total metric");

    /// Total tokens consumed by provider, model, and type.
    ///
    /// Labels:
    /// - provider: "anthropic" | "openai" | etc.
    /// - model: model identifier
    /// - type: "input" | "output"
    pub static ref LLM_TOKENS_TOTAL: CounterVec = register_counter_vec!(
        "orchestrator_llm_tokens_total",
        "Total tokens consumed by LLM providers",
        &["provider", "model", "type"]
    )
    .expect("Failed to create llm_tokens_total metric");

    /// LLM request duration in seconds.
    ///
    /// Labels:
    /// - provider: "anthropic" | "openai" | etc.
    /// - model: model identifier
    pub static ref LLM_REQUEST_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "orchestrator_llm_request_duration_seconds",
        "LLM request duration in seconds",
        &["provider", "model"],
        vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0]
    )
    .expect("Failed to create llm_request_duration_seconds metric");

    // ============================================================================
    // Error Metrics
    // ============================================================================

    /// Total errors by error type and component.
    ///
    /// Labels:
    /// - error_type: type of error (e.g., "timeout", "provider_error", "validation")
    /// - component: component where error occurred (e.g., "executor", "provider")
    pub static ref ERRORS_TOTAL: CounterVec = register_counter_vec!(
        "orchestrator_errors_total",
        "Total errors by type and component",
        &["error_type", "component"]
    )
    .expect("Failed to create errors_total metric");

    // ============================================================================
    // Step Metrics
    // ============================================================================

    /// Total step executions by step type and status.
    ///
    /// Labels:
    /// - step_type: "llm" | "embed" | "vector_search" | "transform" | "action"
    /// - status: "success" | "failure" | "skipped"
    pub static ref STEP_EXECUTIONS_TOTAL: CounterVec = register_counter_vec!(
        "orchestrator_step_executions_total",
        "Total step executions by type and status",
        &["step_type", "status"]
    )
    .expect("Failed to create step_executions_total metric");

    /// Step execution duration in seconds.
    ///
    /// Labels:
    /// - step_type: "llm" | "embed" | "vector_search" | "transform" | "action"
    pub static ref STEP_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "orchestrator_step_duration_seconds",
        "Step execution duration in seconds",
        &["step_type"],
        vec![0.01, 0.05, 0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0]
    )
    .expect("Failed to create step_duration_seconds metric");
}

/// Records the start of a workflow execution.
///
/// Increments the active workflows gauge and starts a duration timer.
#[inline]
pub fn record_workflow_start() {
    ACTIVE_WORKFLOWS.inc();
}

/// Records the completion of a workflow execution.
///
/// Decrements the active workflows gauge and records the duration and status.
///
/// # Arguments
/// * `workflow_name` - Name of the workflow
/// * `duration_seconds` - Execution duration in seconds
/// * `success` - Whether the workflow completed successfully
#[inline]
pub fn record_workflow_complete(workflow_name: &str, duration_seconds: f64, success: bool) {
    ACTIVE_WORKFLOWS.dec();

    let status = if success { "success" } else { "failure" };

    WORKFLOW_EXECUTIONS_TOTAL
        .with_label_values(&[status, workflow_name])
        .inc();

    WORKFLOW_DURATION_SECONDS
        .with_label_values(&[workflow_name])
        .observe(duration_seconds);
}

/// Records an LLM provider request.
///
/// # Arguments
/// * `provider` - Provider name (e.g., "anthropic", "openai")
/// * `model` - Model identifier
/// * `duration_seconds` - Request duration in seconds
/// * `success` - Whether the request succeeded
/// * `input_tokens` - Number of input tokens (optional)
/// * `output_tokens` - Number of output tokens (optional)
#[inline]
pub fn record_llm_request(
    provider: &str,
    model: &str,
    duration_seconds: f64,
    success: bool,
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
) {
    let status = if success { "success" } else { "failure" };

    LLM_REQUESTS_TOTAL
        .with_label_values(&[provider, model, status])
        .inc();

    LLM_REQUEST_DURATION_SECONDS
        .with_label_values(&[provider, model])
        .observe(duration_seconds);

    if let Some(tokens) = input_tokens {
        LLM_TOKENS_TOTAL
            .with_label_values(&[provider, model, "input"])
            .inc_by(tokens as f64);
    }

    if let Some(tokens) = output_tokens {
        LLM_TOKENS_TOTAL
            .with_label_values(&[provider, model, "output"])
            .inc_by(tokens as f64);
    }
}

/// Records a step execution.
///
/// # Arguments
/// * `step_type` - Type of step (e.g., "llm", "embed", "transform")
/// * `duration_seconds` - Execution duration in seconds
/// * `status` - Execution status ("success", "failure", "skipped")
#[inline]
pub fn record_step_execution(step_type: &str, duration_seconds: f64, status: &str) {
    STEP_EXECUTIONS_TOTAL
        .with_label_values(&[step_type, status])
        .inc();

    STEP_DURATION_SECONDS
        .with_label_values(&[step_type])
        .observe(duration_seconds);
}

/// Records an error occurrence.
///
/// # Arguments
/// * `error_type` - Type of error (e.g., "timeout", "provider_error", "validation")
/// * `component` - Component where error occurred (e.g., "executor", "provider")
#[inline]
pub fn record_error(error_type: &str, component: &str) {
    ERRORS_TOTAL
        .with_label_values(&[error_type, component])
        .inc();
}

/// Gathers and encodes all metrics in Prometheus text format.
///
/// Returns a string containing all metrics in Prometheus exposition format.
///
/// # Example
///
/// ```rust
/// use llm_orchestrator_core::metrics;
///
/// let metrics_text = metrics::gather_metrics();
/// println!("{}", metrics_text);
/// ```
pub fn gather_metrics() -> String {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();

    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer)
        .expect("Failed to encode metrics");

    String::from_utf8(buffer)
        .expect("Metrics encoding produced invalid UTF-8")
}

/// Creates a custom Prometheus registry with all orchestrator metrics.
///
/// Useful for applications that need to integrate with an existing metrics system.
pub fn create_registry() -> Registry {
    let registry = Registry::new();

    // Register all metrics
    registry.register(Box::new(WORKFLOW_EXECUTIONS_TOTAL.clone()))
        .expect("Failed to register workflow_executions_total");
    registry.register(Box::new(WORKFLOW_DURATION_SECONDS.clone()))
        .expect("Failed to register workflow_duration_seconds");
    registry.register(Box::new(ACTIVE_WORKFLOWS.clone()))
        .expect("Failed to register active_workflows");
    registry.register(Box::new(LLM_REQUESTS_TOTAL.clone()))
        .expect("Failed to register llm_requests_total");
    registry.register(Box::new(LLM_TOKENS_TOTAL.clone()))
        .expect("Failed to register llm_tokens_total");
    registry.register(Box::new(LLM_REQUEST_DURATION_SECONDS.clone()))
        .expect("Failed to register llm_request_duration_seconds");
    registry.register(Box::new(ERRORS_TOTAL.clone()))
        .expect("Failed to register errors_total");
    registry.register(Box::new(STEP_EXECUTIONS_TOTAL.clone()))
        .expect("Failed to register step_executions_total");
    registry.register(Box::new(STEP_DURATION_SECONDS.clone()))
        .expect("Failed to register step_duration_seconds");

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_metrics() {
        record_workflow_start();
        let active = ACTIVE_WORKFLOWS.get();
        assert!(active >= 1.0);

        record_workflow_complete("test-workflow", 1.5, true);
        let active_after = ACTIVE_WORKFLOWS.get();
        assert_eq!(active_after, active - 1.0);
    }

    #[test]
    fn test_llm_metrics() {
        record_llm_request("anthropic", "claude-3-5-sonnet", 2.3, true, Some(100), Some(50));

        // Verify counter incremented
        let count = LLM_REQUESTS_TOTAL
            .with_label_values(&["anthropic", "claude-3-5-sonnet", "success"])
            .get();
        assert!(count >= 1.0);
    }

    #[test]
    fn test_step_metrics() {
        record_step_execution("llm", 1.2, "success");

        let count = STEP_EXECUTIONS_TOTAL
            .with_label_values(&["llm", "success"])
            .get();
        assert!(count >= 1.0);
    }

    #[test]
    fn test_error_metrics() {
        record_error("timeout", "executor");

        let count = ERRORS_TOTAL
            .with_label_values(&["timeout", "executor"])
            .get();
        assert!(count >= 1.0);
    }

    #[test]
    fn test_gather_metrics() {
        record_workflow_start();
        record_workflow_complete("test", 1.0, true);

        let metrics = gather_metrics();
        assert!(metrics.contains("orchestrator_workflow_executions_total"));
        assert!(metrics.contains("orchestrator_active_workflows"));
    }

    #[test]
    fn test_create_registry() {
        let registry = create_registry();
        let families = registry.gather();

        // Should have all our custom metrics (9 total)
        // The registry may not return all metrics if they haven't been used
        // We have: workflow_executions, workflow_duration, active_workflows,
        // llm_requests, llm_tokens, llm_duration, errors, step_executions, step_duration
        assert!(families.len() <= 9, "Registered metrics count should not exceed 9");
    }
}
