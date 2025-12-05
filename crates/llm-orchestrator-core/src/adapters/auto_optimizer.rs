// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Auto-Optimizer adapter for consuming optimization recommendations.
//!
//! This adapter provides a thin integration layer to consume optimization
//! and self-correction parameters from LLM-Auto-Optimizer without modifying
//! core workflow logic.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// An optimization recommendation from the auto-optimizer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    /// Unique identifier for this recommendation.
    pub id: Uuid,
    /// Type of optimization (latency, cost, quality, throughput).
    pub optimization_type: String,
    /// Target of the optimization (step_id, provider, model, etc.).
    pub target: String,
    /// Current value/setting.
    pub current_value: serde_json::Value,
    /// Recommended value/setting.
    pub recommended_value: serde_json::Value,
    /// Expected improvement percentage.
    pub expected_improvement: f64,
    /// Confidence level (0.0 to 1.0).
    pub confidence: f64,
    /// Reasoning for the recommendation.
    pub reasoning: String,
    /// Whether this recommendation requires human approval.
    pub requires_approval: bool,
}

/// Parameters for self-correction behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionParams {
    /// Whether self-correction is enabled.
    pub enabled: bool,
    /// Maximum correction attempts per step.
    pub max_attempts: u32,
    /// Threshold for triggering correction (error rate, latency, etc.).
    pub threshold: f64,
    /// Correction strategy (retry, fallback, adapt).
    pub strategy: String,
    /// Cooldown period between corrections (in seconds).
    pub cooldown_seconds: u64,
    /// Parameters specific to the correction strategy.
    pub strategy_params: HashMap<String, serde_json::Value>,
}

impl Default for CorrectionParams {
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: 3,
            threshold: 0.1,
            strategy: "retry".to_string(),
            cooldown_seconds: 60,
            strategy_params: HashMap::new(),
        }
    }
}

/// Adapter for consuming optimization recommendations from LLM-Auto-Optimizer.
///
/// This adapter enables the orchestrator to consume and apply optimization
/// recommendations and self-correction parameters dynamically.
#[derive(Debug, Clone)]
pub struct AutoOptimizerAdapter {
    /// Base URL for the auto-optimizer service.
    endpoint: String,
    /// Default correction parameters.
    default_correction_params: CorrectionParams,
    /// Whether to auto-apply safe recommendations.
    auto_apply_safe: bool,
    /// Whether the adapter is enabled.
    enabled: bool,
}

impl Default for AutoOptimizerAdapter {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            default_correction_params: CorrectionParams::default(),
            auto_apply_safe: false,
            enabled: false,
        }
    }
}

impl AutoOptimizerAdapter {
    /// Creates a new auto-optimizer adapter with the given endpoint.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            default_correction_params: CorrectionParams::default(),
            auto_apply_safe: false,
            enabled: true,
        }
    }

    /// Creates an adapter with auto-apply enabled for safe recommendations.
    pub fn with_auto_apply(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            default_correction_params: CorrectionParams::default(),
            auto_apply_safe: true,
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

    /// Gets optimization recommendations for a workflow.
    ///
    /// Consumes recommendations from the auto-optimizer based on historical data.
    pub async fn get_recommendations(
        &self,
        workflow_id: Uuid,
    ) -> Vec<OptimizationRecommendation> {
        if !self.enabled {
            return Vec::new();
        }

        // Placeholder: In production, this would call llm-optimizer client
        Vec::new()
    }

    /// Gets recommendations for a specific step.
    pub async fn get_step_recommendations(
        &self,
        workflow_id: Uuid,
        step_id: &str,
    ) -> Vec<OptimizationRecommendation> {
        if !self.enabled {
            return Vec::new();
        }

        // Placeholder: Would get step-specific recommendations
        Vec::new()
    }

    /// Gets self-correction parameters for the workflow.
    pub async fn get_correction_params(
        &self,
        workflow_id: Uuid,
    ) -> CorrectionParams {
        if !self.enabled {
            return self.default_correction_params.clone();
        }

        // Placeholder: Would get dynamic correction params from optimizer
        self.default_correction_params.clone()
    }

    /// Gets correction parameters for a specific step.
    pub async fn get_step_correction_params(
        &self,
        workflow_id: Uuid,
        step_id: &str,
    ) -> CorrectionParams {
        if !self.enabled {
            return self.default_correction_params.clone();
        }

        // Placeholder: Would get step-specific correction params
        self.default_correction_params.clone()
    }

    /// Reports execution metrics for optimization learning.
    pub async fn report_execution_metrics(
        &self,
        workflow_id: Uuid,
        step_id: &str,
        metrics: &HashMap<String, f64>,
    ) {
        if !self.enabled {
            return;
        }

        // Placeholder: Would report metrics to optimizer for learning
        tracing::debug!(
            workflow_id = %workflow_id,
            step_id = step_id,
            metrics = ?metrics,
            "Reported execution metrics to optimizer"
        );
    }

    /// Applies an optimization recommendation.
    pub async fn apply_recommendation(
        &self,
        recommendation_id: Uuid,
    ) -> Result<(), String> {
        if !self.enabled {
            return Err("Auto-optimizer adapter is disabled".to_string());
        }

        // Placeholder: Would apply the recommendation
        tracing::info!(
            recommendation_id = %recommendation_id,
            "Applied optimization recommendation"
        );
        Ok(())
    }

    /// Rejects an optimization recommendation with feedback.
    pub async fn reject_recommendation(
        &self,
        recommendation_id: Uuid,
        reason: &str,
    ) {
        if !self.enabled {
            return;
        }

        // Placeholder: Would reject with feedback for learning
        tracing::info!(
            recommendation_id = %recommendation_id,
            reason = reason,
            "Rejected optimization recommendation"
        );
    }

    /// Triggers self-correction for a failed step.
    pub async fn trigger_correction(
        &self,
        workflow_id: Uuid,
        step_id: &str,
        error: &str,
    ) -> Option<CorrectionParams> {
        if !self.enabled {
            return None;
        }

        // Placeholder: Would get correction strategy from optimizer
        Some(self.default_correction_params.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_disabled_by_default() {
        let adapter = AutoOptimizerAdapter::default();
        assert!(!adapter.is_enabled());
    }

    #[test]
    fn test_adapter_enabled_with_endpoint() {
        let adapter = AutoOptimizerAdapter::new("http://localhost:8085");
        assert!(adapter.is_enabled());
    }

    #[test]
    fn test_default_correction_params() {
        let params = CorrectionParams::default();
        assert!(params.enabled);
        assert_eq!(params.max_attempts, 3);
        assert_eq!(params.strategy, "retry");
    }

    #[tokio::test]
    async fn test_get_recommendations_when_disabled() {
        let adapter = AutoOptimizerAdapter::disabled();
        let recs = adapter.get_recommendations(Uuid::new_v4()).await;
        assert!(recs.is_empty());
    }

    #[tokio::test]
    async fn test_get_correction_params_when_disabled() {
        let adapter = AutoOptimizerAdapter::disabled();
        let params = adapter.get_correction_params(Uuid::new_v4()).await;
        assert!(params.enabled); // Returns defaults
    }

    #[tokio::test]
    async fn test_apply_recommendation_when_disabled() {
        let adapter = AutoOptimizerAdapter::disabled();
        let result = adapter.apply_recommendation(Uuid::new_v4()).await;
        assert!(result.is_err());
    }
}
