// Copyright (c) 2025 LLM DevOps
// SPDX-License-Identifier: Apache-2.0

//! Contract error types.

use thiserror::Error;

/// Result type for contract operations.
pub type ContractResult<T> = Result<T, ContractError>;

/// Contract validation and processing errors.
#[derive(Debug, Error)]
pub enum ContractError {
    /// Validation error.
    #[error("Validation error: {0}")]
    Validation(String),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Deserialization error.
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Schema mismatch error.
    #[error("Schema mismatch: expected {expected}, got {actual}")]
    SchemaMismatch {
        expected: String,
        actual: String,
    },

    /// Missing required field.
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Invalid state transition.
    #[error("Invalid state transition: cannot transition from {from} to {to}")]
    InvalidTransition {
        from: String,
        to: String,
    },

    /// Agent not authorized.
    #[error("Agent not authorized: {0}")]
    NotAuthorized(String),

    /// Contract version mismatch.
    #[error("Contract version mismatch: agent requires {required}, got {provided}")]
    VersionMismatch {
        required: String,
        provided: String,
    },
}

impl ContractError {
    /// Creates a validation error.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    /// Creates a missing field error.
    pub fn missing_field(field: impl Into<String>) -> Self {
        Self::MissingField(field.into())
    }

    /// Creates an invalid transition error.
    pub fn invalid_transition(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self::InvalidTransition {
            from: from.into(),
            to: to.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = ContractError::validation("test error");
        assert!(error.to_string().contains("test error"));
    }

    #[test]
    fn test_invalid_transition_error() {
        let error = ContractError::invalid_transition("pending", "completed");
        assert!(error.to_string().contains("pending"));
        assert!(error.to_string().contains("completed"));
    }
}
