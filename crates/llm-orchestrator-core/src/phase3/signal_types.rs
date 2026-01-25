// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Signal Types for Phase 3 Layer 1 DecisionEvents.
//!
//! Phase 3 Layer 1 agents emit:
//! - `execution_strategy_signal` - Routing and coordination signals
//! - `optimization_signal` - Performance optimization signals
//! - `incident_signal` - Escalation and incident signals
//!
//! All signals include confidence and references as per constitution.

use agentics_contracts::{AgentId, AgentVersion, DecisionEvent, DecisionType, ExecutionRef};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Signal types emitted by Phase 3 Layer 1 agents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalType {
    /// Execution strategy signal - routing and coordination.
    ExecutionStrategySignal,
    /// Optimization signal - performance optimization.
    OptimizationSignal,
    /// Incident signal - escalation and incidents.
    IncidentSignal,
}

impl std::fmt::Display for SignalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExecutionStrategySignal => write!(f, "execution_strategy_signal"),
            Self::OptimizationSignal => write!(f, "optimization_signal"),
            Self::IncidentSignal => write!(f, "incident_signal"),
        }
    }
}

/// Confidence level for signals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalConfidence {
    /// Confidence score (0.0 to 1.0).
    pub score: f64,
    /// Factors contributing to confidence.
    pub factors: Vec<ConfidenceFactor>,
}

impl Default for SignalConfidence {
    fn default() -> Self {
        Self {
            score: 0.0,
            factors: Vec::new(),
        }
    }
}

impl SignalConfidence {
    /// Creates a new confidence with a score.
    pub fn new(score: f64) -> Self {
        Self {
            score: score.clamp(0.0, 1.0),
            factors: Vec::new(),
        }
    }

    /// Adds a confidence factor.
    pub fn with_factor(mut self, factor: ConfidenceFactor) -> Self {
        self.factors.push(factor);
        self
    }
}

/// Factor contributing to signal confidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceFactor {
    /// Factor name.
    pub name: String,
    /// Factor weight.
    pub weight: f64,
    /// Factor description.
    pub description: Option<String>,
}

/// References for signal traceability.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SignalReferences {
    /// Source workflow ID.
    pub workflow_id: Option<Uuid>,
    /// Source task ID.
    pub task_id: Option<Uuid>,
    /// Related incident ID.
    pub incident_id: Option<String>,
    /// Upstream agent IDs.
    pub upstream_agents: Vec<String>,
    /// Downstream target IDs.
    pub downstream_targets: Vec<String>,
    /// Additional reference data.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl SignalReferences {
    /// Creates new references with a workflow ID.
    pub fn for_workflow(workflow_id: Uuid) -> Self {
        Self {
            workflow_id: Some(workflow_id),
            ..Default::default()
        }
    }

    /// Adds an upstream agent reference.
    pub fn with_upstream(mut self, agent_id: impl Into<String>) -> Self {
        self.upstream_agents.push(agent_id.into());
        self
    }

    /// Adds a downstream target reference.
    pub fn with_downstream(mut self, target_id: impl Into<String>) -> Self {
        self.downstream_targets.push(target_id.into());
        self
    }
}

/// Execution strategy signal payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStrategySignal {
    /// Recommended execution strategy.
    pub strategy: ExecutionStrategy,
    /// Target agents for routing.
    pub routing_targets: Vec<RoutingTarget>,
    /// Coordination requirements.
    pub coordination: CoordinationRequirements,
    /// Signal confidence.
    pub confidence: SignalConfidence,
    /// Signal references.
    pub references: SignalReferences,
}

/// Execution strategy types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStrategy {
    /// Sequential execution.
    Sequential,
    /// Parallel execution.
    Parallel,
    /// Fan-out execution.
    FanOut,
    /// Fan-in aggregation.
    FanIn,
    /// Priority-based execution.
    Priority { level: u32 },
    /// Deferred execution.
    Deferred { delay_ms: u64 },
}

/// Routing target for execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingTarget {
    /// Target agent ID.
    pub agent_id: String,
    /// Target endpoint.
    pub endpoint: Option<String>,
    /// Routing priority.
    pub priority: u32,
    /// Routing metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Coordination requirements.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoordinationRequirements {
    /// Required synchronization points.
    pub sync_points: Vec<String>,
    /// Required dependencies.
    pub dependencies: Vec<String>,
    /// Timeout for coordination.
    pub timeout_ms: Option<u64>,
}

/// Optimization signal payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSignal {
    /// Optimization type.
    pub optimization_type: OptimizationType,
    /// Optimization recommendations.
    pub recommendations: Vec<OptimizationRecommendation>,
    /// Expected impact.
    pub expected_impact: OptimizationImpact,
    /// Signal confidence.
    pub confidence: SignalConfidence,
    /// Signal references.
    pub references: SignalReferences,
}

/// Optimization types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptimizationType {
    /// Latency optimization.
    Latency,
    /// Throughput optimization.
    Throughput,
    /// Cost optimization.
    Cost,
    /// Resource optimization.
    Resource,
    /// Quality optimization.
    Quality,
}

/// Optimization recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    /// Recommendation ID.
    pub id: String,
    /// Recommendation description.
    pub description: String,
    /// Recommendation priority.
    pub priority: u32,
    /// Estimated improvement percentage.
    pub estimated_improvement_pct: f64,
}

/// Expected optimization impact.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OptimizationImpact {
    /// Latency reduction percentage.
    pub latency_reduction_pct: Option<f64>,
    /// Throughput increase percentage.
    pub throughput_increase_pct: Option<f64>,
    /// Cost reduction percentage.
    pub cost_reduction_pct: Option<f64>,
}

/// Incident signal payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentSignal {
    /// Incident severity.
    pub severity: IncidentSeverity,
    /// Incident category.
    pub category: IncidentCategory,
    /// Incident description.
    pub description: String,
    /// Escalation targets.
    pub escalation_targets: Vec<EscalationTarget>,
    /// Recommended actions.
    pub recommended_actions: Vec<String>,
    /// Signal confidence.
    pub confidence: SignalConfidence,
    /// Signal references.
    pub references: SignalReferences,
}

/// Incident severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentSeverity {
    /// Low severity - informational.
    Low,
    /// Medium severity - requires attention.
    Medium,
    /// High severity - requires immediate attention.
    High,
    /// Critical severity - requires immediate action.
    Critical,
}

/// Incident categories.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentCategory {
    /// Performance degradation.
    Performance,
    /// Error or failure.
    Error,
    /// Security concern.
    Security,
    /// Resource constraint.
    Resource,
    /// Configuration issue.
    Configuration,
    /// Dependency failure.
    Dependency,
}

/// Escalation target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationTarget {
    /// Target system or team.
    pub target: String,
    /// Escalation level.
    pub level: u32,
    /// Contact method.
    pub contact_method: Option<String>,
}

/// Signal-based DecisionEvent for Phase 3 Layer 1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalDecisionEvent {
    /// Base decision event.
    #[serde(flatten)]
    pub base: DecisionEvent,
    /// Signal type.
    pub signal_type: SignalType,
    /// Signal payload (JSON).
    pub signal_payload: serde_json::Value,
}

impl SignalDecisionEvent {
    /// Creates a new execution strategy signal event.
    pub fn execution_strategy(
        agent_id: AgentId,
        agent_version: AgentVersion,
        execution_ref: ExecutionRef,
        signal: ExecutionStrategySignal,
        inputs_hash: String,
    ) -> Self {
        let base = DecisionEvent::new(
            agent_id,
            agent_version,
            DecisionType::Execute,
            inputs_hash,
            execution_ref,
        )
        .with_confidence(signal.confidence.score)
        .with_metadata("signal_type", serde_json::json!("execution_strategy_signal"));

        Self {
            base,
            signal_type: SignalType::ExecutionStrategySignal,
            signal_payload: serde_json::to_value(&signal).unwrap_or_default(),
        }
    }

    /// Creates a new optimization signal event.
    pub fn optimization(
        agent_id: AgentId,
        agent_version: AgentVersion,
        execution_ref: ExecutionRef,
        signal: OptimizationSignal,
        inputs_hash: String,
    ) -> Self {
        let base = DecisionEvent::new(
            agent_id,
            agent_version,
            DecisionType::Execute,
            inputs_hash,
            execution_ref,
        )
        .with_confidence(signal.confidence.score)
        .with_metadata("signal_type", serde_json::json!("optimization_signal"));

        Self {
            base,
            signal_type: SignalType::OptimizationSignal,
            signal_payload: serde_json::to_value(&signal).unwrap_or_default(),
        }
    }

    /// Creates a new incident signal event.
    pub fn incident(
        agent_id: AgentId,
        agent_version: AgentVersion,
        execution_ref: ExecutionRef,
        signal: IncidentSignal,
        inputs_hash: String,
    ) -> Self {
        let base = DecisionEvent::new(
            agent_id,
            agent_version,
            DecisionType::Execute,
            inputs_hash,
            execution_ref,
        )
        .with_confidence(signal.confidence.score)
        .with_metadata("signal_type", serde_json::json!("incident_signal"))
        .with_metadata("severity", serde_json::json!(signal.severity));

        Self {
            base,
            signal_type: SignalType::IncidentSignal,
            signal_payload: serde_json::to_value(&signal).unwrap_or_default(),
        }
    }

    /// Returns the signal confidence score.
    pub fn confidence(&self) -> f64 {
        self.base.confidence
    }

    /// Returns the signal type as a string.
    pub fn signal_type_str(&self) -> &'static str {
        match self.signal_type {
            SignalType::ExecutionStrategySignal => "execution_strategy_signal",
            SignalType::OptimizationSignal => "optimization_signal",
            SignalType::IncidentSignal => "incident_signal",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_type_display() {
        assert_eq!(
            SignalType::ExecutionStrategySignal.to_string(),
            "execution_strategy_signal"
        );
        assert_eq!(
            SignalType::OptimizationSignal.to_string(),
            "optimization_signal"
        );
        assert_eq!(SignalType::IncidentSignal.to_string(), "incident_signal");
    }

    #[test]
    fn test_signal_confidence() {
        let confidence = SignalConfidence::new(0.85)
            .with_factor(ConfidenceFactor {
                name: "historical_accuracy".to_string(),
                weight: 0.7,
                description: Some("Based on historical prediction accuracy".to_string()),
            });

        assert_eq!(confidence.score, 0.85);
        assert_eq!(confidence.factors.len(), 1);
    }

    #[test]
    fn test_confidence_clamping() {
        let high = SignalConfidence::new(1.5);
        assert_eq!(high.score, 1.0);

        let low = SignalConfidence::new(-0.5);
        assert_eq!(low.score, 0.0);
    }

    #[test]
    fn test_execution_strategy_signal() {
        let agent_id = AgentId::from_string("phase3-coordinator");
        let agent_version = AgentVersion::new(0, 1, 0);
        let execution_ref = ExecutionRef::new(Uuid::new_v4());

        let signal = ExecutionStrategySignal {
            strategy: ExecutionStrategy::Parallel,
            routing_targets: vec![RoutingTarget {
                agent_id: "worker-1".to_string(),
                endpoint: Some("/execute".to_string()),
                priority: 1,
                metadata: HashMap::new(),
            }],
            coordination: CoordinationRequirements::default(),
            confidence: SignalConfidence::new(0.9),
            references: SignalReferences::for_workflow(Uuid::new_v4()),
        };

        let event = SignalDecisionEvent::execution_strategy(
            agent_id,
            agent_version,
            execution_ref,
            signal,
            "0".repeat(64),
        );

        assert_eq!(event.signal_type, SignalType::ExecutionStrategySignal);
        assert_eq!(event.confidence(), 0.9);
    }
}
