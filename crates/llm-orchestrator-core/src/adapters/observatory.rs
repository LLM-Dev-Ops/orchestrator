// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Observatory adapter for consuming telemetry traces and runtime metrics.
//!
//! This adapter provides a thin integration layer to consume telemetry
//! services from LLM-Observatory without modifying core workflow logic.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

/// A telemetry event for workflow observability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEvent {
    /// Unique identifier for this event.
    pub id: Uuid,
    /// Event type (workflow_start, step_complete, error, etc.).
    pub event_type: String,
    /// Workflow execution ID.
    pub workflow_id: Uuid,
    /// Step ID (if applicable).
    pub step_id: Option<String>,
    /// Event timestamp.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Event severity (info, warn, error).
    pub severity: String,
    /// Event message.
    pub message: String,
    /// Event attributes.
    pub attributes: HashMap<String, serde_json::Value>,
    /// Trace context for distributed tracing.
    pub trace_id: Option<String>,
    /// Span context for distributed tracing.
    pub span_id: Option<String>,
}

/// Runtime metrics for workflow execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetrics {
    /// Workflow execution ID.
    pub workflow_id: Uuid,
    /// Total execution duration.
    pub total_duration: Duration,
    /// Number of steps executed.
    pub steps_executed: u32,
    /// Number of steps succeeded.
    pub steps_succeeded: u32,
    /// Number of steps failed.
    pub steps_failed: u32,
    /// Number of steps skipped.
    pub steps_skipped: u32,
    /// Total LLM tokens used.
    pub total_tokens: u64,
    /// Total LLM cost (in USD cents).
    pub total_cost_cents: u64,
    /// Average step latency.
    pub avg_step_latency: Duration,
    /// Peak memory usage (bytes).
    pub peak_memory_bytes: u64,
    /// Per-step metrics.
    pub step_metrics: HashMap<String, StepMetrics>,
}

/// Metrics for a single step execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepMetrics {
    /// Step ID.
    pub step_id: String,
    /// Step type.
    pub step_type: String,
    /// Execution duration.
    pub duration: Duration,
    /// Whether the step succeeded.
    pub success: bool,
    /// Retry count.
    pub retries: u32,
    /// Tokens used (for LLM steps).
    pub tokens: Option<u64>,
    /// Cost in USD cents (for LLM steps).
    pub cost_cents: Option<u64>,
    /// Custom metrics.
    pub custom: HashMap<String, f64>,
}

/// Adapter for consuming telemetry from LLM-Observatory.
///
/// This adapter enables the orchestrator to send and consume telemetry
/// data for observability and monitoring.
#[derive(Debug, Clone)]
pub struct ObservatoryAdapter {
    /// Base URL for the observatory service.
    endpoint: String,
    /// Whether to enable distributed tracing.
    tracing_enabled: bool,
    /// Whether to enable metrics collection.
    metrics_enabled: bool,
    /// Whether to enable event logging.
    events_enabled: bool,
    /// Sampling rate for traces (0.0 to 1.0).
    sampling_rate: f64,
    /// Whether the adapter is enabled.
    enabled: bool,
}

impl Default for ObservatoryAdapter {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            tracing_enabled: true,
            metrics_enabled: true,
            events_enabled: true,
            sampling_rate: 1.0,
            enabled: false,
        }
    }
}

impl ObservatoryAdapter {
    /// Creates a new observatory adapter with the given endpoint.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            tracing_enabled: true,
            metrics_enabled: true,
            events_enabled: true,
            sampling_rate: 1.0,
            enabled: true,
        }
    }

    /// Creates an adapter with custom configuration.
    pub fn with_config(
        endpoint: impl Into<String>,
        tracing: bool,
        metrics: bool,
        events: bool,
        sampling_rate: f64,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            tracing_enabled: tracing,
            metrics_enabled: metrics,
            events_enabled: events,
            sampling_rate: sampling_rate.clamp(0.0, 1.0),
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

    /// Emits a telemetry event.
    ///
    /// Sends the event to the observatory for storage and analysis.
    pub async fn emit_event(&self, event: TelemetryEvent) {
        if !self.enabled || !self.events_enabled {
            return;
        }

        // Placeholder: In production, this would send to llm-observatory-core
        tracing::debug!(
            event_type = %event.event_type,
            workflow_id = %event.workflow_id,
            "Emitted telemetry event"
        );
    }

    /// Emits a workflow start event.
    pub async fn emit_workflow_start(&self, workflow_id: Uuid, workflow_name: &str) {
        if !self.enabled || !self.events_enabled {
            return;
        }

        let event = TelemetryEvent {
            id: Uuid::new_v4(),
            event_type: "workflow_start".to_string(),
            workflow_id,
            step_id: None,
            timestamp: chrono::Utc::now(),
            severity: "info".to_string(),
            message: format!("Workflow '{}' started", workflow_name),
            attributes: HashMap::new(),
            trace_id: None,
            span_id: None,
        };

        self.emit_event(event).await;
    }

    /// Emits a workflow completion event.
    pub async fn emit_workflow_complete(
        &self,
        workflow_id: Uuid,
        success: bool,
        duration: Duration,
    ) {
        if !self.enabled || !self.events_enabled {
            return;
        }

        let mut attributes = HashMap::new();
        attributes.insert("success".to_string(), serde_json::json!(success));
        attributes.insert("duration_ms".to_string(), serde_json::json!(duration.as_millis()));

        let event = TelemetryEvent {
            id: Uuid::new_v4(),
            event_type: "workflow_complete".to_string(),
            workflow_id,
            step_id: None,
            timestamp: chrono::Utc::now(),
            severity: if success { "info" } else { "error" }.to_string(),
            message: format!(
                "Workflow completed {} in {:?}",
                if success { "successfully" } else { "with failures" },
                duration
            ),
            attributes,
            trace_id: None,
            span_id: None,
        };

        self.emit_event(event).await;
    }

    /// Emits a step completion event.
    pub async fn emit_step_complete(
        &self,
        workflow_id: Uuid,
        step_id: &str,
        success: bool,
        duration: Duration,
    ) {
        if !self.enabled || !self.events_enabled {
            return;
        }

        let mut attributes = HashMap::new();
        attributes.insert("success".to_string(), serde_json::json!(success));
        attributes.insert("duration_ms".to_string(), serde_json::json!(duration.as_millis()));

        let event = TelemetryEvent {
            id: Uuid::new_v4(),
            event_type: "step_complete".to_string(),
            workflow_id,
            step_id: Some(step_id.to_string()),
            timestamp: chrono::Utc::now(),
            severity: if success { "info" } else { "error" }.to_string(),
            message: format!(
                "Step '{}' completed {} in {:?}",
                step_id,
                if success { "successfully" } else { "with error" },
                duration
            ),
            attributes,
            trace_id: None,
            span_id: None,
        };

        self.emit_event(event).await;
    }

    /// Records workflow metrics.
    pub async fn record_metrics(&self, metrics: WorkflowMetrics) {
        if !self.enabled || !self.metrics_enabled {
            return;
        }

        // Placeholder: Would send metrics to observatory
        tracing::debug!(
            workflow_id = %metrics.workflow_id,
            steps_executed = metrics.steps_executed,
            total_duration_ms = metrics.total_duration.as_millis(),
            "Recorded workflow metrics"
        );
    }

    /// Records step metrics.
    pub async fn record_step_metrics(&self, workflow_id: Uuid, metrics: StepMetrics) {
        if !self.enabled || !self.metrics_enabled {
            return;
        }

        // Placeholder: Would send step metrics to observatory
        tracing::debug!(
            workflow_id = %workflow_id,
            step_id = %metrics.step_id,
            duration_ms = metrics.duration.as_millis(),
            "Recorded step metrics"
        );
    }

    /// Queries historical metrics for a workflow.
    pub async fn get_workflow_history(
        &self,
        workflow_name: &str,
        limit: usize,
    ) -> Vec<WorkflowMetrics> {
        if !self.enabled {
            return Vec::new();
        }

        // Placeholder: Would query observatory for historical data
        Vec::new()
    }

    /// Queries events for a workflow execution.
    pub async fn get_workflow_events(&self, workflow_id: Uuid) -> Vec<TelemetryEvent> {
        if !self.enabled {
            return Vec::new();
        }

        // Placeholder: Would query observatory for events
        Vec::new()
    }

    /// Starts a distributed trace span.
    pub async fn start_span(
        &self,
        workflow_id: Uuid,
        step_id: &str,
        parent_span_id: Option<&str>,
    ) -> Option<String> {
        if !self.enabled || !self.tracing_enabled {
            return None;
        }

        // Placeholder: Would create span via observatory
        Some(Uuid::new_v4().to_string())
    }

    /// Ends a distributed trace span.
    pub async fn end_span(&self, span_id: &str, success: bool) {
        if !self.enabled || !self.tracing_enabled {
            return;
        }

        // Placeholder: Would end span via observatory
        tracing::trace!(span_id = span_id, success = success, "Ended trace span");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_disabled_by_default() {
        let adapter = ObservatoryAdapter::default();
        assert!(!adapter.is_enabled());
    }

    #[test]
    fn test_adapter_enabled_with_endpoint() {
        let adapter = ObservatoryAdapter::new("http://localhost:8086");
        assert!(adapter.is_enabled());
    }

    #[test]
    fn test_sampling_rate_clamping() {
        let adapter = ObservatoryAdapter::with_config(
            "http://localhost:8086",
            true, true, true,
            1.5, // Should be clamped to 1.0
        );
        assert_eq!(adapter.sampling_rate, 1.0);
    }

    #[tokio::test]
    async fn test_emit_event_when_disabled() {
        let adapter = ObservatoryAdapter::disabled();
        let event = TelemetryEvent {
            id: Uuid::new_v4(),
            event_type: "test".to_string(),
            workflow_id: Uuid::new_v4(),
            step_id: None,
            timestamp: chrono::Utc::now(),
            severity: "info".to_string(),
            message: "Test event".to_string(),
            attributes: HashMap::new(),
            trace_id: None,
            span_id: None,
        };
        // Should not panic when disabled
        adapter.emit_event(event).await;
    }

    #[tokio::test]
    async fn test_emit_workflow_events() {
        let adapter = ObservatoryAdapter::new("http://localhost:8086");
        let workflow_id = Uuid::new_v4();

        adapter.emit_workflow_start(workflow_id, "test-workflow").await;
        adapter.emit_step_complete(workflow_id, "step1", true, Duration::from_millis(100)).await;
        adapter.emit_workflow_complete(workflow_id, true, Duration::from_millis(500)).await;
        // Should complete without errors
    }
}
