// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Error types for the LLM Orchestrator core.

use thiserror::Error;

/// Result type alias for orchestrator operations.
pub type Result<T> = std::result::Result<T, OrchestratorError>;

/// Main error type for the orchestrator.
#[derive(Error, Debug)]
pub enum OrchestratorError {
    /// Workflow parsing error.
    #[error("Failed to parse workflow: {0}")]
    ParseError(String),

    /// Workflow validation error.
    #[error("Workflow validation failed: {0}")]
    ValidationError(String),

    /// Cyclic dependency detected in workflow DAG.
    #[error("Cyclic dependency detected in workflow")]
    CyclicDependency,

    /// Step not found in workflow.
    #[error("Step '{0}' not found in workflow")]
    StepNotFound(String),

    /// Invalid step configuration.
    #[error("Invalid step configuration for '{step_id}': {reason}")]
    InvalidStepConfig { step_id: String, reason: String },

    /// Execution error.
    #[error("Execution failed for step '{step_id}': {source}")]
    ExecutionError {
        step_id: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Template rendering error.
    #[error("Template rendering failed: {0}")]
    TemplateError(String),

    /// Context variable not found.
    #[error("Context variable '{0}' not found")]
    ContextVariableNotFound(String),

    /// Invalid state transition.
    #[error("Invalid state transition from {from:?} to {to:?}")]
    InvalidStateTransition {
        from: String,
        to: String,
    },

    /// Timeout error.
    #[error("Operation timed out after {duration:?}")]
    Timeout { duration: std::time::Duration },

    /// Concurrency limit exceeded.
    #[error("Concurrency limit exceeded: {limit}")]
    ConcurrencyLimitExceeded { limit: usize },

    /// Provider error (LLM API errors, rate limits, etc).
    #[error("Provider '{provider}' error: {message}")]
    ProviderError { provider: String, message: String },

    /// IO error.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Generic error.
    #[error("{0}")]
    Other(String),
}

impl OrchestratorError {
    /// Create a new parse error.
    pub fn parse<S: Into<String>>(msg: S) -> Self {
        Self::ParseError(msg.into())
    }

    /// Create a new validation error.
    pub fn validation<S: Into<String>>(msg: S) -> Self {
        Self::ValidationError(msg.into())
    }

    /// Create a new execution error.
    pub fn execution<E>(step_id: impl Into<String>, error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::ExecutionError {
            step_id: step_id.into(),
            source: Box::new(error),
        }
    }

    /// Create a new template error.
    pub fn template<S: Into<String>>(msg: S) -> Self {
        Self::TemplateError(msg.into())
    }

    /// Create a new serialization error.
    pub fn serialization<S: Into<String>>(msg: S) -> Self {
        Self::SerializationError(msg.into())
    }

    /// Create a generic error.
    pub fn other<S: Into<String>>(msg: S) -> Self {
        Self::Other(msg.into())
    }

    /// Check if error is retryable.
    ///
    /// Returns true for transient errors that may succeed on retry:
    /// - Timeout errors
    /// - Concurrency limit errors
    /// - Provider errors (rate limits, temporary API failures, etc.)
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Timeout { .. }
                | Self::ConcurrencyLimitExceeded { .. }
                | Self::ProviderError { .. }
        )
    }
}

// Implement From for common error types
impl From<serde_json::Error> for OrchestratorError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerializationError(err.to_string())
    }
}

impl From<serde_yaml::Error> for OrchestratorError {
    fn from(err: serde_yaml::Error) -> Self {
        Self::SerializationError(err.to_string())
    }
}

impl From<handlebars::RenderError> for OrchestratorError {
    fn from(err: handlebars::RenderError) -> Self {
        Self::TemplateError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = OrchestratorError::parse("invalid YAML");
        assert!(matches!(err, OrchestratorError::ParseError(_)));

        let err = OrchestratorError::validation("missing field");
        assert!(matches!(err, OrchestratorError::ValidationError(_)));
    }

    #[test]
    fn test_is_retryable() {
        let timeout_err = OrchestratorError::Timeout {
            duration: std::time::Duration::from_secs(30),
        };
        assert!(timeout_err.is_retryable());

        let parse_err = OrchestratorError::parse("test");
        assert!(!parse_err.is_retryable());
    }
}
