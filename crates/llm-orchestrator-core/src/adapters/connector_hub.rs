// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Connector Hub adapter for routing workflow steps to different model providers.
//!
//! This adapter provides a thin integration layer to consume provider routing
//! decisions from LLM-Connector-Hub without modifying core workflow logic.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for provider routing decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// Default provider to use when no specific route matches.
    pub default_provider: String,
    /// Whether to enable load balancing across providers.
    pub load_balancing: bool,
    /// Failover configuration.
    pub failover_enabled: bool,
    /// Maximum retry attempts for failed providers.
    pub max_retries: u32,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            default_provider: "openai".to_string(),
            load_balancing: false,
            failover_enabled: true,
            max_retries: 3,
        }
    }
}

/// A route decision for a workflow step to a specific provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRoute {
    /// The selected provider name.
    pub provider: String,
    /// The endpoint URL for the provider.
    pub endpoint: String,
    /// Provider-specific configuration.
    pub config: HashMap<String, serde_json::Value>,
    /// Priority for this route (lower = higher priority).
    pub priority: u32,
    /// Whether this is a fallback route.
    pub is_fallback: bool,
    /// Health status of the provider.
    pub healthy: bool,
}

/// Adapter for consuming provider routing from LLM-Connector-Hub.
///
/// This adapter enables the orchestrator to route workflow steps to
/// different model providers based on availability, load, and policies.
#[derive(Debug, Clone)]
pub struct ConnectorHubAdapter {
    /// Base URL for the connector hub service.
    endpoint: String,
    /// Routing configuration.
    config: RoutingConfig,
    /// Whether the adapter is enabled.
    enabled: bool,
}

impl Default for ConnectorHubAdapter {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            config: RoutingConfig::default(),
            enabled: false,
        }
    }
}

impl ConnectorHubAdapter {
    /// Creates a new connector hub adapter with the given endpoint.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            config: RoutingConfig::default(),
            enabled: true,
        }
    }

    /// Creates a new adapter with custom routing configuration.
    pub fn with_config(endpoint: impl Into<String>, config: RoutingConfig) -> Self {
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

    /// Returns the routing configuration.
    pub fn config(&self) -> &RoutingConfig {
        &self.config
    }

    /// Gets the optimal route for a workflow step.
    ///
    /// Consumes routing decision from connector hub based on step requirements.
    pub async fn get_route(
        &self,
        step_id: &str,
        provider_hint: Option<&str>,
        model: &str,
    ) -> Option<ProviderRoute> {
        if !self.enabled {
            return None;
        }

        // Placeholder: In production, this would call llm-connector-hub client
        // to get the optimal route based on load, availability, and policies.
        let provider = provider_hint.unwrap_or(&self.config.default_provider);

        Some(ProviderRoute {
            provider: provider.to_string(),
            endpoint: format!("https://api.{}.com/v1", provider),
            config: HashMap::new(),
            priority: 1,
            is_fallback: false,
            healthy: true,
        })
    }

    /// Gets all available routes for a provider type.
    ///
    /// Returns a list of routes ordered by priority.
    pub async fn get_available_routes(&self, provider_type: &str) -> Vec<ProviderRoute> {
        if !self.enabled {
            return Vec::new();
        }

        // Placeholder: Would return all registered routes for the provider type
        vec![ProviderRoute {
            provider: provider_type.to_string(),
            endpoint: format!("https://api.{}.com/v1", provider_type),
            config: HashMap::new(),
            priority: 1,
            is_fallback: false,
            healthy: true,
        }]
    }

    /// Reports a provider failure for routing decisions.
    ///
    /// This allows the connector hub to update its routing decisions.
    pub async fn report_failure(&self, provider: &str, error: &str) {
        if !self.enabled {
            return;
        }

        // Placeholder: Would report the failure to connector hub
        tracing::warn!(
            provider = provider,
            error = error,
            "Provider failure reported to connector hub"
        );
    }

    /// Reports a successful provider call for routing metrics.
    pub async fn report_success(&self, provider: &str, latency_ms: u64) {
        if !self.enabled {
            return;
        }

        // Placeholder: Would report success metrics to connector hub
        tracing::debug!(
            provider = provider,
            latency_ms = latency_ms,
            "Provider success reported to connector hub"
        );
    }

    /// Checks the health of a specific provider.
    pub async fn check_provider_health(&self, provider: &str) -> bool {
        if !self.enabled {
            return true; // Assume healthy when disabled
        }

        // Placeholder: Would query connector hub for provider health
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_disabled_by_default() {
        let adapter = ConnectorHubAdapter::default();
        assert!(!adapter.is_enabled());
    }

    #[test]
    fn test_adapter_enabled_with_endpoint() {
        let adapter = ConnectorHubAdapter::new("http://localhost:8081");
        assert!(adapter.is_enabled());
        assert_eq!(adapter.endpoint(), "http://localhost:8081");
    }

    #[test]
    fn test_default_routing_config() {
        let config = RoutingConfig::default();
        assert_eq!(config.default_provider, "openai");
        assert!(config.failover_enabled);
    }

    #[tokio::test]
    async fn test_get_route_when_disabled() {
        let adapter = ConnectorHubAdapter::disabled();
        let route = adapter.get_route("step1", None, "gpt-4").await;
        assert!(route.is_none());
    }

    #[tokio::test]
    async fn test_get_route_with_hint() {
        let adapter = ConnectorHubAdapter::new("http://localhost:8081");
        let route = adapter.get_route("step1", Some("anthropic"), "claude-3").await;
        assert!(route.is_some());
        assert_eq!(route.unwrap().provider, "anthropic");
    }
}
