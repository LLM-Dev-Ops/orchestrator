// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Simulator adapter for offline workflow step simulation.
//!
//! This adapter provides a thin integration layer to consume simulation
//! services from LLM-Simulator without modifying core workflow logic.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

/// Configuration for workflow simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    /// Whether to simulate LLM responses.
    pub simulate_llm: bool,
    /// Whether to simulate embeddings.
    pub simulate_embeddings: bool,
    /// Whether to simulate vector searches.
    pub simulate_vector_search: bool,
    /// Simulated latency range (min, max) in milliseconds.
    pub latency_range_ms: (u64, u64),
    /// Failure rate for testing error handling (0.0 to 1.0).
    pub failure_rate: f64,
    /// Seed for reproducible simulations.
    pub seed: Option<u64>,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            simulate_llm: true,
            simulate_embeddings: true,
            simulate_vector_search: true,
            latency_range_ms: (50, 200),
            failure_rate: 0.0,
            seed: None,
        }
    }
}

/// Result of a simulated step execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    /// Step ID that was simulated.
    pub step_id: String,
    /// Whether the simulation succeeded.
    pub success: bool,
    /// Simulated output data.
    pub output: HashMap<String, serde_json::Value>,
    /// Simulated latency.
    pub latency: Duration,
    /// Error message if simulation failed.
    pub error: Option<String>,
    /// Simulation metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Adapter for consuming simulation services from LLM-Simulator.
///
/// This adapter enables offline testing and development by simulating
/// workflow step executions without calling real providers.
#[derive(Debug, Clone)]
pub struct SimulatorAdapter {
    /// Base URL for the simulator service.
    endpoint: String,
    /// Simulation configuration.
    config: SimulationConfig,
    /// Whether the adapter is enabled.
    enabled: bool,
}

impl Default for SimulatorAdapter {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            config: SimulationConfig::default(),
            enabled: false,
        }
    }
}

impl SimulatorAdapter {
    /// Creates a new simulator adapter with the given endpoint.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            config: SimulationConfig::default(),
            enabled: true,
        }
    }

    /// Creates an adapter with custom simulation configuration.
    pub fn with_config(endpoint: impl Into<String>, config: SimulationConfig) -> Self {
        Self {
            endpoint: endpoint.into(),
            config,
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

    /// Returns the simulation configuration.
    pub fn config(&self) -> &SimulationConfig {
        &self.config
    }

    /// Simulates an LLM completion step.
    pub async fn simulate_llm_step(
        &self,
        step_id: &str,
        prompt: &str,
        model: &str,
    ) -> Option<SimulationResult> {
        if !self.enabled || !self.config.simulate_llm {
            return None;
        }

        // Placeholder: In production, this would call llm-simulator client
        let latency = Duration::from_millis(self.config.latency_range_ms.0);

        let mut output = HashMap::new();
        output.insert(
            "response".to_string(),
            serde_json::json!(format!("[SIMULATED] Response for: {}", prompt)),
        );
        output.insert("model".to_string(), serde_json::json!(model));

        Some(SimulationResult {
            step_id: step_id.to_string(),
            success: true,
            output,
            latency,
            error: None,
            metadata: HashMap::new(),
        })
    }

    /// Simulates an embedding step.
    pub async fn simulate_embed_step(
        &self,
        step_id: &str,
        input: &str,
        dimensions: usize,
    ) -> Option<SimulationResult> {
        if !self.enabled || !self.config.simulate_embeddings {
            return None;
        }

        let latency = Duration::from_millis(self.config.latency_range_ms.0);

        // Generate a deterministic fake embedding
        let embedding: Vec<f32> = (0..dimensions).map(|i| (i as f32 * 0.01) % 1.0).collect();

        let mut output = HashMap::new();
        output.insert("embedding".to_string(), serde_json::json!(embedding));

        Some(SimulationResult {
            step_id: step_id.to_string(),
            success: true,
            output,
            latency,
            error: None,
            metadata: HashMap::new(),
        })
    }

    /// Simulates a vector search step.
    pub async fn simulate_vector_search(
        &self,
        step_id: &str,
        query_vector: &[f32],
        top_k: usize,
    ) -> Option<SimulationResult> {
        if !self.enabled || !self.config.simulate_vector_search {
            return None;
        }

        let latency = Duration::from_millis(self.config.latency_range_ms.0);

        // Generate fake search results
        let results: Vec<serde_json::Value> = (0..top_k)
            .map(|i| {
                serde_json::json!({
                    "id": format!("sim_doc_{}", i),
                    "score": 0.9 - (i as f64 * 0.1),
                    "metadata": {
                        "text": format!("[SIMULATED] Document {} content", i),
                        "source": "simulator"
                    }
                })
            })
            .collect();

        let mut output = HashMap::new();
        output.insert("results".to_string(), serde_json::json!(results));

        Some(SimulationResult {
            step_id: step_id.to_string(),
            success: true,
            output,
            latency,
            error: None,
            metadata: HashMap::new(),
        })
    }

    /// Simulates a generic workflow step.
    pub async fn simulate_step(
        &self,
        step_id: &str,
        step_type: &str,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Option<SimulationResult> {
        if !self.enabled {
            return None;
        }

        let latency = Duration::from_millis(self.config.latency_range_ms.0);

        let mut output = HashMap::new();
        output.insert(
            "simulated".to_string(),
            serde_json::json!(true),
        );
        output.insert(
            "step_type".to_string(),
            serde_json::json!(step_type),
        );

        Some(SimulationResult {
            step_id: step_id.to_string(),
            success: true,
            output,
            latency,
            error: None,
            metadata: HashMap::new(),
        })
    }

    /// Records simulation metrics for analysis.
    pub async fn record_simulation_metrics(
        &self,
        workflow_id: Uuid,
        results: &[SimulationResult],
    ) {
        if !self.enabled {
            return;
        }

        // Placeholder: Would record metrics to simulator service
        tracing::debug!(
            workflow_id = %workflow_id,
            step_count = results.len(),
            "Recorded simulation metrics"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_disabled_by_default() {
        let adapter = SimulatorAdapter::default();
        assert!(!adapter.is_enabled());
    }

    #[test]
    fn test_adapter_enabled_with_endpoint() {
        let adapter = SimulatorAdapter::new("http://localhost:8083");
        assert!(adapter.is_enabled());
    }

    #[test]
    fn test_default_simulation_config() {
        let config = SimulationConfig::default();
        assert!(config.simulate_llm);
        assert!(config.simulate_embeddings);
        assert_eq!(config.failure_rate, 0.0);
    }

    #[tokio::test]
    async fn test_simulate_llm_when_disabled() {
        let adapter = SimulatorAdapter::disabled();
        let result = adapter.simulate_llm_step("step1", "test", "gpt-4").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_simulate_llm_when_enabled() {
        let adapter = SimulatorAdapter::new("http://localhost:8083");
        let result = adapter.simulate_llm_step("step1", "test prompt", "gpt-4").await;
        assert!(result.is_some());
        let result = result.unwrap();
        assert!(result.success);
        assert!(result.output.contains_key("response"));
    }

    #[tokio::test]
    async fn test_simulate_embed() {
        let adapter = SimulatorAdapter::new("http://localhost:8083");
        let result = adapter.simulate_embed_step("step1", "test text", 384).await;
        assert!(result.is_some());
        let result = result.unwrap();
        assert!(result.success);
        assert!(result.output.contains_key("embedding"));
    }
}
