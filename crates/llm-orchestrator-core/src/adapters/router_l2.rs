// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Router L2 adapter for routing decisions and graph-based navigation.
//!
//! This adapter provides a thin integration layer to consume routing
//! decisions from the Router L2 module without modifying core workflow logic.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A routing decision from the L2 router.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// Unique identifier for this decision.
    pub id: Uuid,
    /// Source node in the routing graph.
    pub source: String,
    /// Target node in the routing graph.
    pub target: String,
    /// Decision weight/confidence (0.0 to 1.0).
    pub weight: f64,
    /// Reason for this routing decision.
    pub reason: String,
    /// Alternative routes considered.
    pub alternatives: Vec<String>,
    /// Decision metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Navigator for graph-based workflow routing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNavigator {
    /// Current position in the graph.
    pub current_node: String,
    /// Path taken so far.
    pub path: Vec<String>,
    /// Nodes visited.
    pub visited: Vec<String>,
    /// Remaining nodes to visit.
    pub remaining: Vec<String>,
}

impl GraphNavigator {
    /// Creates a new navigator starting at the given node.
    pub fn new(start_node: impl Into<String>) -> Self {
        let start = start_node.into();
        Self {
            current_node: start.clone(),
            path: vec![start.clone()],
            visited: vec![start],
            remaining: Vec::new(),
        }
    }

    /// Moves to the next node in the path.
    pub fn advance(&mut self, next_node: impl Into<String>) {
        let next = next_node.into();
        self.visited.push(next.clone());
        self.path.push(next.clone());
        self.current_node = next;
    }

    /// Checks if a node has been visited.
    pub fn has_visited(&self, node: &str) -> bool {
        self.visited.contains(&node.to_string())
    }
}

/// Adapter for consuming routing decisions from the Router L2 module.
///
/// This adapter enables intelligent routing within workflows based on
/// graph analysis and decision heuristics.
#[derive(Debug, Clone)]
pub struct RouterL2Adapter {
    /// Base URL for the router L2 service.
    endpoint: String,
    /// Whether to cache routing decisions.
    cache_enabled: bool,
    /// Whether the adapter is enabled.
    enabled: bool,
}

impl Default for RouterL2Adapter {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            cache_enabled: true,
            enabled: false,
        }
    }
}

impl RouterL2Adapter {
    /// Creates a new router L2 adapter with the given endpoint.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            cache_enabled: true,
            enabled: true,
        }
    }

    /// Creates an adapter with caching disabled.
    pub fn without_cache(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            cache_enabled: false,
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

    /// Gets a routing decision for the given workflow context.
    ///
    /// Consumes routing decision from the L2 router based on current state.
    pub async fn get_routing_decision(
        &self,
        workflow_id: Uuid,
        current_step: &str,
        context: &HashMap<String, serde_json::Value>,
    ) -> Option<RoutingDecision> {
        if !self.enabled {
            return None;
        }

        // Placeholder: In production, this would call the router-l2 client
        Some(RoutingDecision {
            id: Uuid::new_v4(),
            source: current_step.to_string(),
            target: "next_step".to_string(),
            weight: 1.0,
            reason: "Default sequential routing".to_string(),
            alternatives: Vec::new(),
            metadata: HashMap::new(),
        })
    }

    /// Gets optimal path through the workflow graph.
    pub async fn get_optimal_path(
        &self,
        workflow_id: Uuid,
        start: &str,
        end: &str,
    ) -> Vec<String> {
        if !self.enabled {
            return vec![start.to_string(), end.to_string()];
        }

        // Placeholder: Would compute optimal path via L2 router
        vec![start.to_string(), end.to_string()]
    }

    /// Creates a graph navigator for the workflow.
    pub async fn create_navigator(
        &self,
        workflow_id: Uuid,
        start_step: &str,
    ) -> GraphNavigator {
        GraphNavigator::new(start_step)
    }

    /// Updates the routing graph with execution results.
    pub async fn update_routing_graph(
        &self,
        workflow_id: Uuid,
        step_id: &str,
        success: bool,
        duration_ms: u64,
    ) {
        if !self.enabled {
            return;
        }

        // Placeholder: Would update routing weights in L2 router
        tracing::debug!(
            workflow_id = %workflow_id,
            step_id = step_id,
            success = success,
            duration_ms = duration_ms,
            "Updated routing graph"
        );
    }

    /// Queries possible routes from the current position.
    pub async fn get_possible_routes(
        &self,
        workflow_id: Uuid,
        current_step: &str,
    ) -> Vec<RoutingDecision> {
        if !self.enabled {
            return Vec::new();
        }

        // Placeholder: Would query L2 router for possible routes
        Vec::new()
    }

    /// Reports a routing failure for learning.
    pub async fn report_routing_failure(
        &self,
        workflow_id: Uuid,
        decision_id: Uuid,
        error: &str,
    ) {
        if !self.enabled {
            return;
        }

        // Placeholder: Would report failure to L2 router for learning
        tracing::warn!(
            workflow_id = %workflow_id,
            decision_id = %decision_id,
            error = error,
            "Routing failure reported"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_disabled_by_default() {
        let adapter = RouterL2Adapter::default();
        assert!(!adapter.is_enabled());
    }

    #[test]
    fn test_adapter_enabled_with_endpoint() {
        let adapter = RouterL2Adapter::new("http://localhost:8084");
        assert!(adapter.is_enabled());
    }

    #[test]
    fn test_graph_navigator() {
        let mut nav = GraphNavigator::new("step1");
        assert_eq!(nav.current_node, "step1");
        assert!(nav.has_visited("step1"));
        assert!(!nav.has_visited("step2"));

        nav.advance("step2");
        assert_eq!(nav.current_node, "step2");
        assert!(nav.has_visited("step2"));
        assert_eq!(nav.path, vec!["step1", "step2"]);
    }

    #[tokio::test]
    async fn test_get_routing_decision_when_disabled() {
        let adapter = RouterL2Adapter::disabled();
        let result = adapter
            .get_routing_decision(Uuid::new_v4(), "step1", &HashMap::new())
            .await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_routing_decision_when_enabled() {
        let adapter = RouterL2Adapter::new("http://localhost:8084");
        let result = adapter
            .get_routing_decision(Uuid::new_v4(), "step1", &HashMap::new())
            .await;
        assert!(result.is_some());
        let decision = result.unwrap();
        assert_eq!(decision.source, "step1");
    }
}
