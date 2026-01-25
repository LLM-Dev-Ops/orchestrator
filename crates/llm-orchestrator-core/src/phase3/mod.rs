// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Phase 3 - Automation & Resilience (Layer 1)
//!
//! This module implements the startup hardening, execution role guards,
//! performance budgets, and signal emission for Phase 3 Layer 1 agents.
//!
//! # Constitution Compliance
//!
//! Agents in this phase MUST:
//! - Coordinate
//! - Route
//! - Optimize
//! - Escalate
//!
//! Agents MUST NOT:
//! - Make final decisions
//! - Emit executive conclusions
//!
//! # DecisionEvent Signal Types
//!
//! Phase 3 Layer 1 agents emit:
//! - `execution_strategy_signal` - Routing and coordination signals
//! - `optimization_signal` - Performance optimization signals
//! - `incident_signal` - Escalation and incident signals

pub mod config;
pub mod execution_guard;
pub mod performance_budget;
pub mod signal_types;

pub use config::{Phase3Config, Phase3Layer, StartupValidator};
pub use execution_guard::{ExecutionGuard, ExecutionRole, GuardViolation};
pub use performance_budget::{PerformanceBudget, BudgetEnforcer, BudgetViolation};
pub use signal_types::{
    SignalType, ExecutionStrategySignal, OptimizationSignal, IncidentSignal,
    SignalConfidence, SignalReferences, SignalDecisionEvent,
};
