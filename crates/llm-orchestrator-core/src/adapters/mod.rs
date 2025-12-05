// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Thin adapter modules for upstream dependency integration.
//!
//! This module provides runtime consumption layers for external services
//! without modifying the core workflow engine logic or public APIs.
//!
//! # Phase 2B Integration
//!
//! These adapters enable the orchestrator to consume data and services from:
//! - **Memory Graph**: Lineage tracking and context history ingestion
//! - **Connector Hub**: Multi-provider routing for workflow steps
//! - **Data Vault**: Secure artifact and intermediate result persistence
//! - **Simulator**: Offline workflow step simulation
//! - **Router L2**: Graph-based routing decisions and navigation
//! - **Auto-Optimizer**: Self-correction and optimization recommendations
//! - **Observatory**: Telemetry traces, events, and runtime metrics
//!
//! # Design Principles
//!
//! - **Additive only**: No modifications to existing workflow engine logic
//! - **Thin adapters**: Minimal wrapper code, delegate to upstream crates
//! - **No circular imports**: Adapters consume from upstream, never vice versa
//! - **Optional integration**: All adapters are feature-gated for flexibility

pub mod memory_graph;
pub mod connector_hub;
pub mod data_vault;
pub mod simulator;
pub mod router_l2;
pub mod auto_optimizer;
pub mod observatory;

// Re-export adapter traits and types for convenient access
pub use memory_graph::{MemoryGraphAdapter, LineageRecord, ContextHistoryEntry};
pub use connector_hub::{ConnectorHubAdapter, ProviderRoute, RoutingConfig};
pub use data_vault::{DataVaultAdapter, ArtifactMetadata, StorageResult};
pub use simulator::{SimulatorAdapter, SimulationConfig, SimulationResult};
pub use router_l2::{RouterL2Adapter, RoutingDecision, GraphNavigator};
pub use auto_optimizer::{AutoOptimizerAdapter, OptimizationRecommendation, CorrectionParams};
pub use observatory::{ObservatoryAdapter, TelemetryEvent, WorkflowMetrics};
