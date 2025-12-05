// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! LLM Orchestrator Core - Workflow orchestration engine for LLM pipelines.
//!
//! This crate provides the core functionality for defining, validating, and executing
//! multi-step LLM workflows with support for DAG-based execution, parallel processing,
//! and fault tolerance.
//!
//! # Example
//!
//! ```rust
//! use llm_orchestrator_core::{Workflow, ExecutionContext};
//! use serde_json::json;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Load workflow from YAML
//! let yaml = r#"
//! name: "simple-workflow"
//! steps:
//!   - id: "step1"
//!     type: "llm"
//!     provider: "openai"
//!     model: "gpt-4"
//!     prompt: "Say hello to {{ name }}"
//!     output: ["greeting"]
//! "#;
//!
//! let workflow = Workflow::from_yaml(yaml)?;
//!
//! // Validate workflow
//! workflow.validate()?;
//!
//! // Create execution context
//! let mut inputs = std::collections::HashMap::new();
//! inputs.insert("name".to_string(), json!("World"));
//! let ctx = ExecutionContext::new(inputs);
//!
//! // Render template with context
//! let prompt = ctx.render_template("Say hello to {{ name }}")?;
//! assert_eq!(prompt, "Say hello to World");
//! # Ok(())
//! # }
//! ```

pub mod context;
pub mod dag;
pub mod error;
pub mod executor;
pub mod executor_state;
pub mod health;
pub mod metrics;
pub mod providers;
pub mod retry;
pub mod workflow;

// Phase 2B: Thin adapter modules for upstream dependency integration
pub mod adapters;

// Re-export commonly used types
pub use context::ExecutionContext;
pub use dag::WorkflowDAG;
pub use error::{OrchestratorError, Result};
pub use executor::{StepResult, StepStatus, WorkflowExecutor};
pub use providers::{CompletionRequest, CompletionResponse, LLMProvider, ProviderError};
pub use retry::{RetryExecutor, RetryPolicy};
pub use workflow::{
    Workflow, Step, StepType, StepConfig,
    LlmStepConfig, EmbedStepConfig, VectorSearchConfig,
    TransformConfig, ActionConfig, ParallelConfig, BranchConfig,
    RetryConfig, BackoffStrategy,
};

/// Library version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name.
pub const NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        // Check that version is set (should be a semver string)
        assert!(VERSION.contains('.'));
        assert_eq!(NAME, "llm-orchestrator-core");
    }
}
